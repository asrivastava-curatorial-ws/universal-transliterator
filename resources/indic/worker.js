#!/usr/bin/env node
/*
 * Offline AI4Bharat IndicXlit worker for Universal Transliterator.
 *
 * The beam-search procedure is adapted from @cloudrumbles/indic-transliterate
 * 1.2.2 (MIT). It talks directly to the bundled onnxruntime-node N-API binding,
 * so no npm install, network access, or localhost service is required.
 *
 * Protocol: one JSON object per line on stdin, one JSON object per line on
 * stdout, answered strictly in order.
 *
 *   {"id":7,"lang":"hi","words":["namaste","duniya"],"count":6}
 *   -> {"id":7,"ok":true,"lang":"hi","words":{"namaste":["नमस्ते",...],...}}
 *
 *   {"id":8,"lang":"hi","text":"namaste duniya"}            (whole-text mode)
 *   -> {"id":8,"ok":true,"result":"नमस्ते दुनिया","candidates":[...],"primary":"..."}
 *
 * Beams of one word are decoded together as a single batched ONNX run per
 * step (the decoder takes encoder_out as [time, batch, dim]), and decoding
 * stops once enough hypotheses have finished. This is ~8x faster than
 * running every beam separately for a fixed 20 steps.
 */
'use strict';

const fs = require('node:fs');
const path = require('node:path');
const readline = require('node:readline');

const root = __dirname;
const modelDir = process.env.UT_INDIC_MODEL_DIR || path.join(root, 'models');
const bindingPath = process.env.UT_ONNX_BINDING ||
  path.join(root, 'onnxruntime', 'bin', 'napi-v3', 'linux', 'x64', 'onnxruntime_binding.node');

const binding = require(bindingPath);
const vocab = JSON.parse(fs.readFileSync(path.join(modelDir, 'vocab.json'), 'utf8'));
const sourceIndex = new Map(vocab.src.map((token, index) => [token, index]));
const sessionOptions = {
  executionProviders: ['cpu'],
  graphOptimizationLevel: 'all',
  intraOpNumThreads: 1,
  interOpNumThreads: 1,
};
const encoder = new binding.InferenceSession();
const decoder = new binding.InferenceSession();
encoder.loadModel(path.join(modelDir, 'indicxlit_encoder.onnx'), sessionOptions);
decoder.loadModel(path.join(modelDir, 'indicxlit_decoder_v2.onnx'), sessionOptions);

const SUPPORTED = new Set([
  'as', 'bn', 'brx', 'gom', 'gu', 'hi', 'kn', 'ks', 'mai', 'ml', 'mni',
  'mr', 'ne', 'or', 'pa', 'sa', 'sd', 'si', 'ta', 'te', 'ur'
]);
const MAX_WORDS_PER_REQUEST = 4000;
const MAX_WORD_LENGTH = 48;
const MAX_TEXT_BYTES = 100000;
const CACHE_LIMIT = 20000;
const cache = new Map();
const special = vocab.special_tokens;
const vocabSize = vocab.tgt.length;

function tensor(type, data, dims) {
  return {type, data, dims};
}

function tokenize(word, lang) {
  const indices = [sourceIndex.get(`__${lang}__`)];
  for (const letter of word) {
    const index = sourceIndex.get(letter);
    indices.push(index === undefined ? special.unk : index);
  }
  indices.push(special.eos);
  return indices;
}

function detokenize(tokens) {
  const result = [];
  for (let index = 1; index < tokens.length; index += 1) {
    const value = tokens[index];
    if (value === special.eos) break;
    if (value === special.bos || value === special.pad || value === special.unk) continue;
    const piece = vocab.tgt[value];
    if (piece) result.push(piece);
  }
  return result.join('').replaceAll(' ', '');
}

/** Repeats a [time, 1, dim] encoder output into [time, batch, dim]. */
function tileEncoder(encoded, batch) {
  if (batch === 1) return encoded;
  const [time, , dim] = encoded.dims;
  const data = new Float32Array(time * batch * dim);
  for (let step = 0; step < time; step += 1) {
    const row = encoded.data.subarray(step * dim, (step + 1) * dim);
    for (let beam = 0; beam < batch; beam += 1) {
      data.set(row, (step * batch + beam) * dim);
    }
  }
  return tensor('float32', data, [time, batch, dim]);
}

function logSoftmaxTop(logits, offset, allowEos, limit) {
  let maxLogit = -Infinity;
  for (let index = 0; index < vocabSize; index += 1) {
    if (index === special.eos && !allowEos) continue;
    const value = logits[offset + index];
    if (value > maxLogit) maxLogit = value;
  }
  let sum = 0;
  for (let index = 0; index < vocabSize; index += 1) {
    if (index === special.eos && !allowEos) continue;
    sum += Math.exp(logits[offset + index] - maxLogit);
  }
  const normalizer = maxLogit + Math.log(sum);
  const top = [];
  for (let index = 0; index < vocabSize; index += 1) {
    if (index === special.eos && !allowEos) continue;
    if (index === special.pad || index === special.bos) continue;
    const score = logits[offset + index] - normalizer;
    if (top.length < limit) {
      top.push({index, score});
      if (top.length === limit) top.sort((a, b) => b.score - a.score);
    } else if (score > top[limit - 1].score) {
      top[limit - 1] = {index, score};
      top.sort((a, b) => b.score - a.score);
    }
  }
  if (top.length < limit) top.sort((a, b) => b.score - a.score);
  return top;
}

function transliterateWord(word, lang, count) {
  const key = `${lang}|${count}|${word}`;
  const cached = cache.get(key);
  if (cached) return cached;

  const source = tokenize(word, lang);
  const encoded = encoder.run({
    src_tokens: tensor('int64', new BigInt64Array(source.map(BigInt)), [1, source.length])
  }, {encoder_out: null}, {}).encoder_out;

  const beamWidth = Math.max(5, Math.min(9, count));
  const maxSteps = Math.min(40, source.length * 2 + 6);
  const penalty = (length) => (5 + length) / 6;
  const needed = count + 2;
  const tiled = new Map();
  let beams = [{score: 0, tokens: [special.eos]}];
  const finished = [];

  for (let step = 0; step < maxSteps && beams.length > 0; step += 1) {
    const batch = beams.length;
    const length = beams[0].tokens.length;
    const previous = new BigInt64Array(batch * length);
    beams.forEach((beam, row) => {
      beam.tokens.forEach((token, column) => {
        previous[row * length + column] = BigInt(token);
      });
    });
    if (!tiled.has(batch)) tiled.set(batch, tileEncoder(encoded, batch));
    const logits = decoder.run({
      prev_tokens: tensor('int64', previous, [batch, length]),
      encoder_out: tiled.get(batch)
    }, {decoder_out: null}, {}).decoder_out.data;

    const expanded = [];
    beams.forEach((beam, row) => {
      const offset = (row * length + (length - 1)) * vocabSize;
      for (const choice of logSoftmaxTop(logits, offset, length > 1, beamWidth * 2)) {
        const candidate = {score: beam.score + choice.score, tokens: [...beam.tokens, choice.index]};
        (choice.index === special.eos ? finished : expanded).push(candidate);
      }
    });
    expanded.sort((a, b) => b.score - a.score);
    beams = expanded.slice(0, beamWidth);

    // Log-probabilities only fall as a hypothesis grows, so an active beam can
    // at best reach score / penalty(maxSteps). Once no beam can beat the
    // needed-th best finished hypothesis, further decoding cannot change the
    // returned list.
    if (finished.length >= needed && beams.length > 0) {
      const normalized = finished
        .map((candidate) => candidate.score / penalty(candidate.tokens.length - 1))
        .sort((a, b) => b - a);
      const threshold = normalized[needed - 1];
      const bestPossible = beams[0].score / penalty(maxSteps);
      if (bestPossible < threshold) break;
    }
  }

  for (const beam of beams) finished.push(beam);
  const ranked = finished.map((candidate) => {
    const length = Math.max(1, candidate.tokens.length - 1);
    return {tokens: candidate.tokens, normalized: candidate.score / penalty(length)};
  }).sort((a, b) => b.normalized - a.normalized);

  const seen = new Set();
  const words = [];
  for (const candidate of ranked) {
    const value = detokenize(candidate.tokens);
    if (value && !seen.has(value)) {
      seen.add(value);
      words.push(value);
      if (words.length >= count) break;
    }
  }
  if (cache.size >= CACHE_LIMIT) cache.delete(cache.keys().next().value);
  cache.set(key, words);
  return words;
}

function checkLanguage(lang) {
  if (!SUPPORTED.has(lang)) throw new Error('unsupported Indic language');
}

function convertWords(words, lang, count) {
  checkLanguage(lang);
  if (!Array.isArray(words) || words.length > MAX_WORDS_PER_REQUEST) {
    throw new Error(`words must be an array of at most ${MAX_WORDS_PER_REQUEST} items`);
  }
  const result = {};
  for (const raw of words) {
    if (typeof raw !== 'string') throw new Error('words must be strings');
    const word = raw.toLowerCase();
    if (!/^[a-z]+$/.test(word) || word.length > MAX_WORD_LENGTH) {
      result[raw] = [];
      continue;
    }
    result[raw] = transliterateWord(word, lang, count);
  }
  return result;
}

function convertText(text, lang) {
  checkLanguage(lang);
  if (typeof text !== 'string' || Buffer.byteLength(text, 'utf8') > MAX_TEXT_BYTES) {
    throw new Error('input must be UTF-8 text no larger than 100,000 bytes');
  }
  const matches = [...text.matchAll(/[A-Za-z]+/g)];
  const byWord = new Map();
  for (const match of matches) {
    const key = match[0].toLowerCase();
    if (!byWord.has(key)) {
      byWord.set(key, key.length <= MAX_WORD_LENGTH ? transliterateWord(key, lang, 5) : []);
    }
  }
  let result = '';
  let cursor = 0;
  for (const match of matches) {
    result += text.slice(cursor, match.index);
    const candidates = byWord.get(match[0].toLowerCase());
    result += candidates && candidates.length ? candidates[0] : match[0];
    cursor = match.index + match[0].length;
  }
  result += text.slice(cursor);
  const trailing = text.match(/[A-Za-z]+$/);
  const candidates = trailing ? (byWord.get(trailing[0].toLowerCase()) || []) : [];
  return {result, candidates, primary: candidates[0] || '', lastWord: trailing ? trailing[0] : ''};
}

function respond(request) {
  const id = Number.isSafeInteger(request && request.id) ? request.id : 0;
  try {
    const lang = typeof request.lang === 'string' ? request.lang : 'hi';
    if (Array.isArray(request.words)) {
      const count = Number.isSafeInteger(request.count) ? Math.max(1, Math.min(9, request.count)) : 6;
      return {id, ok: true, lang, words: convertWords(request.words, lang, count)};
    }
    return {id, ok: true, lang, ...convertText(request.text || '', lang)};
  } catch (error) {
    return {id, ok: false, error: error instanceof Error ? error.message : String(error)};
  }
}

if (process.argv[2] === '--once') {
  const lang = process.argv[3] || 'hi';
  const text = process.argv.slice(4).join(' ');
  process.stdout.write(`${JSON.stringify(respond({id: 1, lang, text}))}\n`);
} else {
  const lines = readline.createInterface({input: process.stdin, crlfDelay: Infinity});
  lines.on('line', (line) => {
    let request;
    try {
      request = JSON.parse(line);
    } catch (_) {
      process.stdout.write(`${JSON.stringify({id: 0, ok: false, error: 'invalid request'})}\n`);
      return;
    }
    process.stdout.write(`${JSON.stringify(respond(request))}\n`);
  });
  lines.on('close', () => process.exit(0));
}
