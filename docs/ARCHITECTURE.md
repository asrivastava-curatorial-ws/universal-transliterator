# Architecture

Universal Transliterator is a Rust/GTK 4 application. It runs as one process, plus one
private child process for the IndicXlit model. Nothing listens on a socket and nothing
is fetched.

```text
┌──────────────────────────── GTK main thread ────────────────────────────┐
│ Roman buffer ─(90 ms debounce)─► tokenizer ─► per-word conversion ─┐    │
│                                    {braces},   learned choice       │    │
│                                    links       or engine result     ▼    │
│ Output buffer ◄── minimal edit ◄── 3-way merge ◄── rendered text + span map │
│      ▲                              (keeps your edits)                   │
│      └── alternatives bar ◄── word under the cursor (either pane)        │
└──────────────┬──────────────────────────────────────────────────────────┘
               │ mpsc channel                        in-process FFI
               ▼                                     ┌─────────────────────┐
   "indicxlit-worker" thread ── stdin/stdout ──►     │ librime  (Mandarin)  │
               │                JSON lines            │ libkkc   (Japanese)  │
               ▼                                     └─────────────────────┘
   node worker.js ─► ONNX Runtime ─► IndicXlit encoder/decoder (batched beam search)
```

## Conversion pipeline (`crates/app/src/pipeline.rs`)

1. **Tokenize.** The language's `WordRule` decides what a word is: ASCII letters for the
   AI model, letters plus ITRANS marks for Exact mode, digits inside words for Arabizi,
   whole phrases for Japanese, and syllable runs for Pinyin. `{…}` spans, URLs and
   e-mail addresses are protected (`crates/transliteration/src/tokenize.rs`).
2. **Convert each word.** A word the user has chosen an alternative for is looked up
   in the learned store first. The key is the namespace (language, Exact, script
   variant) plus the normalised word. Otherwise the language's engine gives the
   primary result:
   - **IndicXlit** answers come from a per-word cache. Uncached words are sent to the
     worker in one batch. Until the answer arrives, the Exact (ITRANS) reading stands in
     and the word is marked *pending*; the GTK loop polls for answers every 30 ms and
     renders again when they arrive.
   - **Rules** (`crates/transliteration`): ITRANS/IAST/ISO 15919 → Devanagari → any
     Brahmic script; Korean RR → Hangul; Vietnamese Telex; Ge'ez syllables; Ol Chiki;
     table and abjad engines for Greek, Cyrillic, Armenian, Georgian, Hebrew, Arabic
     and Persian.
   - **Rime** converts a Pinyin segment, and **libkkc** converts a whole romaji phrase.
     Both run synchronously through small FFI crates, with results cached per segment.
3. **Assemble.** Joining the results gives the output text and a span map: for every
   word, its character range in the Roman pane and in the output. The span map drives
   the alternatives bar, the word highlight in both panes, and learning.
4. **Merge.** `ut_text::merge3(previous render, current output, new render)` does a
   word-level three-way merge. Your manual edits survive as long as the Roman text
   around them doesn't change. When both sides changed the same word, the new
   conversion wins for that word.
5. **Apply.** `minimal_edit` turns the merged text into one replacement of the changed
   range, applied inside a single user action. The cursor, scroll position and undo
   history stay intact.

The command-line mode (`--to`) uses the same converter, synchronously.

## The IndicXlit worker (`resources/indic/worker.js`)

- Protocol: one JSON line per request,
  `{"id", "lang", "words": [...], "count"}` →
  `{"id", "ok", "lang", "words": {word: [candidates]}}`.
- Decoding: batched beam search, with all beams of a word decoded together and the
  encoder output tiled across the batch. Width is `max(5, min(9, count))`. Search stops
  early once no unfinished beam can beat the worst kept result under the length
  penalty `(5+L)/6` at the maximum length. This bound is exact, so results are identical
  to a full search, about three times faster.
- Limits: 4,000 words per request, 48 characters per word, a 20,000-entry cache, and
  at most `2 × length + 6` (capped at 40) decoding steps.
- ONNX Runtime runs on one intra-op thread with full graph optimisation.
- The `indicxlit-worker` thread starts Node with the first request: when an AI
  language is selected, or the first time a session uses one. Sessions that only use
  other engines never start it.

## State and files

| File | Location | Written |
|---|---|---|
| `settings.json` | `~/.config/universaltransliterator/` | on change |
| `session.json` | `$XDG_STATE_HOME/universal-transliterator/` or `~/.local/share/universaltransliterator/universal-transliterator/` | every 15 s while changed, and on exit |
| `learned.json` | `~/.local/share/universaltransliterator/` | when you pick an alternative |
| Rime user data | `~/.local/share/universaltransliterator/rime/` | first Mandarin use |

Every write goes to a temporary file in the same directory, which is then renamed into
place. The files are mode 0600 and the directories 0700. Values are validated and
clamped on load, and a damaged file falls back to defaults instead of stopping the app.

## User interface (`crates/app/src/app.rs`)

- A header bar holds the language button (a searchable popover: recent languages,
  regional groups, engine badges), the AI/Exact switch, script-variant selectors, Copy,
  and the main menu.
- Two `GtkTextView`s sit in a `GtkPaned`, side by side or stacked. Under them are the
  alternatives bar, the find/replace bar, the optional on-screen keyboard, and a status
  line with counts and engine state.
- Theming uses three CSS providers: the palette (GitHub dark or light, swapped at
  runtime), the structure, and the editor fonts. The output font stack puts the
  script's Noto font first and JetBrains Mono after it.
- `GApplication` keeps a single instance: a second launch opens its files in the
  running window. `--new-window` starts a separate instance.

## AppImage layout

```text
AppRun                         sets LD_LIBRARY_PATH, XDG_DATA_DIRS, GDK pixbuf loaders,
                               fontconfig, GSK_RENDERER=cairo and the UT_* data paths
usr/bin/universal-transliterator
usr/libexec/universal-transliterator/node
usr/lib/x86_64-linux-gnu/      GTK 4 and dependencies, librime, libkkc (+ language model),
                               libnode, gdk-pixbuf loaders (including SVG)
usr/share/universal-transliterator/indic/   models, ONNX Runtime binding, worker.js
usr/share/rime-data, usr/share/skk, usr/share/libkkc
usr/share/fonts/               JetBrains Mono, Noto Sans (Indic, Arabic, Hebrew, Armenian,
                               Georgian, Thai, Ethiopic, Ol Chiki, Meetei Mayek), Noto CJK
usr/share/doc/universal-transliterator/   licences, Debian copyright files, SPDX SBOM
```

glibc, the GL/EGL/Vulkan stacks and the network GIO modules are not bundled.
