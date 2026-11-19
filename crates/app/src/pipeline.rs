#![forbid(unsafe_code)]
//! The conversion pipeline: tokenizes Roman input, converts every word with
//! the language's engine (neural model, Rime, libkkc or rules), applies the
//! person's learned choices, and reports where each word landed in the
//! output so alternatives can be offered for any word.

use crate::indic;
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::path::{Path, PathBuf};
use ut_core::{CompositionEngine, EngineKey, EngineSnapshot};
use ut_japanese_engine::{JapaneseEngine, JapaneseInputMode};
use ut_rime_engine::RimeEngine;
use ut_storage::Learned;
use ut_transliteration::common::common_spelling;
use ut_transliteration::tokenize::{tokenize, TokenKind};
use ut_transliteration::{
    exact_indic, normalize_pinyin_input, punctuate, rule_candidates, word_rule, Engine,
    LanguageInfo, Script,
};

/// Longest Pinyin run handed to Rime in one piece.
const MAX_PINYIN_SEGMENT: usize = 60;
const MAX_CACHE_ENTRIES: usize = 60_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// Exact (ITRANS / IAST) mode for Indic languages.
    pub exact: bool,
    pub native_digits: bool,
    /// Traditional Chinese (Rime luna_pinyin) instead of Simplified.
    pub traditional: bool,
    /// Katakana composition for Japanese.
    pub katakana: bool,
}

/// Where one Roman word went.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordSpan {
    /// Character range in the Roman text.
    pub roman: Range<usize>,
    /// Character range in the rendered output.
    pub output: Range<usize>,
    /// Lookup key (normalized Roman word).
    pub key: String,
    /// The word exactly as typed.
    pub word: String,
    /// The text currently used for this word.
    pub text: String,
    /// Waiting for the AI model.
    pub pending: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Rendered {
    pub text: String,
    pub words: Vec<WordSpan>,
    pub pending: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub text: String,
    /// Short label shown next to special alternatives.
    pub hint: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusKind {
    Ready,
    Busy,
    Warning,
}

pub struct Converter {
    neural: Option<indic::Client>,
    neural_ready: bool,
    neural_error: Option<String>,
    neural_cache: HashMap<String, HashMap<String, Vec<String>>>,
    neural_pending: HashSet<(String, String)>,
    inflight: HashMap<u64, (String, Vec<String>)>,
    rime: Option<RimeEngine>,
    rime_schema: &'static str,
    rime_error: Option<String>,
    rime_user_dir: PathBuf,
    zh_primary: HashMap<(&'static str, String), String>,
    zh_alternatives: HashMap<(&'static str, String), Vec<String>>,
    kkc: Option<JapaneseEngine>,
    kkc_katakana: bool,
    kkc_error: Option<String>,
    ja_primary: HashMap<(bool, String), String>,
    ja_alternatives: HashMap<(bool, String), Vec<String>>,
    rules_cache: HashMap<(&'static str, String), Vec<String>>,
    pub learned: Learned,
}

impl Converter {
    /// `background_model` starts the AI worker as a background process (GUI);
    /// otherwise the model is run synchronously when needed (command line).
    pub fn new(rime_user_dir: PathBuf, learned: Learned, background_model: bool) -> Self {
        Self {
            neural: background_model.then(indic::Client::start),
            neural_ready: false,
            neural_error: None,
            neural_cache: HashMap::new(),
            neural_pending: HashSet::new(),
            inflight: HashMap::new(),
            rime: None,
            rime_schema: "",
            rime_error: None,
            rime_user_dir,
            zh_primary: HashMap::new(),
            zh_alternatives: HashMap::new(),
            kkc: None,
            kkc_katakana: false,
            kkc_error: None,
            ja_primary: HashMap::new(),
            ja_alternatives: HashMap::new(),
            rules_cache: HashMap::new(),
            learned,
        }
    }

    /// Loads the AI model in the background so the first word is fast.
    pub fn warm_up(&mut self, language: &LanguageInfo) {
        if let (Some(model), Some(client)) = (language.neural_model(), self.neural.as_mut()) {
            let key = (model.to_owned(), "namaste".to_owned());
            if !self.neural_pending.contains(&key) && !self.neural_ready {
                if let Some(id) = client.request(model, vec!["namaste".into()]) {
                    self.inflight
                        .insert(id, (model.to_owned(), vec!["namaste".into()]));
                    self.neural_pending.insert(key);
                }
            }
        }
    }

    /// Prepares the dictionary engine a language needs (may take a moment the
    /// first time Rime deploys its data).
    pub fn prepare(&mut self, language: &LanguageInfo, options: &Options) {
        match language.engine {
            Engine::Rime => {
                self.rime_engine(options.traditional);
            }
            Engine::Kkc => {
                self.kkc_engine(options.katakana);
            }
            _ => {}
        }
    }

    pub fn needs_preparation(&self, language: &LanguageInfo, options: &Options) -> bool {
        match language.engine {
            Engine::Rime => {
                self.rime.is_none() || self.rime_schema != schema_for(options.traditional)
            }
            Engine::Kkc => self.kkc.is_none() || self.kkc_katakana != options.katakana,
            _ => false,
        }
    }

    pub fn status(&self, language: &LanguageInfo, options: &Options) -> (StatusKind, String) {
        match language.engine {
            Engine::Neural { .. } if options.exact => (
                StatusKind::Ready,
                "Exact ITRANS / IAST rules · offline".into(),
            ),
            Engine::Neural { .. } => match (&self.neural_error, self.neural_ready) {
                (Some(error), _) => (
                    StatusKind::Warning,
                    format!("AI model unavailable ({error}); using ITRANS rules"),
                ),
                (None, false) if self.neural.is_some() => {
                    (StatusKind::Busy, "Loading the offline AI model…".into())
                }
                _ => (
                    StatusKind::Ready,
                    "AI4Bharat IndicXlit · running locally".into(),
                ),
            },
            Engine::Rime => match &self.rime_error {
                Some(error) => (StatusKind::Warning, format!("Rime unavailable: {error}")),
                None => (
                    StatusKind::Ready,
                    "Rime Pinyin dictionaries · offline".into(),
                ),
            },
            Engine::Kkc => match &self.kkc_error {
                Some(error) => (
                    StatusKind::Warning,
                    format!("Japanese engine unavailable: {error}"),
                ),
                None => (
                    StatusKind::Ready,
                    "libkkc kana–kanji conversion · offline".into(),
                ),
            },
            Engine::Rules => (
                StatusKind::Ready,
                "Rule-based transliteration · offline".into(),
            ),
            Engine::Keyboard => (
                StatusKind::Ready,
                "Direct typing — use the on-screen keyboard or your system layout".into(),
            ),
        }
    }

    pub fn has_pending(&self) -> bool {
        !self.inflight.is_empty()
    }

    /// Collects finished AI answers. Returns true when new results arrived.
    pub fn poll(&mut self) -> bool {
        let Some(client) = self.neural.as_ref() else {
            return false;
        };
        let mut changed = false;
        while let Some(response) = client.try_response() {
            let Some((model, words)) = self.inflight.remove(&response.id) else {
                continue;
            };
            for word in &words {
                self.neural_pending.remove(&(model.clone(), word.clone()));
            }
            let cache = self.neural_cache.entry(model).or_default();
            if cache.len() > MAX_CACHE_ENTRIES {
                cache.clear();
            }
            if response.ok {
                self.neural_ready = true;
                self.neural_error = None;
                cache.extend(response.words);
            } else {
                self.neural_error = Some(response.error);
                for word in words {
                    cache.entry(word).or_default();
                }
            }
            changed = true;
        }
        changed
    }

    /// Namespace for learned choices.
    pub fn namespace(language: &LanguageInfo, options: &Options) -> String {
        match language.engine {
            Engine::Neural { .. } if options.exact => format!("{}+exact", language.id),
            Engine::Rime => schema_namespace(options.traditional).into(),
            _ => language.id.into(),
        }
    }

    fn key(language: &LanguageInfo, options: &Options, word: &str) -> String {
        match language.engine {
            Engine::Neural { .. } if options.exact => word.to_owned(),
            Engine::Neural { .. } => word.to_ascii_lowercase(),
            Engine::Kkc => romaji_key(word),
            Engine::Rime => pinyin_key(word),
            Engine::Rules | Engine::Keyboard => word.to_owned(),
        }
    }

    /// Converts the whole Roman text.
    pub fn render(
        &mut self,
        language: &'static LanguageInfo,
        roman: &str,
        options: &Options,
    ) -> Rendered {
        let mut rendered = self.render_once(language, roman, options);
        if rendered.pending > 0 && self.neural.is_none() {
            // Command-line mode: run the model synchronously, then re-render.
            if let Some(model) = language.neural_model() {
                let words: Vec<String> = self
                    .neural_pending
                    .iter()
                    .filter(|(pending_model, _)| pending_model == model)
                    .map(|(_, word)| word.clone())
                    .collect();
                match indic::run_once(model, &words) {
                    Ok(results) => {
                        self.neural_ready = true;
                        self.neural_cache
                            .entry(model.into())
                            .or_default()
                            .extend(results);
                    }
                    Err(error) => {
                        self.neural_error = Some(error);
                        let cache = self.neural_cache.entry(model.into()).or_default();
                        for word in &words {
                            cache.entry(word.clone()).or_default();
                        }
                    }
                }
                self.neural_pending.clear();
                rendered = self.render_once(language, roman, options);
            }
        }
        rendered
    }

    fn render_once(
        &mut self,
        language: &'static LanguageInfo,
        roman: &str,
        options: &Options,
    ) -> Rendered {
        let rule = word_rule(language, options.exact);
        let tokens = tokenize(roman, &rule);
        let namespace = Self::namespace(language, options);
        let mut rendered = Rendered {
            text: String::with_capacity(roman.len() * 2),
            ..Rendered::default()
        };
        let mut output_chars = 0usize;
        let mut roman_chars = 0usize;
        let mut to_request: Vec<String> = Vec::new();
        let drop_spaces = matches!(language.script, Script::Han | Script::Japanese);
        for token in tokens {
            let token_chars = roman[token.start..token.end].chars().count();
            match token.kind {
                TokenKind::Word => {
                    let word = token.text(roman);
                    let key = Self::key(language, options, word);
                    let learned = self.learned.get(&namespace, &key).cloned();
                    let (text, pending) = match learned {
                        Some(choice) => {
                            // Keep the model's alternatives available for the bar.
                            self.ensure_requested(language, options, &key, &mut to_request);
                            (choice, false)
                        }
                        None => self.primary(language, options, word, &key, &mut to_request),
                    };
                    let start = output_chars;
                    output_chars += text.chars().count();
                    rendered.text.push_str(&text);
                    rendered.pending += usize::from(pending);
                    rendered.words.push(WordSpan {
                        roman: roman_chars..roman_chars + token_chars,
                        output: start..output_chars,
                        key,
                        word: word.to_owned(),
                        text,
                        pending,
                    });
                }
                TokenKind::Text => {
                    let mut text = punctuate(language, token.text(roman), options.native_digits);
                    if drop_spaces {
                        text.retain(|ch| ch != ' ' && ch != '\t');
                    }
                    output_chars += text.chars().count();
                    rendered.text.push_str(&text);
                }
                TokenKind::Protected => {
                    let text = token.inner(roman);
                    output_chars += text.chars().count();
                    rendered.text.push_str(text);
                }
            }
            roman_chars += token_chars;
        }
        if !to_request.is_empty() {
            if let (Some(model), Some(client)) = (language.neural_model(), self.neural.as_mut()) {
                for chunk in to_request.chunks(indic::MAX_WORDS_PER_REQUEST) {
                    match client.request(model, chunk.to_vec()) {
                        Some(id) => {
                            self.inflight.insert(id, (model.to_owned(), chunk.to_vec()));
                        }
                        None => {
                            for word in chunk {
                                self.neural_pending
                                    .remove(&(model.to_owned(), word.clone()));
                            }
                        }
                    }
                }
            }
        }
        rendered
    }

    /// Queues a word for the AI model if its alternatives are not known yet.
    fn ensure_requested(
        &mut self,
        language: &LanguageInfo,
        options: &Options,
        key: &str,
        to_request: &mut Vec<String>,
    ) {
        let Engine::Neural { model } = language.engine else {
            return;
        };
        if options.exact || self.neural.is_none() {
            return;
        }
        let cached = self
            .neural_cache
            .get(model)
            .is_some_and(|cache| cache.contains_key(key));
        let pending_key = (model.to_owned(), key.to_owned());
        if !cached && !self.neural_pending.contains(&pending_key) {
            self.neural_pending.insert(pending_key);
            to_request.push(key.to_owned());
        }
    }

    /// The engine's own first choice for a word (ignoring learned choices).
    /// The flag is true while the AI model has not answered yet.
    fn primary(
        &mut self,
        language: &'static LanguageInfo,
        options: &Options,
        word: &str,
        key: &str,
        to_request: &mut Vec<String>,
    ) -> (String, bool) {
        match language.engine {
            Engine::Neural { model } if !options.exact => {
                if let Some(common) = common_spelling(language.id, key) {
                    // The model's candidates are still wanted for the bar.
                    self.ensure_requested(language, options, key, to_request);
                    return (common.to_owned(), false);
                }
                if let Some(candidates) = self
                    .neural_cache
                    .get(model)
                    .and_then(|cache| cache.get(key))
                {
                    return match candidates.first() {
                        Some(first) => (first.clone(), false),
                        None => (neural_fallback(language, key), false),
                    };
                }
                if self.neural_error.is_some() && self.neural.is_some() {
                    return (neural_fallback(language, key), false);
                }
                let pending_key = (model.to_owned(), key.to_owned());
                if !self.neural_pending.contains(&pending_key) {
                    self.neural_pending.insert(pending_key);
                    to_request.push(key.to_owned());
                }
                (neural_fallback(language, key), true)
            }
            Engine::Neural { .. } => (
                exact_indic(language, word).unwrap_or_else(|| word.to_owned()),
                false,
            ),
            Engine::Rime => (
                self.zh_convert(key, options.traditional)
                    .unwrap_or_else(|| word.to_owned()),
                false,
            ),
            Engine::Kkc => (
                self.ja_convert(key, options.katakana)
                    .unwrap_or_else(|| word.to_owned()),
                false,
            ),
            Engine::Rules => (
                self.rules(language, word)
                    .first()
                    .cloned()
                    .unwrap_or_else(|| word.to_owned()),
                false,
            ),
            Engine::Keyboard => (word.to_owned(), false),
        }
    }

    fn rules(&mut self, language: &'static LanguageInfo, word: &str) -> Vec<String> {
        let key = (language.id, word.to_owned());
        if let Some(cached) = self.rules_cache.get(&key) {
            return cached.clone();
        }
        let candidates = rule_candidates(language, word);
        if self.rules_cache.len() > MAX_CACHE_ENTRIES {
            self.rules_cache.clear();
        }
        self.rules_cache.insert(key, candidates.clone());
        candidates
    }

    /// Alternatives for one word, the current choice included. `None` while
    /// the AI model is still working on the word.
    pub fn alternatives(
        &mut self,
        language: &'static LanguageInfo,
        options: &Options,
        span: &WordSpan,
    ) -> Option<Vec<Candidate>> {
        let mut list: Vec<Candidate> = Vec::new();
        let plain = |text: String| Candidate { text, hint: "" };
        match language.engine {
            Engine::Neural { model } if !options.exact => {
                let common = common_spelling(language.id, &span.key);
                let cached = self
                    .neural_cache
                    .get(model)
                    .and_then(|cache| cache.get(&span.key));
                if cached.is_none() && common.is_none() {
                    return None;
                }
                list.extend(common.map(|text| plain(text.to_owned())));
                list.extend(cached.into_iter().flatten().cloned().map(plain));
                // Capitals inside a word ("maaDuttEne") are intentional ITRANS;
                // a capital first letter is usually just sentence case.
                let intentional_case = span.word.chars().skip(1).any(char::is_uppercase);
                let reading = if intentional_case {
                    &span.word
                } else {
                    &span.key
                };
                if let Some(exact) = exact_indic(language, reading) {
                    list.push(Candidate {
                        text: exact,
                        hint: "ITRANS",
                    });
                }
            }
            Engine::Neural { .. } => {
                if let Some(exact) = exact_indic(language, &span.word) {
                    list.push(plain(exact));
                }
            }
            Engine::Rime => list.extend(
                self.zh_alternatives(&span.key, options.traditional)
                    .into_iter()
                    .map(plain),
            ),
            Engine::Kkc => list.extend(
                self.ja_alternatives(&span.key, options.katakana)
                    .into_iter()
                    .map(plain),
            ),
            Engine::Rules => list.extend(self.rules(language, &span.word).into_iter().map(plain)),
            Engine::Keyboard => {}
        }
        if !list.iter().any(|candidate| candidate.text == span.text) {
            list.insert(
                0,
                Candidate {
                    text: span.text.clone(),
                    hint: "chosen",
                },
            );
        }
        list.push(Candidate {
            text: span.word.clone(),
            hint: "as typed",
        });
        let mut seen = HashSet::new();
        list.retain(|candidate| !candidate.text.is_empty() && seen.insert(candidate.text.clone()));
        list.truncate(9);
        Some(list)
    }

    /// Records a chosen alternative. Choosing the engine's own first choice
    /// forgets the learned entry instead.
    pub fn choose(
        &mut self,
        language: &'static LanguageInfo,
        options: &Options,
        span: &WordSpan,
        text: &str,
    ) {
        let namespace = Self::namespace(language, options);
        let mut ignored = Vec::new();
        let (default, _) = self.primary(language, options, &span.word, &span.key, &mut ignored);
        for word in ignored {
            if let Some(model) = language.neural_model() {
                self.neural_pending.remove(&(model.to_owned(), word));
            }
        }
        if default == text {
            self.learned.remove(&namespace, &span.key);
        } else {
            self.learned.set(&namespace, &span.key, text);
        }
    }

    // --- Mandarin (Rime) -----------------------------------------------

    fn rime_engine(&mut self, traditional: bool) -> Option<&mut RimeEngine> {
        let schema = schema_for(traditional);
        if self.rime.is_none() || self.rime_schema != schema {
            self.rime = None;
            write_rime_patches(&self.rime_user_dir);
            match RimeEngine::new_with_schema(&rime_data_dir(), &self.rime_user_dir, schema) {
                Ok(engine) => {
                    self.rime = Some(engine);
                    self.rime_schema = schema;
                    self.rime_error = None;
                }
                Err(error) => self.rime_error = Some(error.public_message().to_owned()),
            }
        }
        self.rime.as_mut()
    }

    fn zh_convert(&mut self, key: &str, traditional: bool) -> Option<String> {
        let schema = schema_for(traditional);
        if let Some(cached) = self.zh_primary.get(&(schema, key.to_owned())) {
            return Some(cached.clone());
        }
        let mut output = String::new();
        for piece in split_pinyin(key) {
            let engine = self.rime_engine(traditional)?;
            output.push_str(&rime_full(engine, &piece, 0).unwrap_or(piece));
        }
        if self.zh_primary.len() > MAX_CACHE_ENTRIES {
            self.zh_primary.clear();
        }
        self.zh_primary
            .insert((schema, key.to_owned()), output.clone());
        Some(output)
    }

    fn zh_alternatives(&mut self, key: &str, traditional: bool) -> Vec<String> {
        let schema = schema_for(traditional);
        if let Some(cached) = self.zh_alternatives.get(&(schema, key.to_owned())) {
            return cached.clone();
        }
        let mut list = Vec::new();
        if key.len() <= MAX_PINYIN_SEGMENT {
            if let Some(engine) = self.rime_engine(traditional) {
                for choice in 0..9 {
                    match rime_full(engine, key, choice) {
                        Some(text) if !list.contains(&text) => list.push(text),
                        Some(_) => {}
                        None if choice > 0 => break,
                        None => {}
                    }
                }
            }
        }
        self.zh_alternatives
            .insert((schema, key.to_owned()), list.clone());
        list
    }

    // --- Japanese (libkkc) ---------------------------------------------

    fn kkc_engine(&mut self, katakana: bool) -> Option<&mut JapaneseEngine> {
        if self.kkc.is_none() || self.kkc_katakana != katakana {
            self.kkc = None;
            let mode = if katakana {
                JapaneseInputMode::Katakana
            } else {
                JapaneseInputMode::Hiragana
            };
            match JapaneseEngine::new_with_data(
                &kkc_dictionary(),
                Some(&kkc_model()),
                Some(&kkc_rule()),
                mode,
            ) {
                Ok(engine) => {
                    self.kkc = Some(engine);
                    self.kkc_katakana = katakana;
                    self.kkc_error = None;
                }
                Err(error) => self.kkc_error = Some(error.public_message().to_owned()),
            }
        }
        self.kkc.as_mut()
    }

    fn ja_convert(&mut self, key: &str, katakana: bool) -> Option<String> {
        if let Some(cached) = self.ja_primary.get(&(katakana, key.to_owned())) {
            return Some(cached.clone());
        }
        let engine = self.kkc_engine(katakana)?;
        let kana = feed(engine, key)?.preedit;
        let converted = engine
            .process(EngineKey::Space)
            .ok()
            .map(|snapshot| snapshot.preedit);
        let _ = engine.reset();
        let text = converted
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| finish_kana(&kana, katakana));
        if self.ja_primary.len() > MAX_CACHE_ENTRIES {
            self.ja_primary.clear();
        }
        self.ja_primary
            .insert((katakana, key.to_owned()), text.clone());
        Some(text)
    }

    /// Whole-sentence alternatives: for each segment (bunsetsu) of the
    /// conversion, the sentence with that segment's next candidates.
    fn ja_alternatives(&mut self, key: &str, katakana: bool) -> Vec<String> {
        if let Some(cached) = self.ja_alternatives.get(&(katakana, key.to_owned())) {
            return cached.clone();
        }
        let primary = self.ja_convert(key, katakana);
        let mut per_segment: Vec<Vec<String>> = Vec::new();
        if let Some(engine) = self.kkc_engine(katakana) {
            for segment in 0..4 {
                if feed(engine, key).is_none() || engine.process(EngineKey::Space).is_err() {
                    break;
                }
                if segment >= engine.segment_count() {
                    break;
                }
                for _ in 0..segment {
                    let _ = engine.process(EngineKey::Right);
                }
                let listed = engine
                    .process(EngineKey::Space)
                    .map(|snapshot| snapshot.candidates.len())
                    .unwrap_or(0);
                let mut previews = Vec::new();
                for index in 0..listed.min(if segment == 0 { 5 } else { 3 }) {
                    if let Ok(snapshot) = engine.preview(index) {
                        if !snapshot.preedit.is_empty()
                            && Some(&snapshot.preedit) != primary.as_ref()
                        {
                            previews.push(snapshot.preedit);
                        }
                    }
                }
                per_segment.push(previews);
            }
            let _ = engine.reset();
        }
        let mut list: Vec<String> = primary.iter().cloned().collect();
        let longest = per_segment.iter().map(Vec::len).max().unwrap_or(0);
        for round in 0..longest {
            for previews in &per_segment {
                if let Some(text) = previews.get(round) {
                    list.push(text.clone());
                }
            }
        }
        let mut seen = HashSet::new();
        list.retain(|item| !item.is_empty() && seen.insert(item.clone()));
        list.truncate(8);
        self.ja_alternatives
            .insert((katakana, key.to_owned()), list.clone());
        list
    }
}

fn neural_fallback(language: &LanguageInfo, key: &str) -> String {
    exact_indic(language, key).unwrap_or_else(|| key.to_owned())
}

fn schema_for(traditional: bool) -> &'static str {
    if traditional {
        "luna_pinyin"
    } else {
        "pinyin_simp"
    }
}

fn schema_namespace(traditional: bool) -> &'static str {
    if traditional {
        "zh-hant"
    } else {
        "zh-hans"
    }
}

/// Lowercase ASCII Pinyin without spaces; apostrophes split syllables.
fn pinyin_key(word: &str) -> String {
    normalize_pinyin_input(word)
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic() || *ch == '\'')
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

/// Lowercase romaji without spaces. A space after "n" before a vowel or "y"
/// means ん ("kin en" → "kin'en"), as the person intended a word break.
fn romaji_key(word: &str) -> String {
    let lower = word.to_ascii_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let mut key = String::with_capacity(lower.len());
    for (index, ch) in chars.iter().enumerate() {
        if *ch == ' ' || *ch == '\t' {
            let before_n =
                index > 0 && chars[index - 1] == 'n' && (index < 2 || chars[index - 2] != 'n');
            let next = chars[index + 1..]
                .iter()
                .find(|value| **value != ' ' && **value != '\t');
            if before_n
                // "takusan no" must stay たくさんの, not "takusanno" → たくさんお.
                && next.is_some_and(|value| {
                    matches!(value, 'a' | 'i' | 'u' | 'e' | 'o' | 'y' | 'n')
                })
            {
                key.push('\'');
            }
            continue;
        }
        key.push(*ch);
    }
    key
}

/// Splits very long Pinyin runs so Rime gets manageable pieces.
fn split_pinyin(key: &str) -> Vec<String> {
    if key.len() <= MAX_PINYIN_SEGMENT {
        return vec![key.to_owned()];
    }
    let mut pieces = Vec::new();
    let mut rest = key;
    while rest.len() > MAX_PINYIN_SEGMENT {
        // Prefer cutting before a consonant so syllables stay whole.
        let mut cut = MAX_PINYIN_SEGMENT;
        while cut > MAX_PINYIN_SEGMENT / 2
            && !matches!(
                rest.as_bytes()[cut],
                b'b' | b'p'
                    | b'm'
                    | b'f'
                    | b'd'
                    | b't'
                    | b'l'
                    | b'k'
                    | b'h'
                    | b'j'
                    | b'q'
                    | b'x'
                    | b'z'
                    | b'c'
                    | b's'
                    | b'r'
                    | b'w'
                    | b'y'
            )
        {
            cut -= 1;
        }
        pieces.push(rest[..cut].to_owned());
        rest = &rest[cut..];
    }
    if !rest.is_empty() {
        pieces.push(rest.to_owned());
    }
    pieces
}

/// Converts a Pinyin key with Rime, choosing `choice` on the first page and
/// the best candidate for whatever remains. Returns the full text.
fn rime_full(engine: &mut RimeEngine, key: &str, choice: usize) -> Option<String> {
    let _ = engine.process(EngineKey::Escape);
    let mut snapshot = EngineSnapshot::default();
    for ch in key.chars() {
        snapshot = engine.process(EngineKey::Char(ch)).ok()?;
    }
    if choice >= snapshot.candidates.len() {
        let _ = engine.process(EngineKey::Escape);
        return None;
    }
    let mut committed = String::new();
    let mut current = engine.process(EngineKey::Candidate(choice)).ok();
    for _ in 0..64 {
        let Some(mut state) = current.take() else {
            break;
        };
        if let Some(text) = state.committed.take() {
            committed.push_str(&text);
        }
        if !state.composing || state.candidates.is_empty() {
            break;
        }
        current = engine.process(EngineKey::Candidate(0)).ok();
    }
    let _ = engine.process(EngineKey::Escape);
    (!committed.is_empty() && !committed.is_ascii()).then_some(committed)
}

fn feed(engine: &mut JapaneseEngine, key: &str) -> Option<EngineSnapshot> {
    engine.reset().ok()?;
    let mut snapshot = EngineSnapshot::default();
    for ch in key.chars() {
        snapshot = engine.process(EngineKey::Char(ch)).ok()?;
    }
    Some(snapshot)
}

/// Turns a pending final "n" into ん / ン.
fn finish_kana(kana: &str, katakana: bool) -> String {
    match kana.strip_suffix('n') {
        Some(stem) => format!("{stem}{}", if katakana { 'ン' } else { 'ん' }),
        None => kana.to_owned(),
    }
}

/// Makes Rime deterministic for a re-rendering editor: no user-dictionary
/// learning (the app keeps its own learned choices), nine candidates per
/// page, and only the two bundled schemas.
pub fn write_rime_patches(user_dir: &Path) {
    let _ = std::fs::create_dir_all(user_dir);
    let files = [
        (
            "default.custom.yaml",
            "patch:\n  schema_list:\n    - schema: pinyin_simp\n    - schema: luna_pinyin\n  menu/page_size: 9\n",
        ),
        (
            "pinyin_simp.custom.yaml",
            "patch:\n  translator/enable_user_dict: false\n  menu/page_size: 9\n",
        ),
        (
            "luna_pinyin.custom.yaml",
            "patch:\n  translator/enable_user_dict: false\n  menu/page_size: 9\n",
        ),
    ];
    for (name, contents) in files {
        let path = user_dir.join(name);
        let current = std::fs::read_to_string(&path).unwrap_or_default();
        if current != contents {
            let _ = ut_storage::atomic_write_private(&path, contents.as_bytes(), true);
        }
    }
}

// --- Bundled data locations ------------------------------------------------

fn data_path(variable: &str, relative: &str, system: &str) -> PathBuf {
    if let Some(value) = std::env::var_os(variable) {
        return PathBuf::from(value);
    }
    if let Some(directory) = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
    {
        let candidate = directory.join("..").join(relative);
        if candidate.exists() {
            return candidate;
        }
    }
    PathBuf::from(system)
}

pub fn rime_data_dir() -> PathBuf {
    data_path(
        "UT_RIME_DATA_DIR",
        "share/rime-data",
        "/usr/share/rime-data",
    )
}

pub fn kkc_dictionary() -> PathBuf {
    data_path(
        "UT_KKC_DICTIONARY",
        "share/skk/SKK-JISYO.L",
        "/usr/share/skk/SKK-JISYO.L",
    )
}

pub fn kkc_model() -> PathBuf {
    data_path(
        "UT_KKC_MODEL_METADATA",
        "lib/x86_64-linux-gnu/libkkc/models/sorted3/metadata.json",
        "/usr/lib/x86_64-linux-gnu/libkkc/models/sorted3/metadata.json",
    )
}

pub fn kkc_rule() -> PathBuf {
    data_path(
        "UT_KKC_RULE_METADATA",
        "share/libkkc/rules/default/metadata.json",
        "/usr/share/libkkc/rules/default/metadata.json",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ut_transliteration::language;

    fn converter() -> Converter {
        let directory =
            std::env::temp_dir().join(format!("ut-pipeline-test-{}", std::process::id()));
        Converter::new(directory, Learned::default(), false)
    }

    #[test]
    fn rule_languages_render_with_spans() {
        let mut converter = converter();
        let korean = language("ko").unwrap();
        let rendered = converter.render(korean, "annyeonghaseyo, {Rust}!", &Options::default());
        assert_eq!(rendered.text, "안녕하세요, Rust!");
        assert_eq!(rendered.words.len(), 1);
        assert_eq!(rendered.words[0].roman, 0..14);
        assert_eq!(rendered.words[0].output, 0..5);
    }

    #[test]
    fn learned_choices_override_and_reset() {
        let mut converter = converter();
        let hindi = language("hi").unwrap();
        let options = Options {
            exact: true,
            ..Options::default()
        };
        let rendered = converter.render(hindi, "bhaarat", &options);
        assert_eq!(rendered.text, "भारत");
        let span = rendered.words[0].clone();
        converter.choose(hindi, &options, &span, "हिन्दुस्तान");
        assert_eq!(
            converter.render(hindi, "bhaarat", &options).text,
            "हिन्दुस्तान"
        );
        converter.choose(hindi, &options, &span, "भारत");
        assert!(converter.learned.is_empty());
    }

    #[test]
    fn common_hindi_words_use_standard_spelling() {
        let mut converter = converter();
        let hindi = language("hi").unwrap();
        let options = Options::default();
        let rendered = converter.render(hindi, "Main nahi", &options);
        assert_eq!(rendered.text, "मैं नहीं");
        assert_eq!(rendered.pending, 0);
        let list = converter
            .alternatives(hindi, &options, &rendered.words[0])
            .unwrap();
        assert_eq!(list[0].text, "मैं");
        // Picking the default again keeps nothing in the learned store.
        converter.choose(hindi, &options, &rendered.words[0], "मैं");
        assert!(converter.learned.is_empty());
    }

    #[test]
    fn alternatives_include_as_typed() {
        let mut converter = converter();
        let arabic = language("ar").unwrap();
        let rendered = converter.render(arabic, "salaam", &Options::default());
        let list = converter
            .alternatives(arabic, &Options::default(), &rendered.words[0])
            .unwrap();
        assert_eq!(list.first().map(|item| item.text.as_str()), Some("سلام"));
        assert!(list
            .iter()
            .any(|item| item.text == "salaam" && item.hint == "as typed"));
    }

    #[test]
    fn pinyin_keys_and_splitting() {
        assert_eq!(pinyin_key("Nǐ hǎo"), "nihao");
        assert_eq!(pinyin_key("xi'an"), "xi'an");
        let long = "woxiangqubeijing".repeat(6);
        let pieces = split_pinyin(&long);
        assert!(pieces.iter().all(|piece| piece.len() <= MAX_PINYIN_SEGMENT));
        assert_eq!(pieces.concat(), long);
    }

    #[test]
    fn romaji_keys_keep_n_breaks() {
        assert_eq!(romaji_key("kin en"), "kin'en");
        assert_eq!(romaji_key("Watashi ha"), "watashiha");
        assert_eq!(romaji_key("konnichiha sekai"), "konnichihasekai");
        assert_eq!(romaji_key("onna no"), "onnano");
        assert_eq!(romaji_key("takusan no hito"), "takusan'nohito");
    }

    #[test]
    fn finishing_kana() {
        assert_eq!(finish_kana("らーめn", false), "らーめん");
        assert_eq!(finish_kana("ラーメn", true), "ラーメン");
    }
}
