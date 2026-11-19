# Language support

42 languages. Every engine runs on your computer; nothing is downloaded.

## Indian languages — all 22 scheduled languages

| Language | Script | Engine | Example input → output |
|---|---|---|---|
| Hindi | Devanagari | AI + Exact | `namaste duniya` → नमस्ते दुनिया |
| Marathi | Devanagari | AI + Exact | `namaskar` → नमस्कार |
| Nepali | Devanagari | AI + Exact | `mero naam` → मेरो नाम |
| Sanskrit | Devanagari | AI + Exact | `vasudhaiva kuTumbakam` (Exact) → वसुधैव कुटुम्बकम् |
| Maithili | Devanagari | AI + Exact | `pranam` → प्रणाम |
| Bodo | Devanagari | AI + Exact | |
| Konkani | Devanagari | AI + Exact | `goem` → गोंय |
| Dogri | Devanagari | AI (Hindi model) + Exact | `jammu` → जम्मू |
| Bengali | Bengali | AI + Exact | `dhonnobad` → ধন্যবাদ |
| Assamese | Assamese (ৰ, ৱ) | AI + Exact | `oxom` → অসম, `guwahati` → গুৱাহাটী |
| Manipuri | Meetei Mayek | AI | `manipur` → ꯃꯅꯤꯄꯨꯔ |
| Gujarati | Gujarati | AI + Exact | `kem chho` → કેમ છો |
| Punjabi | Gurmukhi | AI + Exact | `punjab` → ਪੰਜਾਬ |
| Odia | Odia | AI + Exact | `namaskar` → ନମସ୍କାର |
| Tamil | Tamil | AI + Exact | `vanakkam` → வணக்கம் |
| Telugu | Telugu | AI + Exact | `namaskaram` → నమస్కారం |
| Kannada | Kannada | AI + Exact | `namaskara` → ನಮಸ್ಕಾರ |
| Malayalam | Malayalam | AI + Exact | `namaskaram` → നമസ്കാരം |
| Sinhala | Sinhala | AI + Exact | `sthuthiyi` → ස්තුතියි |
| Urdu | Perso-Arabic (Nastaliq) | AI | `aap kaise hain` → آپ کیسے ہیں |
| Kashmiri | Perso-Arabic | AI | `srinagar` → سرینگر |
| Sindhi | Perso-Arabic | AI | `sindhi` → سنڌي |
| Santali | Ol Chiki | Rules | `johar santaRi` → ᱡᱚᱦᱟᱨ ᱥᱟᱱᱛᱟᱲᱤ |

**AI** is the AI4Bharat IndicXlit transformer (21 language heads), run locally through
ONNX Runtime. It accepts natural, inconsistent spelling and offers up to five
alternatives per word plus the exact rule-based reading. Its Manipuri head writes Meetei
Mayek, the official script for Manipuri, so Manipuri has no Exact mode. The model reads
some short Hindi words as English abbreviations, so about 70 very common ones (`main`,
`nahi`, `kyun`, `aap`, `kahan`…) default to their standard spelling: `main ghar ja raha
hoon, tum kahan ho?` → मैं घर जा रहा हूँ, तुम कहाँ हो?

**Exact** is deterministic ITRANS with IAST / ISO 15919 letters, case-sensitive — see
the table in the [user manual](USER-MANUAL.md#exact-mode-itrans--iast). Sanskrit adds a
virama to word-final consonants; the others follow Hindi spelling. In Tamil, Telugu,
Kannada and Malayalam `e`/`o` are the short vowels and `E`/`O` the long ones.

Santali (Ol Chiki): `a` ᱟ · `o` ᱚ · `O` ᱳ · `e` ᱮ · `i` ᱤ · `u` ᱩ · `c`/`ch` ᱪ ·
`T D N R` (or `tt dd nn rr`) for ᱴ ᱰ ᱬ ᱲ · `kh gh …` aspirates with ᱷ · `~` nasal ᱸ ·
`|` ᱾.

## East and Southeast Asia

| Language | Engine | Scheme |
|---|---|---|
| Mandarin (Simplified or Traditional) | Rime `pinyin_simp` / `luna_pinyin` | Pinyin with or without spaces; tone marks accepted; `v` = ü; `'` separates syllables (`xi'an`). Punctuation becomes full-width. |
| Japanese | libkkc + SKK dictionary | Hepburn romaji. A phrase is converted as one sentence (`watashi ha gakusei desu` → 私は学生です); alternatives change one part at a time. `n'` = ん, `-` = ー. |
| Korean | Rules | Revised Romanization → Hangul (`annyeonghaseyo` → 안녕하세요). Ambiguous syllable splits are offered as alternatives (`yeongeo` → 연거 / 영어); `-` or `'` forces a split (`jung-ang` → 중앙). |
| Vietnamese | Rules (Telex) | `aa` â · `aw` ă · `ee` ê · `oo` ô · `ow` ơ · `uw` ư · `dd` đ · tones `s f r x j`, `z` clears (`tieengs Vieetj` → tiếng Việt). Words that are not Vietnamese syllables are left alone. |
| Thai | Keyboard | Type with the on-screen keyboard or your system Thai layout. |

## Middle East

| Language | Scheme |
|---|---|
| Arabic | Arabizi: `7` ح · `3` ع · `2` ء · `5` خ · `6` ط · capital `H S D T Z` for ح ص ض ط ظ · `aa` ا · `th kh dh sh gh`. Short *a* is not written; the version with it is an alternative, as are a final ة and unwritten short *i*/*u* (`3alaykum` → عليكم). `al-` and `ash-`-style articles → ال (`al-3arabiya` → العربية, `ash-shams` → الشمس). |
| Persian | Finglish: `aa` ا (آ at the start) · `kh` خ · `gh` غ · `sh` ش · `ch` چ · `zh` ژ · `g` گ · `p` پ · final `e` ه. |
| Hebrew | `t` ת · `T` ט · `k` כ · `q`/`K` ק · `ch`/`x` ח · `sh` ש · `tz` צ · `3` ע. Final letters (ך ם ן ף ץ) are automatic. |

Arabic-script output uses ، ؟ ؛ (and ۔ for Urdu); native digits are ٠–٩ or ۰–۹.

## Europe and the Caucasus

| Language | Scheme |
|---|---|
| Russian | `zh` ж · `kh`/`x` х · `ts`/`c` ц · `ch` ч · `sh` ш · `shch` щ · `ya yu yo ye` · `e'` э · `'` ь · `''` ъ; `y` after a vowel is й. |
| Ukrainian | `y` и · `i` і · `yi`/`ji` ї · `ye`/`je` є · `h` г · `g` ґ · `'` ь · `` ` `` apostrophe. |
| Belarusian | `y` ы · `i` і · `w` ў · `h` г · `eh` э · `'` ь. |
| Bulgarian | `y` ъ (й after a vowel: `zdravey` → здравей) · `sht` щ · `ya` я · `yu` ю. |
| Serbian | Latinica → ћирилица: `č ć š ž đ dž lj nj` (or `ch cj sh zh dj dzh`). |
| Macedonian | `gj` ѓ · `kj` ќ · `dz` ѕ · `lj` љ · `nj` њ · `dzh` џ · `j` ј. |
| Greek | `th` θ · `ch` χ · `ps` ψ · `h` η · `w` ω · `x`/`ks` ξ · `'` after a vowel adds tonos · final σ → ς · `?` → ; |
| Armenian | `y` յ · `u` ու · `ev` և · `ye` ե · `gh` ղ · `kh` խ · `rr` ռ · `t' p' k'` aspirates · `@` ը. |
| Georgian | `k` ქ · `k'` კ · `t` თ · `t'` ტ · `p` ფ · `p'` პ · `q` ყ · `ts` ც · `ts'` წ · `ch` ჩ · `ch'` ჭ · `dz` ძ · `zh` ჟ. |

## Africa

| Language | Scheme |
|---|---|
| Amharic, Tigrinya | Ge'ez syllables: consonant + `e` (1st order, ä) · `u` · `i` · `a` · `ie`/`E` · nothing (6th) · `o` · `wa`. Standalone vowels use the glottal series. Ejectives `T C P S`; Tigrinya `Q` ቐ. Doubled consonants are written once. `selam` → ሰላም, `amarnya` → አማርኛ. |

## Script conversion and romanization

From the output pane (<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>K</kbd>) any Indic text in
Devanagari, Bengali, Assamese, Gurmukhi, Gujarati, Odia, Tamil, Telugu, Kannada,
Malayalam or Sinhala converts to any other of those scripts, or to IAST, ISO 15919,
ITRANS or a plain English-style spelling. Script-specific spelling is applied: Gurmukhi
addak for doubled consonants, Malayalam chillu letters, Odia ୱ in clusters, Tamil
approximations for aspirated stops.

## On-screen keyboards

31 layouts: Devanagari, Bengali, Assamese, Gujarati, Gurmukhi, Odia, Tamil, Telugu,
Kannada, Malayalam, Sinhala, Arabic, Urdu/Kashmiri/Sindhi, Persian, Hebrew, Greek,
Russian, Ukrainian, Belarusian, Serbian, Macedonian, Armenian, Georgian, Korean jamo,
Japanese, Pinyin, Vietnamese, Thai, Ethiopic, Ol Chiki and Meetei Mayek — with
vowel-sign rows for the Indic scripts.
