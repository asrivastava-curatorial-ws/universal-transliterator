# Known limitations

## Platform

- Linux x86-64 only. The AppImage is built on Ubuntu 24.04 and needs glibc 2.39 or
  newer, so it does not start on Ubuntu 22.04 or Debian 12. Other distributions of the
  same age as Ubuntu 24.04 should work, but only Ubuntu/Kubuntu 24.04 has been tested.
- The AppImage is about 130 MB because it carries Node.js, ONNX Runtime, the
  IndicXlit model, the Rime and libkkc dictionaries, and the fonts.
- It is an editor, not a system input method. To use the text elsewhere, copy it
  (<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>C</kbd>), save it, or export it.
- GTK draws with the Cairo renderer by default (`GSK_RENDERER=cairo`) for reliable
  output on every GPU driver. Set `GSK_RENDERER=ngl` before launching to try the GL
  renderer.

## Conversion quality

- **IndicXlit converts word by word**, without sentence context, and it sometimes reads
  a short word as an English abbreviation (`aap` → एएपी). For Hindi, about 70 of the
  most common chat words (`main`, `nahi`, `kyun`, `aap`, `kahan`…) default to their
  standard spelling. In the other languages, pick the right alternative once and it is
  remembered for that word.
- The first word in an AI language takes about half a second while the model loads.
  After that, each new word takes roughly 50–100 ms on a modest CPU and is cached. A
  long document opened in the Roman pane is converted in the background; the Exact
  reading shows until the AI result arrives.
- **Exact mode** is not available for Urdu, Kashmiri, Sindhi or Manipuri, whose
  scripts are not Brahmic. Manipuri is written in Meetei Mayek, the script the model
  produces; Bengali-script Manipuri is not offered.
- **Japanese** uses libkkc, whose kana–kanji conversion is weaker than Mozc's. Foreign
  names come out in kanji or hiragana; switch to Katakana or pick an alternative.
- **Mandarin** converts each Pinyin run as Rime would, without the full Rime
  candidate menu. Alternatives are whole readings of the segment.
- **Arabic, Persian and Hebrew** are rule-based. Short vowels are guessed, and the
  alternatives cover the usual other spellings (with or without written *a*, final ة,
  short *i*/*u*).
- **Korean** uses Revised Romanization. Where a syllable split is ambiguous
  (`yeongeo`), the alternatives offer both readings; type `-` to force a split.
- **Thai** has no transliteration. Use the on-screen keyboard or your system Thai
  layout.

## Editing

- If you edit a word in the output and then change the same word in the Roman pane,
  the new conversion replaces your edit for that word. Edits elsewhere are kept.
- The Roman pane holds up to 1 MB of text; files up to 16 MB can be opened and saved.
- There is no spell checker, printing or PDF export. Word export writes one paragraph
  per line with the script's font and right-to-left settings, without other formatting.
