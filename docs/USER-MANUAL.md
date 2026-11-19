# User manual

## The two panes

Type in **Roman letters** on the left. The **output** on the right updates as you type.
Both panes are ordinary editors: select, cut, paste and undo (<kbd>Ctrl</kbd>+<kbd>Z</kbd>)
work in each.

The output is yours to edit. If you fix a word on the right and then keep typing on the
left, your fix stays. If you later retype that same word on the left, the new spelling
wins for that word only.

## Choosing a language

Click the language button in the middle of the title bar or press
<kbd>Ctrl</kbd>+<kbd>L</kbd>. Type part of a name in English or in its own script
(`kan`, `ಕನ್ನಡ`, `tam`…) and press <kbd>Enter</kbd>. Recently used languages are at the
top; <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>L</kbd> switches back to the previous one —
handy when you alternate between, say, Hindi and Kannada.

Badges show how each language is converted:

| Badge | Engine |
|---|---|
| **AI** | AI4Bharat IndicXlit neural model, run locally. Natural, forgiving spelling. |
| **DICT** | Dictionary IME: Rime (Mandarin) or libkkc (Japanese). |
| **RULES** | Deterministic rules (Korean, Vietnamese Telex, Arabic, Persian, Hebrew, Greek, Cyrillic, Armenian, Georgian, Ethiopic, Ol Chiki). |
| **KEYS** | No conversion — type with the on-screen keyboard or your system layout (Thai). |

## Alternatives

Put the cursor on (or just after) a word in either pane, or click it. The bar under the
panes lists other spellings:

- <kbd>Alt</kbd>+<kbd>1</kbd> … <kbd>Alt</kbd>+<kbd>9</kbd> or a click picks one;
- **ITRANS** marks the exact rule-based reading;
- **as typed** keeps the Roman word unchanged (for English words in Indic text).

Picked alternatives are remembered for that word and language. To undo one, pick the
first (default) alternative again; to forget everything, use *Preferences → Forget
learned words*. Learning can be switched off.

In Hindi, about 70 very common chat words (`main`, `nahi`, `kyun`, `aap`, `kahan`…)
start with their standard spelling (मैं, नहीं, क्यों, आप, कहाँ). The AI model's own
readings are still in the bar if you meant something else.

For Mandarin, alternatives are whole readings of the Pinyin segment. For Japanese, a
phrase is converted as one sentence and the alternatives change one part (bunsetsu) at a
time.

## Keeping text unchanged

- `{anything}` — the braces disappear and the text inside is kept: `mera {laptop}` →
  `मेरा laptop`. Use `{{` for a literal `{`.
- Links (`https://…`, `www.…`) and e-mail addresses are kept automatically.
- Numbers stay as typed unless *Native digits* is on.

## Exact mode (ITRANS / IAST)

For Indic languages, the **Exact** switch (<kbd>Ctrl</kbd>+<kbd>M</kbd>) replaces the AI
model with deterministic rules. It is case-sensitive:

| Type | Get | Type | Get |
|---|---|---|---|
| `a aa/A i ii/I u uu/U` | अ आ इ ई उ ऊ | `RRi` / `R^i` / `ṛ` | ऋ |
| `e ai o au` | ए ऐ ओ औ | `M` / `.n` | ं (anusvara) |
| `k kh g gh ~N` | क ख ग घ ङ | `H` | ः (visarga) |
| `ch Ch j jh ~n` | च छ ज झ ञ | `.N` / `~` | ँ (chandrabindu) |
| `T Th D Dh N` | ट ठ ड ढ ण | `.h` | ् (explicit virama) |
| `t th d dh n` | त थ द ध न | `.a` / `'` | ऽ (avagraha) |
| `sh Sh s h` | श ष स ह | `OM` | ॐ |
| `kSh` / `x`, `GY` / `j~n` | क्ष, ज्ञ | `q K G z f .D .Dh` | क़ ख़ ग़ ज़ फ़ ड़ ढ़ |

IAST and ISO 15919 letters (`ā ī ū ṛ ṃ ḥ ś ṣ ṭ ḍ ṇ ñ ṅ`) work too. In Sanskrit a word
ending in a consonant gets a virama (`bhaarat` → भारत्); other languages follow Hindi
spelling (`bhaarat` → भारत). `|` and `||` give । and ॥ in every Indic mode.

Tamil, Telugu, Kannada and Malayalam have short and long *e* and *o*: there `e` and `o`
are the short vowels and `E` and `O` the long ones (`koDu` → ಕೊಡು, `kODu` → ಕೋಡು,
`maaDuttEne` → ಮಾಡುತ್ತೇನೆ). Elsewhere `e`/`E` and `o`/`O` are the same vowel.

## Script converter

<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>K</kbd> converts the output (or its selection)
between Indic scripts, or romanizes it as IAST, ISO 15919, ITRANS or simple Roman
(`अभिष्ट श्रीवास्तव` → `abhisht shrivastav`). Copy the result or replace the output with
it; <kbd>Ctrl</kbd>+<kbd>Z</kbd> in the output undoes a replacement.

## Files and sessions

- **Save** (<kbd>Ctrl</kbd>+<kbd>S</kbd>) writes the output as UTF-8 text.
- **Export Word document** (<kbd>Ctrl</kbd>+<kbd>E</kbd>) writes a .docx with the right
  direction for Arabic-script and Hebrew text.
- **Save Roman input as…** (menu) keeps your Roman source.
- **Open** (<kbd>Ctrl</kbd>+<kbd>O</kbd>, or drop a file on the window): plain ASCII files
  are imported into the Roman pane for conversion; anything else opens as the output
  document, and what you type is added after it.
- Your work (both panes, language, file name) is kept privately and restored on the next
  start, so closing never loses text. Turn this off in Preferences if you prefer to be
  asked about unsaved changes when closing.

Private data lives in `~/.config/universaltransliterator/` (settings) and
`~/.local/share/universaltransliterator/` (learned words, Rime data and the session, or
`$XDG_STATE_HOME/universal-transliterator/` for the session when that is set). Files are
created readable only by you.

## Command line

```bash
universal-transliterator --to hi "mera naam abhisht hai"
universal-transliterator --to kn < notes.txt > notes-kn.txt
universal-transliterator --to sa --exact "OM namaH shivaaya"
universal-transliterator --to hi --native-digits "2026 mein"
universal-transliterator --list-languages
universal-transliterator --self-test        # checks every engine offline
universal-transliterator --diagnostics      # shows where bundled data was found
```

Learned choices are applied on the command line too.

## Desktop integration

To get a menu entry and file associations for the AppImage, either use an AppImage
manager (Gear Lever, AppImageLauncher) or run:

```bash
mkdir -p ~/Applications ~/.local/share/applications ~/.local/share/icons/hicolor/scalable/apps
mv Universal-Transliterator-1.2.0-x86_64.AppImage ~/Applications/
cd ~/Applications
./Universal-Transliterator-1.2.0-x86_64.AppImage --appimage-extract 'usr/share/icons/hicolor/scalable/apps/*' >/dev/null
./Universal-Transliterator-1.2.0-x86_64.AppImage --appimage-extract 'usr/share/applications/*' >/dev/null
cp squashfs-root/usr/share/icons/hicolor/scalable/apps/*.svg ~/.local/share/icons/hicolor/scalable/apps/
sed "s|^Exec=universal-transliterator|Exec=$HOME/Applications/Universal-Transliterator-1.2.0-x86_64.AppImage|" \
  squashfs-root/usr/share/applications/*.desktop \
  | tee ~/.local/share/applications/org.universaltransliterator.UniversalTransliterator.desktop >/dev/null
rm -rf squashfs-root
update-desktop-database ~/.local/share/applications 2>/dev/null || true
```

## Troubleshooting

- **The window is blank or flickers** — start with `--safe-mode` (the AppImage already
  uses the conservative Cairo renderer unless `GSK_RENDERER` is set).
- **“AI model unavailable”** — the app keeps working with the exact rules; run
  `--diagnostics` and `--self-test` from a terminal to see what is missing.
- **FUSE errors when starting the AppImage** — the AppImage runtime is static and
  needs only the `fusermount3` tool (`sudo apt install fuse3`); `libfuse2` is not
  required. Without FUSE, run it with `--appimage-extract-and-run`.
