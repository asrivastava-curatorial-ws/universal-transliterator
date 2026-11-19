# Performance

Measured on 2026-11-19 on the build machine: a 2-vCPU Intel Xeon at 2.10 GHz running
Ubuntu 24.04, release build, wall-clock times. A desktop CPU will usually be faster.
The GUI figures come from the AppImage running on Xvfb with the Cairo renderer.

## Editor

| What | Measured |
|---|---|
| Launch to visible window (warm disk cache) | 0.25–0.33 s |
| Memory, editor process (Hindi / Japanese with libkkc loaded) | 135 MB / 189 MB resident |
| Memory, AI worker (only while an AI language has been used) | 165–180 MB resident |
| Output update after a typing pause | 90 ms debounce, then one render |

The editor never waits for the AI model. A new word first shows its Exact reading,
and the AI result replaces it within about one keystroke's time. Results are polled
every 30 ms. A render touches only the changed part of the output buffer, so the cursor,
selection, scroll position and undo history are kept.

## AI model (IndicXlit worker)

| What | Measured |
|---|---|
| Worker start, model load and first word | 0.40 s |
| A new, uncached word while typing (72 prefixes of 7 words) | median 51 ms, 90th percentile 98 ms, maximum 132 ms |
| 59 new words in one batch (for example, opening a file) | 2.8 s, about 21 words/s on one thread |
| A word seen before in this session | cache, under 1 ms |

The batched beam search with its exact early-stop bound is about three times faster than
the 1.1.0 worker, and the results are identical.

## Command line and other engines

Whole process, including start-up:

| What | Measured |
|---|---|
| Hindi, 10-word sentence, cold (process, Node, model) | 0.71 s |
| Hindi, 200 words with 59 distinct, cold | 3.5 s |
| Mandarin, 400 Pinyin syllables | 0.51 s (0.92 s on the very first run while Rime builds its user data) |
| Japanese, 20 sentences (150 words) | 0.07 s |
| Rule-based languages (Korean, Arabic, …), one call | 0.02 s |
| `--self-test`, every engine from cold | 1.0–1.1 s |

## Limits

| Resource | Limit |
|---|---:|
| Roman pane | 1 MB |
| Open / save / Word export | 16 MiB |
| Words per AI request (app / worker) | 512 / 4,000 |
| Word length sent to the AI model | 48 characters |
| AI result cache (worker / app, per model) | 20,000 / 60,000 words |
| Learned choices | 50,000 |
| Pinyin run handed to Rime in one piece | 60 characters |

## How to measure on your machine

```bash
time ./Universal-Transliterator-1.2.0-x86_64.AppImage --self-test
time ./Universal-Transliterator-1.2.0-x86_64.AppImage --to hi "mera naam abhisht hai"
```
