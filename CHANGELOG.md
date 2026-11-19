# Changelog

## 1.2.0 — 2026-11-19

A daily-driver release: a new interface, much better editing behaviour, and ten more
languages.

### Interface
- GitHub dark and light themes (Ctrl+Shift+T, or follow the system) with JetBrains Mono.
- Header bar with a searchable language picker (Ctrl+L): recent languages first,
  grouped by region, engine badges; Ctrl+Shift+L returns to the previous language.
- AI / Exact switch for Indic languages (Ctrl+M); Simplified/Traditional Chinese and
  Hiragana/Katakana selectors where they apply.
- Alternatives bar for the word under the cursor in either pane, with Alt+1…9; the word
  is highlighted in both panes.
- Find & replace bar (Ctrl+F / Ctrl+H) with match highlighting and Replace all.
- Zoom (Ctrl+= / Ctrl+- / Ctrl+0), side-by-side or stacked panes (Ctrl+Shift+V),
  word and character counts, cursor position, toast notifications.
- Preferences, keyboard-shortcuts and About windows; drag and drop of text files;
  files can be opened from the command line or the file manager.
- Script converter (Ctrl+Shift+K): Devanagari, Bengali, Assamese, Gurmukhi, Gujarati,
  Odia, Tamil, Telugu, Kannada, Malayalam and Sinhala, plus IAST, ISO 15919, ITRANS and
  simple Roman spelling.
- New icon; on-screen keyboards for 31 scripts with vowel-sign rows.
- Native-script text in labels and the find bar always uses the bundled Noto fonts
  (no broken fallback fonts); Latin words inside the output use Noto Sans.

### Behaviour
- Manual edits in the output pane are kept while you continue typing (word-level
  three-way merge). Previously the output was overwritten on every keystroke.
- Alternatives for every word, not only the last one; chosen alternatives are learned
  and reused (stored privately, can be forgotten in Preferences).
- Hindi: about 70 very common words that the model gets wrong on its own (`main` →
  मैन, `aap` → एएपी, `nahi` → नही) now default to their standard spelling (मैं, आप,
  नहीं); the model's readings stay in the alternatives.
- `{braces}` keep text exactly as typed; URLs and e-mail addresses are kept
  automatically.
- Session restore: both panes, the language and the file come back after closing.
  Unsaved-changes prompts for New and Open.
- Opening a plain Roman (ASCII) file imports it into the Roman pane; other files open as
  the output document and new typing is appended after them.
- Japanese is converted phrase by phrase (`watashi ha gakusei desu` → 私は学生です)
  with per-segment alternatives; previously each word was converted alone (私葉学生デス).
  A word-final `n` before `n…` stays ん (`takusan no hito` → たくさんの人).
- Mandarin alternatives are full-segment readings; Rime's own learning is disabled so
  conversions are deterministic; Chinese and Japanese punctuation become full-width.
- Native digits option; `|` and `||` become । and ॥; Arabic-script, Greek, Armenian and
  Ethiopic punctuation conventions.
- The AI model worker is about 3× faster (batched beam search with an exact early-stop
  bound, identical results) and results are cached per word. It starts only when an AI
  language is first used, so Mandarin, Japanese or other non-AI sessions save about
  180 MB of memory.
- Command-line mode: `--to LANG`, `--exact`, `--native-digits`, `--list-languages`.
- Single instance: launching again focuses the open window (`--new-window` to opt out).
  On X11 the window's class is the application ID, so KDE and GNOME show the right
  icon in the taskbar and can pin it.

### Languages
- New: Dogri, Santali (Ol Chiki) — completing all 22 scheduled languages of India —
  Persian, Ukrainian, Belarusian, Bulgarian, Serbian, Macedonian, Amharic, Tigrinya.
- Korean now converts Revised Romanization to Hangul (with syllable-split
  alternatives); Vietnamese uses Telex.
- Exact ITRANS / IAST / ISO 15919 engine for all Brahmic-script Indic languages
  (Sanskrit keeps final viramas; in Tamil, Telugu, Kannada and Malayalam `e`/`o` are
  short and `E`/`O` long, so `maaDuttEne` → ಮಾಡುತ್ತೇನೆ).
- Manipuri is labelled and rendered as Meetei Mayek, which is what the model writes
  (1.1.0 labelled it as Bengali script and used the Bengali keyboard).
- Greek final sigma and tonos, Hebrew final letters, Armenian ու / և, Georgian
  aspirate/ejective distinctions, Arabic and Persian vowel spelling with alternatives,
  Arabic definite article (`al-`, `ash-`) and ة endings.

### Fixes
- The 1.1.0 source did not compile: GTK 4.10+ APIs were used without enabling the
  `gtk4` feature (`v4_14`).
- `libonnxruntime.so.1` had been flattened into a 24-byte text file, so the AI model
  could not load; the library is now stored under its soname.
- Exporting a Word document over an existing file failed after the save dialog had
  already asked to replace it.
- Missing SVG icons in the AppImage (the gdk-pixbuf SVG loader is now bundled).
- The AppImage build always bundles Ubuntu's Node.js (with its licence) rather than any
  newer `node` on `PATH`, and no longer stores a second copy of ONNX Runtime.

## 1.1.0 — 2026-10-18

- Rust/GTK 4 rewrite of the earlier browser-based tool; two-pane workflow; offline
  IndicXlit neural conversion for 21 Indic languages; Rime Mandarin; libkkc Japanese;
  AppImage packaging.
