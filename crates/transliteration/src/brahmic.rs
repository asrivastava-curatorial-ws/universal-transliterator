//! Rule-based Indic ("Brahmic") transliteration.
//!
//! * [`itrans_to_devanagari`] reads ITRANS (with common casual aliases) and
//!   IAST / ISO 15919 diacritics and writes Devanagari.
//! * [`to_devanagari`] / [`from_devanagari`] convert between Devanagari and
//!   the other major Indic scripts. Together they give script-to-script
//!   conversion through a Devanagari pivot.
//! * [`romanize`] writes IAST, ISO 15919, ITRANS, or a plain "casual" Roman
//!   spelling suitable for English-language labels.
//!
//! Everything is table-driven, deterministic, and offline.

use unicode_normalization::UnicodeNormalization;

/// Indic scripts that the converter can read and write.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IndicScript {
    Devanagari,
    Bengali,
    Assamese,
    Gurmukhi,
    Gujarati,
    Odia,
    Tamil,
    Telugu,
    Kannada,
    Malayalam,
    Sinhala,
}

impl IndicScript {
    pub const ALL: [IndicScript; 11] = [
        IndicScript::Devanagari,
        IndicScript::Bengali,
        IndicScript::Assamese,
        IndicScript::Gurmukhi,
        IndicScript::Gujarati,
        IndicScript::Odia,
        IndicScript::Tamil,
        IndicScript::Telugu,
        IndicScript::Kannada,
        IndicScript::Malayalam,
        IndicScript::Sinhala,
    ];

    pub fn name(self) -> &'static str {
        match self {
            IndicScript::Devanagari => "Devanagari",
            IndicScript::Bengali => "Bengali",
            IndicScript::Assamese => "Assamese",
            IndicScript::Gurmukhi => "Gurmukhi",
            IndicScript::Gujarati => "Gujarati",
            IndicScript::Odia => "Odia",
            IndicScript::Tamil => "Tamil",
            IndicScript::Telugu => "Telugu",
            IndicScript::Kannada => "Kannada",
            IndicScript::Malayalam => "Malayalam",
            IndicScript::Sinhala => "Sinhala",
        }
    }

    /// Offset of the script's Unicode block from the Devanagari block, for the
    /// nine ISCII-derived blocks that share Devanagari's layout.
    fn offset(self) -> Option<u32> {
        match self {
            IndicScript::Devanagari => Some(0),
            IndicScript::Bengali | IndicScript::Assamese => Some(0x80),
            IndicScript::Gurmukhi => Some(0x100),
            IndicScript::Gujarati => Some(0x180),
            IndicScript::Odia => Some(0x200),
            IndicScript::Tamil => Some(0x280),
            IndicScript::Telugu => Some(0x300),
            IndicScript::Kannada => Some(0x380),
            IndicScript::Malayalam => Some(0x400),
            IndicScript::Sinhala => None,
        }
    }

    /// The script a character belongs to, if it is one of the supported blocks.
    pub fn of_char(ch: char) -> Option<IndicScript> {
        match ch as u32 {
            0x0900..=0x097F => Some(IndicScript::Devanagari),
            0x09F0 | 0x09F1 => Some(IndicScript::Assamese),
            0x0980..=0x09FF => Some(IndicScript::Bengali),
            0x0A00..=0x0A7F => Some(IndicScript::Gurmukhi),
            0x0A80..=0x0AFF => Some(IndicScript::Gujarati),
            0x0B00..=0x0B7F => Some(IndicScript::Odia),
            0x0B80..=0x0BFF => Some(IndicScript::Tamil),
            0x0C00..=0x0C7F => Some(IndicScript::Telugu),
            0x0C80..=0x0CFF => Some(IndicScript::Kannada),
            0x0D00..=0x0D7F => Some(IndicScript::Malayalam),
            0x0D80..=0x0DFF => Some(IndicScript::Sinhala),
            _ => None,
        }
    }
}

/// Roman output schemes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Romanization {
    /// International Alphabet of Sanskrit Transliteration.
    Iast,
    /// ISO 15919.
    Iso15919,
    /// ASCII ITRANS (round-trips through [`itrans_to_devanagari`]).
    Itrans,
    /// Plain English-style spelling without diacritics; drops the final
    /// inherent vowel the way Hindi speakers write names ("bharat").
    Casual,
}

impl Romanization {
    pub const ALL: [Romanization; 4] = [
        Romanization::Iast,
        Romanization::Iso15919,
        Romanization::Itrans,
        Romanization::Casual,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Romanization::Iast => "IAST",
            Romanization::Iso15919 => "ISO 15919",
            Romanization::Itrans => "ITRANS",
            Romanization::Casual => "Simple Roman",
        }
    }
}

/// Conversion target for [`convert`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Script(IndicScript),
    Roman(Romanization),
}

impl Target {
    pub fn all() -> Vec<Target> {
        IndicScript::ALL
            .iter()
            .map(|script| Target::Script(*script))
            .chain(
                Romanization::ALL
                    .iter()
                    .map(|scheme| Target::Roman(*scheme)),
            )
            .collect()
    }

    pub fn name(self) -> &'static str {
        match self {
            Target::Script(script) => script.name(),
            Target::Roman(scheme) => scheme.name(),
        }
    }
}

const VIRAMA: char = '\u{094D}';
const NUKTA: char = '\u{093C}';

// ---------------------------------------------------------------------------
// ITRANS / IAST / ISO 15919 → Devanagari
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum Kind {
    Consonant(&'static str),
    Vowel(&'static str, &'static str),
    /// Anusvara, chandrabindu, visarga: attach to the previous syllable.
    Sign(&'static str),
    Avagraha,
    Virama,
    Om,
    /// `_` separates tokens without producing output.
    Separator,
}

const ITRANS: &[(&str, Kind)] = &[
    // Conjunct shortcuts.
    ("kSh", Kind::Consonant("क्ष")),
    ("kSH", Kind::Consonant("क्ष")),
    ("ksh", Kind::Consonant("क्ष")),
    ("kṣ", Kind::Consonant("क्ष")),
    ("x", Kind::Consonant("क्ष")),
    ("GY", Kind::Consonant("ज्ञ")),
    ("dny", Kind::Consonant("ज्ञ")),
    ("j~n", Kind::Consonant("ज्ञ")),
    ("jñ", Kind::Consonant("ज्ञ")),
    // Velars.
    ("kh", Kind::Consonant("ख")),
    ("k", Kind::Consonant("क")),
    ("q", Kind::Consonant("क़")),
    ("K", Kind::Consonant("ख़")),
    ("gh", Kind::Consonant("घ")),
    ("g", Kind::Consonant("ग")),
    ("G", Kind::Consonant("ग़")),
    ("~N", Kind::Consonant("ङ")),
    ("N^", Kind::Consonant("ङ")),
    ("ṅ", Kind::Consonant("ङ")),
    // Palatals.
    ("chh", Kind::Consonant("छ")),
    ("Ch", Kind::Consonant("छ")),
    ("ch", Kind::Consonant("च")),
    ("c", Kind::Consonant("च")),
    ("jh", Kind::Consonant("झ")),
    ("j", Kind::Consonant("ज")),
    ("z", Kind::Consonant("ज़")),
    ("~n", Kind::Consonant("ञ")),
    ("JN", Kind::Consonant("ञ")),
    ("ñ", Kind::Consonant("ञ")),
    // Retroflexes.
    ("Th", Kind::Consonant("ठ")),
    ("T", Kind::Consonant("ट")),
    ("ṭh", Kind::Consonant("ठ")),
    ("ṭ", Kind::Consonant("ट")),
    (".Dh", Kind::Consonant("ढ़")),
    (".D", Kind::Consonant("ड़")),
    ("Rh", Kind::Consonant("ढ़")),
    ("R", Kind::Consonant("ड़")),
    ("Dh", Kind::Consonant("ढ")),
    ("D", Kind::Consonant("ड")),
    ("ḍh", Kind::Consonant("ढ")),
    ("ḍ", Kind::Consonant("ड")),
    ("N", Kind::Consonant("ण")),
    ("ṇ", Kind::Consonant("ण")),
    // Dentals.
    ("th", Kind::Consonant("थ")),
    ("t", Kind::Consonant("त")),
    ("dh", Kind::Consonant("ध")),
    ("d", Kind::Consonant("द")),
    ("n", Kind::Consonant("न")),
    // Labials.
    ("ph", Kind::Consonant("फ")),
    ("p", Kind::Consonant("प")),
    ("f", Kind::Consonant("फ़")),
    ("bh", Kind::Consonant("भ")),
    ("b", Kind::Consonant("ब")),
    ("m", Kind::Consonant("म")),
    // Approximants and sibilants.
    ("y", Kind::Consonant("य")),
    ("Y", Kind::Consonant("य़")),
    ("r", Kind::Consonant("र")),
    ("l", Kind::Consonant("ल")),
    ("L", Kind::Consonant("ळ")),
    ("ḻ", Kind::Consonant("ळ")),
    ("v", Kind::Consonant("व")),
    ("w", Kind::Consonant("व")),
    ("shh", Kind::Consonant("ष")),
    ("Sh", Kind::Consonant("ष")),
    ("S", Kind::Consonant("ष")),
    ("ṣ", Kind::Consonant("ष")),
    ("sh", Kind::Consonant("श")),
    ("ś", Kind::Consonant("श")),
    ("s", Kind::Consonant("स")),
    ("h", Kind::Consonant("ह")),
    // Vowels: (independent, dependent sign).
    ("a", Kind::Vowel("अ", "")),
    ("aa", Kind::Vowel("आ", "ा")),
    ("A", Kind::Vowel("आ", "ा")),
    ("ā", Kind::Vowel("आ", "ा")),
    ("i", Kind::Vowel("इ", "ि")),
    ("ii", Kind::Vowel("ई", "ी")),
    ("I", Kind::Vowel("ई", "ी")),
    ("ee", Kind::Vowel("ई", "ी")),
    ("ī", Kind::Vowel("ई", "ी")),
    ("u", Kind::Vowel("उ", "ु")),
    ("uu", Kind::Vowel("ऊ", "ू")),
    ("U", Kind::Vowel("ऊ", "ू")),
    ("oo", Kind::Vowel("ऊ", "ू")),
    ("ū", Kind::Vowel("ऊ", "ू")),
    ("RRi", Kind::Vowel("ऋ", "ृ")),
    ("R^i", Kind::Vowel("ऋ", "ृ")),
    ("Ri", Kind::Vowel("ऋ", "ृ")),
    ("ṛ", Kind::Vowel("ऋ", "ृ")),
    ("r\u{325}", Kind::Vowel("ऋ", "ृ")),
    ("RRI", Kind::Vowel("ॠ", "ॄ")),
    ("R^I", Kind::Vowel("ॠ", "ॄ")),
    ("ṝ", Kind::Vowel("ॠ", "ॄ")),
    ("r\u{325}\u{304}", Kind::Vowel("ॠ", "ॄ")),
    ("LLi", Kind::Vowel("ऌ", "ॢ")),
    ("L^i", Kind::Vowel("ऌ", "ॢ")),
    ("ḷ", Kind::Vowel("ऌ", "ॢ")),
    ("l\u{325}", Kind::Vowel("ऌ", "ॢ")),
    ("LLI", Kind::Vowel("ॡ", "ॣ")),
    ("L^I", Kind::Vowel("ॡ", "ॣ")),
    ("ḹ", Kind::Vowel("ॡ", "ॣ")),
    ("e", Kind::Vowel("ए", "े")),
    ("E", Kind::Vowel("ए", "े")),
    ("ē", Kind::Vowel("ए", "े")),
    ("ai", Kind::Vowel("ऐ", "ै")),
    ("ei", Kind::Vowel("ऐ", "ै")),
    ("o", Kind::Vowel("ओ", "ो")),
    ("O", Kind::Vowel("ओ", "ो")),
    ("ō", Kind::Vowel("ओ", "ो")),
    ("au", Kind::Vowel("औ", "ौ")),
    ("ou", Kind::Vowel("औ", "ौ")),
    ("ê", Kind::Vowel("ऍ", "ॅ")),
    ("ô", Kind::Vowel("ऑ", "ॉ")),
    // Signs.
    ("M", Kind::Sign("ं")),
    (".n", Kind::Sign("ं")),
    (".m", Kind::Sign("ं")),
    ("ṃ", Kind::Sign("ं")),
    ("ṁ", Kind::Sign("ं")),
    (".N", Kind::Sign("ँ")),
    ("m\u{310}", Kind::Sign("ँ")),
    ("~", Kind::Sign("ँ")),
    ("H", Kind::Sign("ः")),
    ("ḥ", Kind::Sign("ः")),
    (".a", Kind::Avagraha),
    ("'", Kind::Avagraha),
    (".h", Kind::Virama),
    ("OM", Kind::Om),
    ("AUM", Kind::Om),
    ("_", Kind::Separator),
];

fn longest_token(rest: &str) -> Option<(&'static str, Kind)> {
    ITRANS
        .iter()
        .filter(|(token, _)| rest.starts_with(token))
        .max_by_key(|(token, _)| token.len())
        .map(|(token, kind)| (*token, *kind))
}

/// Converts ITRANS/IAST/ISO 15919 text to Devanagari.
///
/// With `final_virama` a consonant that ends a word receives an explicit
/// virama (Sanskrit convention: `bhaarat` → भारत्). Without it the consonant
/// keeps its inherent vowel the way Hindi is written (`bhaarat` → भारत).
pub fn itrans_to_devanagari(input: &str, final_virama: bool) -> String {
    itrans_to_devanagari_with(input, final_virama, false)
}

/// Like [`itrans_to_devanagari`]. With `dravidian`, `e`/`o` are the short
/// vowels (ऎ ऒ) and `E`/`O`/`ē`/`ō` the long ones, as in ITRANS and ISO 15919
/// for Tamil, Telugu, Kannada and Malayalam.
pub fn itrans_to_devanagari_with(input: &str, final_virama: bool, dravidian: bool) -> String {
    let input: String = input.nfc().collect();
    let mut output = String::with_capacity(input.len() * 3);
    let mut offset = 0;
    let mut pending_consonant = false;
    while offset < input.len() {
        let rest = &input[offset..];
        if let Some((token, kind)) = longest_token(rest) {
            match kind {
                Kind::Consonant(value) => {
                    if pending_consonant {
                        output.push(VIRAMA);
                    }
                    output.push_str(value);
                    pending_consonant = true;
                }
                Kind::Vowel(independent, sign) => {
                    let (independent, sign) = match (dravidian, token) {
                        (true, "e") => ("ऎ", "ॆ"),
                        (true, "o") => ("ऒ", "ॊ"),
                        _ => (independent, sign),
                    };
                    output.push_str(if pending_consonant { sign } else { independent });
                    pending_consonant = false;
                }
                Kind::Sign(value) => {
                    output.push_str(value);
                    pending_consonant = false;
                }
                Kind::Avagraha => {
                    output.push('ऽ');
                    pending_consonant = false;
                }
                Kind::Virama => {
                    if pending_consonant {
                        output.push(VIRAMA);
                    }
                    pending_consonant = false;
                }
                Kind::Om => {
                    if pending_consonant && final_virama {
                        output.push(VIRAMA);
                    }
                    output.push('ॐ');
                    pending_consonant = false;
                }
                Kind::Separator => {}
            }
            offset += token.len();
            continue;
        }
        let ch = rest.chars().next().expect("offset is inside the input");
        if pending_consonant && final_virama {
            output.push(VIRAMA);
        }
        pending_consonant = false;
        output.push(ch);
        offset += ch.len_utf8();
    }
    if pending_consonant && final_virama {
        output.push(VIRAMA);
    }
    // NFC keeps nukta letters decomposed (U+0958..U+095F are composition
    // exclusions) and composes everything else.
    output.nfc().collect()
}

// ---------------------------------------------------------------------------
// Devanagari → other scripts
// ---------------------------------------------------------------------------

fn is_devanagari_consonant(ch: char) -> bool {
    matches!(ch as u32, 0x0915..=0x0939 | 0x0958..=0x095F | 0x0979..=0x097F)
}

/// Dependent vowel signs (matras).
fn is_matra(ch: char) -> bool {
    matches!(ch as u32, 0x093A..=0x094C | 0x094E | 0x094F | 0x0955..=0x0957 | 0x0962 | 0x0963)
}

fn is_letter_or_mark(ch: char) -> bool {
    ch.is_alphabetic() || matches!(ch as u32, 0x0900..=0x0DFF) && !ch.is_whitespace()
}

/// Script-specific replacements for Devanagari characters that have no
/// direct counterpart at the same offset.
fn special(script: IndicScript, ch: char) -> Option<&'static str> {
    use IndicScript::*;
    let value = match (script, ch) {
        // Bengali / Assamese.
        (Bengali | Assamese, 'ऍ' | 'ऎ') => "এ",
        (Bengali | Assamese, 'ॅ' | 'ॆ') => "ে",
        (Bengali | Assamese, 'ऑ') => "অ",
        (Bengali | Assamese, 'ॉ') => "",
        (Bengali | Assamese, 'ऒ') => "ও",
        (Bengali | Assamese, 'ॊ') => "ো",
        (Bengali | Assamese, 'ऩ') => "ন",
        (Bengali | Assamese, 'ऱ') => "র",
        (Bengali | Assamese, 'ळ' | 'ऴ') => "ল",
        (Bengali, 'व') => "ব",
        (Assamese, 'व') => "ৱ",
        (Assamese, 'र') => "ৰ",
        (Bengali | Assamese, 'ॐ') => "ওঁ",
        // Gurmukhi.
        (Gurmukhi, 'ँ' | 'ं') => "ਂ",
        (Gurmukhi, 'ऋ') => "ਰਿ",
        (Gurmukhi, 'ॠ') => "ਰੀ",
        (Gurmukhi, 'ऌ') => "ਲਿ",
        (Gurmukhi, 'ॡ') => "ਲੀ",
        (Gurmukhi, 'ृ') => "੍ਰਿ",
        (Gurmukhi, 'ॄ') => "੍ਰੀ",
        (Gurmukhi, 'ॢ') => "੍ਲਿ",
        (Gurmukhi, 'ॣ') => "੍ਲੀ",
        (Gurmukhi, 'ष') => "ਸ਼",
        (Gurmukhi, 'ऽ') => "ऽ",
        (Gurmukhi, 'ऍ' | 'ऎ') => "ਏ",
        (Gurmukhi, 'ॅ' | 'ॆ') => "ੇ",
        (Gurmukhi, 'ऑ') => "ਔ",
        (Gurmukhi, 'ॉ') => "ੌ",
        (Gurmukhi, 'ऒ') => "ਓ",
        (Gurmukhi, 'ॊ') => "ੋ",
        (Gurmukhi, 'ऩ') => "ਨ",
        (Gurmukhi, 'ऱ') => "ਰ",
        (Gurmukhi, 'ऴ') => "ਲ਼",
        (Gurmukhi, 'ॐ') => "ਓਂ",
        // Gujarati.
        (Gujarati, 'ऎ') => "એ",
        (Gujarati, 'ॆ') => "ે",
        (Gujarati, 'ऒ') => "ઓ",
        (Gujarati, 'ॊ') => "ો",
        (Gujarati, 'ऩ') => "ન",
        (Gujarati, 'ऱ') => "ર",
        (Gujarati, 'ऴ') => "ળ",
        // Odia.
        (Odia, 'ऍ' | 'ऎ') => "ଏ",
        (Odia, 'ॅ' | 'ॆ') => "େ",
        (Odia, 'ऑ') => "ଅ",
        (Odia, 'ॉ') => "",
        (Odia, 'ऒ') => "ଓ",
        (Odia, 'ॊ') => "ୋ",
        (Odia, 'ऩ') => "ନ",
        (Odia, 'ऱ') => "ର",
        (Odia, 'ऴ') => "ଳ",
        (Odia, 'ॐ') => "ଓଁ",
        // Tamil (no aspirated or voiced stops).
        (Tamil, 'ख' | 'ग' | 'घ') => "க",
        (Tamil, 'छ') => "ச",
        (Tamil, 'झ') => "ஜ",
        (Tamil, 'ठ' | 'ड' | 'ढ') => "ட",
        (Tamil, 'थ' | 'द' | 'ध') => "த",
        (Tamil, 'फ' | 'ब' | 'भ') => "ப",
        (Tamil, 'ँ' | 'ं') => "ம்",
        (Tamil, 'ः') => "ஃ",
        (Tamil, 'ऋ') => "ரு",
        (Tamil, 'ॠ') => "ரூ",
        (Tamil, 'ऌ') => "லு",
        (Tamil, 'ॡ') => "லூ",
        (Tamil, 'ृ') => "்ரு",
        (Tamil, 'ॄ') => "்ரூ",
        (Tamil, 'ॢ') => "்லு",
        (Tamil, 'ॣ') => "்லூ",
        (Tamil, 'ऍ') => "எ",
        (Tamil, 'ॅ') => "ெ",
        (Tamil, 'ऑ') => "ஆ",
        (Tamil, 'ॉ') => "ா",
        (Tamil, 'ऽ' | '\u{093C}') => "",
        (Tamil, 'ॐ') => "ஓம்",
        // Telugu.
        (Telugu, 'ऍ') => "ఎ",
        (Telugu, 'ॅ') => "ె",
        (Telugu, 'ऑ') => "ఆ",
        (Telugu, 'ॉ') => "ా",
        (Telugu, 'ऩ') => "న",
        (Telugu, '\u{093C}') => "",
        (Telugu, 'ॐ') => "ఓం",
        // Kannada.
        (Kannada, 'ऍ') => "ಎ",
        (Kannada, 'ॅ') => "ೆ",
        (Kannada, 'ऑ') => "ಆ",
        (Kannada, 'ॉ') => "ಾ",
        (Kannada, 'ऩ') => "ನ",
        (Kannada, 'ऴ') => "ೞ",
        (Kannada, 'ॐ') => "ಓಂ",
        // Malayalam.
        (Malayalam, 'ऍ') => "എ",
        (Malayalam, 'ॅ') => "െ",
        (Malayalam, 'ऑ') => "ഓ",
        (Malayalam, 'ॉ') => "ോ",
        (Malayalam, '\u{093C}') => "",
        (Malayalam, 'ॐ') => "ഓം",
        _ => return None,
    };
    Some(value)
}

fn sinhala(ch: char) -> Option<&'static str> {
    let value = match ch {
        'अ' => "අ",
        'आ' => "ආ",
        'इ' => "ඉ",
        'ई' => "ඊ",
        'उ' => "උ",
        'ऊ' => "ඌ",
        'ऋ' => "ඍ",
        'ॠ' => "ඎ",
        'ऌ' => "ඏ",
        'ॡ' => "ඐ",
        'ऎ' => "එ",
        'ए' => "ඒ",
        'ऐ' => "ඓ",
        'ऒ' => "ඔ",
        'ओ' => "ඕ",
        'औ' => "ඖ",
        'ऍ' => "ඇ",
        'ऑ' => "ඔ",
        'क' => "ක",
        'ख' => "ඛ",
        'ग' => "ග",
        'घ' => "ඝ",
        'ङ' => "ඞ",
        'च' => "ච",
        'छ' => "ඡ",
        'ज' => "ජ",
        'झ' => "ඣ",
        'ञ' => "ඤ",
        'ट' => "ට",
        'ठ' => "ඨ",
        'ड' => "ඩ",
        'ढ' => "ඪ",
        'ण' => "ණ",
        'त' => "ත",
        'थ' => "ථ",
        'द' => "ද",
        'ध' => "ධ",
        'न' | 'ऩ' => "න",
        'प' => "ප",
        'फ' => "ඵ",
        'ब' => "බ",
        'भ' => "භ",
        'म' => "ම",
        'य' => "ය",
        'र' | 'ऱ' => "ර",
        'ल' => "ල",
        'ळ' | 'ऴ' => "ළ",
        'व' => "ව",
        'श' => "ශ",
        'ष' => "ෂ",
        'स' => "ස",
        'ह' => "හ",
        'ा' => "ා",
        'ि' => "ි",
        'ी' => "ී",
        'ु' => "ු",
        'ू' => "ූ",
        'ृ' => "ෘ",
        'ॄ' => "ෲ",
        'ॢ' => "ෟ",
        'ॣ' => "ෳ",
        'ॆ' => "ෙ",
        'े' => "ේ",
        'ै' => "ෛ",
        'ॊ' => "ො",
        'ो' => "ෝ",
        'ौ' => "ෞ",
        'ॅ' => "ැ",
        'ॉ' => "ෝ",
        '्' => "්",
        'ं' | 'ँ' => "ං",
        'ः' => "ඃ",
        'ऽ' | '\u{093C}' => "",
        'ॐ' => "ඕම්",
        '०'..='९' => return None,
        _ => return None,
    };
    Some(value)
}

/// Converts Devanagari text to another Indic script. Characters outside the
/// Devanagari block pass through unchanged.
pub fn from_devanagari(text: &str, script: IndicScript) -> String {
    if script == IndicScript::Devanagari {
        return text.nfc().collect();
    }
    // NFC (not NFD): it splits the nukta letters U+0958..U+095F, which are
    // composition exclusions, but keeps two-part vowel signs of other
    // scripts (e.g. Kannada ೇ) intact.
    let chars: Vec<char> = text.nfc().collect();
    let mut output = String::with_capacity(text.len() * 2);
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        let next = chars.get(index + 1).copied();
        let after = chars.get(index + 2).copied();
        if !matches!(ch as u32, 0x0900..=0x097F) {
            output.push(ch);
            index += 1;
            continue;
        }
        // Script-specific orthography for consonant clusters.
        match script {
            IndicScript::Gurmukhi if is_devanagari_consonant(ch) && next == Some(VIRAMA) => {
                let base = map_one(ch, script);
                match after {
                    Some(following) if following == ch => {
                        output.push('ੱ');
                        output.push_str(&base);
                        index += 3;
                        continue;
                    }
                    Some('र' | 'व' | 'ह' | 'य') => {
                        output.push_str(&base);
                        output.push('੍');
                        index += 2;
                        continue;
                    }
                    _ => {
                        // Punjabi spelling writes most clusters without a halant.
                        output.push_str(&base);
                        index += 2;
                        continue;
                    }
                }
            }
            IndicScript::Malayalam if next == Some(VIRAMA) => {
                let word_ends = after.map_or(true, |following| !is_letter_or_mark(following));
                let chillu = match ch {
                    'ण' => Some('ൺ'),
                    'न' => Some('ൻ'),
                    'र' => Some('ർ'),
                    'ल' => Some('ൽ'),
                    'ळ' => Some('ൾ'),
                    _ => None,
                };
                if let (true, Some(chillu)) = (word_ends, chillu) {
                    output.push(chillu);
                    index += 2;
                    continue;
                }
            }
            IndicScript::Tamil if ch == 'फ' && next == Some(NUKTA) => {
                output.push_str("ஃப");
                index += 2;
                continue;
            }
            IndicScript::Odia if ch == 'व' => {
                let previous_is_virama = output.ends_with('୍');
                output.push(if previous_is_virama { 'ୱ' } else { 'ବ' });
                index += 1;
                continue;
            }
            _ => {}
        }
        output.push_str(&map_one(ch, script));
        index += 1;
    }
    output.nfc().collect()
}

fn map_one(ch: char, script: IndicScript) -> String {
    if matches!(ch, '।' | '॥') {
        return ch.to_string();
    }
    if script == IndicScript::Sinhala {
        if let Some(value) = sinhala(ch) {
            return value.to_owned();
        }
        if ('०'..='९').contains(&ch) {
            return char::from(b'0' + (ch as u32 - '०' as u32) as u8).to_string();
        }
        return ch.to_string();
    }
    if let Some(value) = special(script, ch) {
        return value.to_owned();
    }
    let offset = script.offset().unwrap_or(0);
    char::from_u32(ch as u32 + offset)
        .filter(|candidate| is_assigned_in(script, *candidate))
        .map_or_else(|| ch.to_string(), |candidate| candidate.to_string())
}

/// Conservative check that an offset-mapped code point is a real letter or
/// sign in the target block (rather than an unassigned or unrelated slot).
fn is_assigned_in(script: IndicScript, ch: char) -> bool {
    let relative = ch as u32 & 0x7F;
    let unassigned: &[u32] = match script {
        IndicScript::Bengali | IndicScript::Assamese => &[
            0x00, 0x04, 0x0D, 0x0E, 0x11, 0x12, 0x29, 0x31, 0x33, 0x34, 0x35, 0x3A, 0x3B, 0x45,
            0x46, 0x49, 0x4A, 0x4E, 0x4F, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57,
        ],
        IndicScript::Gurmukhi => &[
            0x00, 0x04, 0x0B, 0x0C, 0x0D, 0x0E, 0x11, 0x12, 0x29, 0x31, 0x34, 0x37, 0x3A, 0x3B,
            0x3D, 0x43, 0x44, 0x45, 0x46, 0x49, 0x4A, 0x4E, 0x4F, 0x50, 0x52, 0x53, 0x54, 0x55,
            0x56, 0x57, 0x60, 0x61, 0x62, 0x63,
        ],
        IndicScript::Gujarati => &[
            0x00, 0x04, 0x0E, 0x12, 0x29, 0x31, 0x34, 0x3A, 0x3B, 0x46, 0x4A, 0x4E, 0x4F, 0x51,
            0x52, 0x53, 0x54, 0x55, 0x56, 0x57,
        ],
        IndicScript::Odia => &[
            0x00, 0x04, 0x0D, 0x0E, 0x11, 0x12, 0x29, 0x31, 0x34, 0x3A, 0x3B, 0x45, 0x46, 0x49,
            0x4A, 0x4E, 0x4F, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57,
        ],
        IndicScript::Tamil => &[
            0x00, 0x01, 0x04, 0x0B, 0x0C, 0x0D, 0x11, 0x16, 0x17, 0x18, 0x1B, 0x1D, 0x20, 0x21,
            0x22, 0x25, 0x26, 0x27, 0x2B, 0x2C, 0x2D, 0x3A, 0x3B, 0x3C, 0x3D, 0x43, 0x44, 0x45,
            0x49, 0x4E, 0x4F, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x60, 0x61, 0x62, 0x63,
        ],
        IndicScript::Telugu => &[
            0x00, 0x04, 0x0D, 0x11, 0x29, 0x3A, 0x3B, 0x3C, 0x45, 0x49, 0x4E, 0x4F, 0x50, 0x51,
            0x52, 0x53, 0x54, 0x55, 0x56, 0x57,
        ],
        IndicScript::Kannada => &[
            0x00, 0x04, 0x0D, 0x11, 0x29, 0x34, 0x3A, 0x3B, 0x45, 0x49, 0x4E, 0x4F, 0x50, 0x51,
            0x52, 0x53, 0x54, 0x55, 0x56, 0x57,
        ],
        IndicScript::Malayalam => &[
            0x00, 0x04, 0x0D, 0x11, 0x3A, 0x3B, 0x3C, 0x45, 0x49, 0x4E, 0x4F, 0x50, 0x51, 0x52,
            0x53, 0x54, 0x55, 0x56, 0x57,
        ],
        IndicScript::Devanagari | IndicScript::Sinhala => &[],
    };
    !unassigned.contains(&relative)
}

// ---------------------------------------------------------------------------
// Any Indic script → Devanagari
// ---------------------------------------------------------------------------

fn sinhala_to_devanagari(ch: char) -> Option<&'static str> {
    let value = match ch {
        'අ' => "अ",
        'ආ' => "आ",
        'ඇ' => "ऍ",
        'ඈ' => "ऍ",
        'ඉ' => "इ",
        'ඊ' => "ई",
        'උ' => "उ",
        'ඌ' => "ऊ",
        'ඍ' => "ऋ",
        'ඎ' => "ॠ",
        'ඏ' => "ऌ",
        'ඐ' => "ॡ",
        'එ' => "ऎ",
        'ඒ' => "ए",
        'ඓ' => "ऐ",
        'ඔ' => "ऒ",
        'ඕ' => "ओ",
        'ඖ' => "औ",
        'ක' => "क",
        'ඛ' => "ख",
        'ග' => "ग",
        'ඝ' => "घ",
        'ඞ' => "ङ",
        'ඟ' => "ङ्ग",
        'ච' => "च",
        'ඡ' => "छ",
        'ජ' => "ज",
        'ඣ' => "झ",
        'ඤ' => "ञ",
        'ඥ' => "ज्ञ",
        'ට' => "ट",
        'ඨ' => "ठ",
        'ඩ' => "ड",
        'ඪ' => "ढ",
        'ණ' => "ण",
        'ඬ' => "ण्ड",
        'ත' => "त",
        'ථ' => "थ",
        'ද' => "द",
        'ධ' => "ध",
        'න' => "न",
        'ඳ' => "न्द",
        'ප' => "प",
        'ඵ' => "फ",
        'බ' => "ब",
        'භ' => "भ",
        'ම' => "म",
        'ඹ' => "म्ब",
        'ය' => "य",
        'ර' => "र",
        'ල' => "ल",
        'ව' => "व",
        'ශ' => "श",
        'ෂ' => "ष",
        'ස' => "स",
        'හ' => "ह",
        'ළ' => "ळ",
        'ෆ' => "फ़",
        'ා' => "ा",
        'ැ' => "ॅ",
        'ෑ' => "ॅ",
        'ි' => "ि",
        'ී' => "ी",
        'ු' => "ु",
        'ූ' => "ू",
        'ෘ' => "ृ",
        'ෲ' => "ॄ",
        'ෟ' => "ॢ",
        'ෳ' => "ॣ",
        'ෙ' => "ॆ",
        'ේ' => "े",
        'ෛ' => "ै",
        'ො' => "ॊ",
        'ෝ' => "ो",
        'ෞ' => "ौ",
        '්' => "्",
        'ං' => "ं",
        'ඃ' => "ः",
        _ => return None,
    };
    Some(value)
}

/// Converts text in any supported Indic script to Devanagari. Other text is
/// returned unchanged.
pub fn to_devanagari(text: &str) -> String {
    let chars: Vec<char> = text.nfc().collect();
    let mut output = String::with_capacity(text.len());
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        let next = chars.get(index + 1).copied();
        let Some(script) = IndicScript::of_char(ch) else {
            output.push(ch);
            index += 1;
            continue;
        };
        index += 1;
        let mapped: Option<String> = match (script, ch) {
            (IndicScript::Devanagari, _) => Some(ch.to_string()),
            (IndicScript::Sinhala, _) => {
                sinhala_to_devanagari(ch).map(str::to_owned).or_else(|| {
                    ('෦'..='෯')
                        .contains(&ch)
                        .then(|| char::from(b'0' + (ch as u32 - '෦' as u32) as u8).to_string())
                })
            }
            (_, 'ৰ') => Some("र".into()),
            (_, 'ৱ') => Some("व".into()),
            (_, 'ৎ') => Some("त्".into()),
            (_, 'ੰ') => Some("ं".into()),
            (_, 'ੱ') => {
                // Addak doubles the following consonant.
                match next.and_then(|following| reverse_offset(following, IndicScript::Gurmukhi)) {
                    Some(consonant) if is_devanagari_consonant(consonant) => {
                        Some(format!("{consonant}{VIRAMA}"))
                    }
                    _ => Some(String::new()),
                }
            }
            (_, 'ஃ') => {
                if next == Some('ப') {
                    index += 1;
                    Some("फ़".into())
                } else {
                    Some("ः".into())
                }
            }
            (_, 'ୱ') => Some("व".into()),
            (_, 'ൺ') => Some("ण्".into()),
            (_, 'ൻ') => Some("न्".into()),
            (_, 'ർ') => Some("र्".into()),
            (_, 'ൽ') => Some("ल्".into()),
            (_, 'ൾ') => Some("ळ्".into()),
            (_, 'ൿ') => Some("क्".into()),
            (_, 'ೞ') => Some("ऴ".into()),
            _ => reverse_offset(ch, script).map(String::from),
        };
        output.push_str(&mapped.unwrap_or_else(|| ch.to_string()));
    }
    output.nfc().collect()
}

fn reverse_offset(ch: char, script: IndicScript) -> Option<char> {
    let offset = script.offset()?;
    if !is_assigned_in(script, ch) {
        return None;
    }
    let candidate = char::from_u32((ch as u32).checked_sub(offset)?)?;
    matches!(candidate as u32, 0x0900..=0x097F).then_some(candidate)
}

/// Detects the dominant Indic script of a text.
pub fn detect(text: &str) -> Option<IndicScript> {
    let mut counts = [0usize; 11];
    for ch in text.chars() {
        if let Some(script) = IndicScript::of_char(ch) {
            if let Some(position) = IndicScript::ALL.iter().position(|item| *item == script) {
                counts[position] += 1;
            }
        }
    }
    let (position, count) = counts.iter().enumerate().max_by_key(|(_, count)| **count)?;
    (*count > 0).then(|| IndicScript::ALL[position])
}

// ---------------------------------------------------------------------------
// Romanization
// ---------------------------------------------------------------------------

struct RomanTable {
    vowels: fn(char) -> Option<&'static str>,
    consonant: fn(char, bool) -> Option<&'static str>,
    anusvara: &'static str,
    chandrabindu: &'static str,
    visarga: &'static str,
    avagraha: &'static str,
    om: &'static str,
    danda: &'static str,
    double_danda: &'static str,
}

fn iast_vowel(ch: char) -> Option<&'static str> {
    Some(match ch {
        'अ' => "a",
        'आ' | 'ा' => "ā",
        'इ' | 'ि' => "i",
        'ई' | 'ी' => "ī",
        'उ' | 'ु' => "u",
        'ऊ' | 'ू' => "ū",
        'ऋ' | 'ृ' => "ṛ",
        'ॠ' | 'ॄ' => "ṝ",
        'ऌ' | 'ॢ' => "ḷ",
        'ॡ' | 'ॣ' => "ḹ",
        'ए' | 'े' | 'ऎ' | 'ॆ' => "e",
        'ऐ' | 'ै' => "ai",
        'ओ' | 'ो' | 'ऒ' | 'ॊ' => "o",
        'औ' | 'ौ' => "au",
        'ऍ' | 'ॅ' => "ê",
        'ऑ' | 'ॉ' => "ô",
        _ => return None,
    })
}

fn iso_vowel(ch: char) -> Option<&'static str> {
    Some(match ch {
        'ऋ' | 'ृ' => "r̥",
        'ॠ' | 'ॄ' => "r̥̄",
        'ऌ' | 'ॢ' => "l̥",
        'ॡ' | 'ॣ' => "l̥̄",
        'ए' | 'े' => "ē",
        'ओ' | 'ो' => "ō",
        _ => return iast_vowel(ch),
    })
}

fn itrans_vowel(ch: char) -> Option<&'static str> {
    Some(match ch {
        'अ' => "a",
        'आ' | 'ा' => "A",
        'इ' | 'ि' => "i",
        'ई' | 'ी' => "I",
        'उ' | 'ु' => "u",
        'ऊ' | 'ू' => "U",
        'ऋ' | 'ृ' => "RRi",
        'ॠ' | 'ॄ' => "RRI",
        'ऌ' | 'ॢ' => "LLi",
        'ॡ' | 'ॣ' => "LLI",
        'ए' | 'े' | 'ऎ' | 'ॆ' => "e",
        'ऐ' | 'ै' => "ai",
        'ओ' | 'ो' | 'ऒ' | 'ॊ' => "o",
        'औ' | 'ौ' => "au",
        'ऍ' | 'ॅ' => "e",
        'ऑ' | 'ॉ' => "o",
        _ => return None,
    })
}

fn casual_vowel(ch: char) -> Option<&'static str> {
    Some(match ch {
        'अ' => "a",
        'आ' | 'ा' => "a",
        'इ' | 'ि' | 'ई' | 'ी' => "i",
        'उ' | 'ु' | 'ऊ' | 'ू' => "u",
        'ऋ' | 'ृ' | 'ॠ' | 'ॄ' => "ri",
        'ऌ' | 'ॢ' | 'ॡ' | 'ॣ' => "li",
        'ए' | 'े' | 'ऎ' | 'ॆ' | 'ऍ' | 'ॅ' => "e",
        'ऐ' | 'ै' => "ai",
        'ओ' | 'ो' | 'ऒ' | 'ॊ' | 'ऑ' | 'ॉ' => "o",
        'औ' | 'ौ' => "au",
        _ => return None,
    })
}

fn iast_consonant(ch: char, nukta: bool) -> Option<&'static str> {
    Some(match (ch, nukta) {
        ('क', true) => "q",
        ('ख', true) => "k͟h",
        ('ग', true) => "ġ",
        ('ज', true) => "z",
        ('ड', true) => "ṛ",
        ('ढ', true) => "ṛh",
        ('फ', true) => "f",
        ('य', true) => "ẏ",
        ('क', _) => "k",
        ('ख', _) => "kh",
        ('ग', _) => "g",
        ('घ', _) => "gh",
        ('ङ', _) => "ṅ",
        ('च', _) => "c",
        ('छ', _) => "ch",
        ('ज', _) => "j",
        ('झ', _) => "jh",
        ('ञ', _) => "ñ",
        ('ट', _) => "ṭ",
        ('ठ', _) => "ṭh",
        ('ड', _) => "ḍ",
        ('ढ', _) => "ḍh",
        ('ण', _) => "ṇ",
        ('त', _) => "t",
        ('थ', _) => "th",
        ('द', _) => "d",
        ('ध', _) => "dh",
        ('न', _) => "n",
        ('ऩ', _) => "ṉ",
        ('प', _) => "p",
        ('फ', _) => "ph",
        ('ब', _) => "b",
        ('भ', _) => "bh",
        ('म', _) => "m",
        ('य', _) => "y",
        ('र', _) => "r",
        ('ऱ', _) => "ṟ",
        ('ल', _) => "l",
        ('ळ', _) => "ḷ",
        ('ऴ', _) => "ḻ",
        ('व', _) => "v",
        ('श', _) => "ś",
        ('ष', _) => "ṣ",
        ('स', _) => "s",
        ('ह', _) => "h",
        _ => return None,
    })
}

fn itrans_consonant(ch: char, nukta: bool) -> Option<&'static str> {
    Some(match (ch, nukta) {
        ('क', true) => "q",
        ('ख', true) => "K",
        ('ग', true) => "G",
        ('ज', true) => "z",
        ('ड', true) => ".D",
        ('ढ', true) => ".Dh",
        ('फ', true) => "f",
        ('य', true) => "Y",
        ('ङ', _) => "~N",
        ('च', _) => "ch",
        ('छ', _) => "Ch",
        ('ञ', _) => "~n",
        ('ट', _) => "T",
        ('ठ', _) => "Th",
        ('ड', _) => "D",
        ('ढ', _) => "Dh",
        ('ण', _) => "N",
        ('ळ' | 'ऴ', _) => "L",
        ('ऩ', _) => "n",
        ('ऱ', _) => "r",
        ('श', _) => "sh",
        ('ष', _) => "Sh",
        _ => return iast_consonant(ch, false),
    })
}

fn casual_consonant(ch: char, nukta: bool) -> Option<&'static str> {
    Some(match (ch, nukta) {
        ('क', true) => "q",
        ('ख', true) => "kh",
        ('ग', true) => "g",
        ('ज', true) => "z",
        ('ड', true) => "r",
        ('ढ', true) => "rh",
        ('फ', true) => "f",
        ('य', true) => "y",
        ('ङ' | 'ञ' | 'ण' | 'ऩ', _) => "n",
        ('च', _) => "ch",
        ('छ', _) => "chh",
        ('ट', _) => "t",
        ('ठ', _) => "th",
        ('ड', _) => "d",
        ('ढ', _) => "dh",
        ('ळ' | 'ऴ', _) => "l",
        ('ऱ', _) => "r",
        ('श' | 'ष', _) => "sh",
        _ => return iast_consonant(ch, false),
    })
}

fn table(scheme: Romanization) -> RomanTable {
    match scheme {
        Romanization::Iast => RomanTable {
            vowels: iast_vowel,
            consonant: iast_consonant,
            anusvara: "ṃ",
            chandrabindu: "m̐",
            visarga: "ḥ",
            avagraha: "'",
            om: "oṃ",
            danda: "|",
            double_danda: "||",
        },
        Romanization::Iso15919 => RomanTable {
            vowels: iso_vowel,
            consonant: iast_consonant,
            anusvara: "ṁ",
            chandrabindu: "m̐",
            visarga: "ḥ",
            avagraha: "'",
            om: "ōṁ",
            danda: ".",
            double_danda: "..",
        },
        Romanization::Itrans => RomanTable {
            vowels: itrans_vowel,
            consonant: itrans_consonant,
            anusvara: "M",
            chandrabindu: ".N",
            visarga: "H",
            avagraha: ".a",
            om: "OM",
            danda: "|",
            double_danda: "||",
        },
        Romanization::Casual => RomanTable {
            vowels: casual_vowel,
            consonant: casual_consonant,
            anusvara: "n",
            chandrabindu: "n",
            visarga: "h",
            avagraha: "",
            om: "om",
            danda: ".",
            double_danda: ".",
        },
    }
}

/// Romanizes text in any supported Indic script.
pub fn romanize(text: &str, scheme: Romanization) -> String {
    let devanagari: Vec<char> = to_devanagari(text).chars().collect();
    let table = table(scheme);
    let mut output = String::with_capacity(text.len());
    // Byte position in `output` where the current word's inherent final vowel
    // may be removed (casual scheme only), plus the syllable count so far.
    let mut word_syllables = 0usize;
    let mut droppable_a: Option<usize> = None;
    let mut index = 0;
    while index < devanagari.len() {
        let ch = devanagari[index];
        let next = devanagari.get(index + 1).copied();
        if is_devanagari_consonant(ch) || matches!(ch, 'ऩ' | 'ऱ' | 'ऴ') {
            let nukta = next == Some(NUKTA);
            let step = if nukta { 2 } else { 1 };
            let following = devanagari.get(index + step).copied();
            let roman = (table.consonant)(ch, nukta).unwrap_or("");
            // Anusvara before a labial reads as "m" in plain spelling.
            output.push_str(roman);
            index += step;
            word_syllables += 1;
            match following {
                Some(VIRAMA) => {
                    index += 1;
                    droppable_a = None;
                }
                Some(sign) if is_matra(sign) && (table.vowels)(sign).is_some() => {
                    output.push_str((table.vowels)(sign).unwrap_or(""));
                    index += 1;
                    droppable_a = None;
                }
                _ => {
                    droppable_a = Some(output.len());
                    output.push('a');
                }
            }
            continue;
        }
        if let Some(vowel) = (table.vowels)(ch) {
            output.push_str(vowel);
            word_syllables += 1;
            droppable_a = None;
            index += 1;
            continue;
        }
        let sign = match ch {
            'ं' => {
                let labial = devanagari
                    .get(index + 1)
                    .is_some_and(|following| matches!(following, 'प' | 'फ' | 'ब' | 'भ' | 'म'));
                Some(if scheme == Romanization::Casual && labial {
                    "m"
                } else {
                    table.anusvara
                })
            }
            'ँ' => Some(table.chandrabindu),
            'ः' => Some(table.visarga),
            'ऽ' => Some(table.avagraha),
            'ॐ' => Some(table.om),
            '।' => Some(table.danda),
            '॥' => Some(table.double_danda),
            '्' | '\u{093C}' => Some(""),
            _ => None,
        };
        if let Some(sign) = sign {
            if matches!(ch, 'ं' | 'ँ' | 'ः') {
                droppable_a = None;
            }
            output.push_str(sign);
            index += 1;
            continue;
        }
        // Word boundary.
        finish_word(&mut output, scheme, &mut droppable_a, &mut word_syllables);
        if ('०'..='९').contains(&ch) {
            output.push(char::from(b'0' + (ch as u32 - '०' as u32) as u8));
        } else {
            output.push(ch);
        }
        index += 1;
    }
    finish_word(&mut output, scheme, &mut droppable_a, &mut word_syllables);
    output.nfc().collect()
}

fn finish_word(
    output: &mut String,
    scheme: Romanization,
    droppable_a: &mut Option<usize>,
    syllables: &mut usize,
) {
    if scheme == Romanization::Casual && *syllables > 1 {
        if let Some(position) = *droppable_a {
            if position + 1 == output.len() {
                output.truncate(position);
            }
        }
    }
    *droppable_a = None;
    *syllables = 0;
}

/// Converts Indic text (any supported script, auto-detected per character)
/// to the requested target.
pub fn convert(text: &str, target: Target) -> String {
    match target {
        Target::Script(script) => from_devanagari(&to_devanagari(text), script),
        Target::Roman(scheme) => romanize(text, scheme),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Normalizes an expected literal so tests do not depend on how an
    /// editor stored nukta or other composable characters.
    fn n(value: &str) -> String {
        value.nfc().collect()
    }

    #[test]
    fn itrans_hindi_style() {
        assert_eq!(itrans_to_devanagari("namaste", false), n("नमस्ते"));
        assert_eq!(itrans_to_devanagari("bhaarat", false), n("भारत"));
        assert_eq!(itrans_to_devanagari("shakti", false), n("शक्ति"));
        assert_eq!(itrans_to_devanagari("chaay", false), n("चाय"));
        assert_eq!(itrans_to_devanagari("kShatriya", false), n("क्षत्रिय"));
        assert_eq!(itrans_to_devanagari("j~naana", false), n("ज्ञान"));
        assert_eq!(itrans_to_devanagari("hindii", false), n("हिन्दी"));
        assert_eq!(itrans_to_devanagari("doodh", false), n("दूध"));
        assert_eq!(itrans_to_devanagari("zindagii", false), n("ज़िन्दगी"));
    }

    #[test]
    fn itrans_sanskrit_style() {
        assert_eq!(itrans_to_devanagari("saMskRRitam", true), n("संस्कृतम्"));
        assert_eq!(itrans_to_devanagari("bhaarat", true), n("भारत्"));
        assert_eq!(itrans_to_devanagari("raamaH", true), n("रामः"));
        assert_eq!(
            itrans_to_devanagari("OM namaH shivaaya", true),
            n("ॐ नमः शिवाय")
        );
        assert_eq!(itrans_to_devanagari("so.aham", true), n("सोऽहम्"));
    }

    #[test]
    fn dravidian_short_and_long_vowels() {
        let kannada = |text: &str| {
            from_devanagari(
                &itrans_to_devanagari_with(text, false, true),
                IndicScript::Kannada,
            )
        };
        assert_eq!(kannada("maaDuttEne"), n("ಮಾಡುತ್ತೇನೆ"));
        assert_eq!(kannada("kOTe"), n("ಕೋಟೆ"));
    }

    #[test]
    fn iast_input() {
        assert_eq!(itrans_to_devanagari("saṃskṛtam", true), n("संस्कृतम्"));
        assert_eq!(itrans_to_devanagari("kṛṣṇa", false), n("कृष्ण"));
        assert_eq!(itrans_to_devanagari("śiva", false), n("शिव"));
        // Decomposed diacritics are normalized first.
        assert_eq!(itrans_to_devanagari("ra\u{304}ma", false), n("राम"));
    }

    #[test]
    #[rustfmt::skip]
    fn script_to_script() {
        let hindi = "नमस्ते भारत";
        assert_eq!(from_devanagari(hindi, IndicScript::Kannada), n("ನಮಸ್ತೇ ಭಾರತ"));
        assert_eq!(from_devanagari(hindi, IndicScript::Telugu), n("నమస్తే భారత"));
        assert_eq!(from_devanagari(hindi, IndicScript::Bengali), n("নমস্তে ভারত"));
        assert_eq!(from_devanagari(hindi, IndicScript::Tamil), n("நமஸ்தே பாரத"));
        assert_eq!(
            from_devanagari(hindi, IndicScript::Gujarati),
            n("નમસ્તે ભારત")
        );
        assert_eq!(from_devanagari(hindi, IndicScript::Sinhala), n("නමස්තේ භාරත"));
        assert_eq!(from_devanagari("पक्का", IndicScript::Gurmukhi), n("ਪੱਕਾ"));
        assert_eq!(from_devanagari("नमस्ते", IndicScript::Gurmukhi), n("ਨਮਸਤੇ"));
        assert_eq!(from_devanagari("विश्व", IndicScript::Odia), n("ବିଶ୍ୱ"));
        assert_eq!(from_devanagari("अवन्", IndicScript::Malayalam), n("അവൻ"));
        assert_eq!(from_devanagari("रवि", IndicScript::Assamese), n("ৰৱি"));
        assert_eq!(from_devanagari("१२३", IndicScript::Kannada), n("೧೨೩"));
    }

    #[test]
    fn reverse_to_devanagari() {
        assert_eq!(to_devanagari("ನಮಸ್ತೇ"), n("नमस्ते"));
        assert_eq!(to_devanagari("ਪੱਕਾ"), n("पक्का"));
        assert_eq!(to_devanagari("നന്ദി"), n("नन्दि"));
        assert_eq!(to_devanagari("അവൻ"), n("अवन्"));
        assert_eq!(to_devanagari("ভারত"), n("भारत"));
        assert_eq!(to_devanagari("ভারত hello"), n("भारत hello"));
        assert_eq!(to_devanagari("නමස්තේ"), n("नमस्ते"));
    }

    #[test]
    fn romanization_schemes() {
        assert_eq!(romanize("संस्कृतम्", Romanization::Iast), n("saṃskṛtam"));
        assert_eq!(romanize("संस्कृतम्", Romanization::Iso15919), n("saṁskr̥tam"));
        assert_eq!(romanize("कृष्ण", Romanization::Itrans), n("kRRiShNa"));
        assert_eq!(
            romanize("अभिष्ट श्रीवास्तव", Romanization::Casual),
            n("abhisht shrivastav")
        );
        assert_eq!(romanize("भारत", Romanization::Casual), n("bharat"));
        assert_eq!(romanize("न", Romanization::Casual), n("na"));
        assert_eq!(romanize("ನಮಸ್ತೇ", Romanization::Iast), n("namaste"));
        assert_eq!(romanize("संबंध", Romanization::Casual), n("sambandh"));
    }

    #[test]
    fn itrans_round_trip() {
        for word in [
            "namaste",
            "kRRiShNa",
            "saMskRRitam",
            "j~nAna",
            "bhagavadgItA",
        ] {
            let devanagari = itrans_to_devanagari(word, true);
            let back = romanize(&devanagari, Romanization::Itrans);
            assert_eq!(itrans_to_devanagari(&back, true), devanagari, "{word}");
        }
    }

    #[test]
    fn detection() {
        assert_eq!(detect("ನಮಸ್ತೇ abc"), Some(IndicScript::Kannada));
        assert_eq!(detect("hello"), None);
    }
}
