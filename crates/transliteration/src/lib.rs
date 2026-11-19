#![forbid(unsafe_code)]
//! Language registry and rule-based transliteration engines for Universal
//! Transliterator. Everything here is deterministic and offline; the neural
//! (IndicXlit), Rime and libkkc engines live in other crates and are selected
//! through [`Engine`].

pub mod brahmic;
pub mod common;
pub mod ethiopic;
pub mod hangul;
pub mod olchiki;
pub mod tables;
pub mod tokenize;
pub mod vietnamese;

use brahmic::IndicScript;
use tokenize::{TokenKind, WordKind, WordRule};
use unicode_normalization::UnicodeNormalization;

/// How a language's Roman input is converted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Engine {
    /// AI4Bharat IndicXlit neural model. `model` is the model's language tag.
    Neural { model: &'static str },
    /// Rime Pinyin dictionaries (Mandarin).
    Rime,
    /// libkkc kana–kanji conversion (Japanese).
    Kkc,
    /// Deterministic rules in this crate.
    Rules,
    /// No conversion: use the on-screen keyboard or a system input method.
    Keyboard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    India,
    EastAsia,
    SoutheastAsia,
    MiddleEast,
    Europe,
    Africa,
}

impl Group {
    pub const ALL: [Group; 6] = [
        Group::India,
        Group::EastAsia,
        Group::SoutheastAsia,
        Group::MiddleEast,
        Group::Europe,
        Group::Africa,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Group::India => "Indian languages",
            Group::EastAsia => "East Asia",
            Group::SoutheastAsia => "Southeast Asia",
            Group::MiddleEast => "Middle East",
            Group::Europe => "Europe & Caucasus",
            Group::Africa => "Africa",
        }
    }
}

/// Output writing system, used for fonts, digits and punctuation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Script {
    Indic(IndicScript),
    Arabic,
    Nastaliq,
    Hebrew,
    Greek,
    Cyrillic,
    Armenian,
    Georgian,
    Han,
    Japanese,
    Hangul,
    Latin,
    Thai,
    Ethiopic,
    OlChiki,
    MeeteiMayek,
}

impl Script {
    /// Preferred font family for this script (all bundled in the AppImage).
    pub fn font_family(self) -> &'static str {
        match self {
            Script::Indic(IndicScript::Devanagari) => "Noto Sans Devanagari",
            Script::Indic(IndicScript::Bengali | IndicScript::Assamese) => "Noto Sans Bengali",
            Script::Indic(IndicScript::Gurmukhi) => "Noto Sans Gurmukhi",
            Script::Indic(IndicScript::Gujarati) => "Noto Sans Gujarati",
            Script::Indic(IndicScript::Odia) => "Noto Sans Oriya",
            Script::Indic(IndicScript::Tamil) => "Noto Sans Tamil",
            Script::Indic(IndicScript::Telugu) => "Noto Sans Telugu",
            Script::Indic(IndicScript::Kannada) => "Noto Sans Kannada",
            Script::Indic(IndicScript::Malayalam) => "Noto Sans Malayalam",
            Script::Indic(IndicScript::Sinhala) => "Noto Sans Sinhala",
            Script::Arabic => "Noto Naskh Arabic",
            Script::Nastaliq => "Noto Nastaliq Urdu",
            Script::Hebrew => "Noto Sans Hebrew",
            Script::Greek | Script::Cyrillic | Script::Latin => "JetBrains Mono",
            Script::Armenian => "Noto Sans Armenian",
            Script::Georgian => "Noto Sans Georgian",
            Script::Han => "Noto Sans CJK SC",
            Script::Japanese => "Noto Sans CJK JP",
            Script::Hangul => "Noto Sans CJK KR",
            Script::Thai => "Noto Sans Thai",
            Script::Ethiopic => "Noto Sans Ethiopic",
            Script::OlChiki => "Noto Sans Ol Chiki",
            Script::MeeteiMayek => "Noto Sans Meetei Mayek",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LanguageInfo {
    /// Stable identifier (ISO 639 code).
    pub id: &'static str,
    /// English name.
    pub name: &'static str,
    /// Name in the language's own script.
    pub native: &'static str,
    pub group: Group,
    /// On-screen keyboard layout id.
    pub keyboard: &'static str,
    pub engine: Engine,
    pub script: Script,
    pub rtl: bool,
    /// Example Roman input, shown as a hint.
    pub example: &'static str,
    /// One-line description of the typing scheme.
    pub note: &'static str,
}

impl LanguageInfo {
    pub fn label(&self) -> String {
        format!("{} · {}", self.native, self.name)
    }

    /// Whether the language offers the rule-based "Exact (ITRANS/IAST)" mode
    /// next to the AI model.
    pub fn supports_exact(&self) -> bool {
        matches!(self.engine, Engine::Neural { .. }) && self.indic_script().is_some()
    }

    pub fn indic_script(&self) -> Option<IndicScript> {
        match self.script {
            Script::Indic(script) => Some(script),
            _ => None,
        }
    }

    pub fn is_neural(&self) -> bool {
        matches!(self.engine, Engine::Neural { .. })
    }

    pub fn neural_model(&self) -> Option<&'static str> {
        match self.engine {
            Engine::Neural { model } => Some(model),
            _ => None,
        }
    }

    /// Short engine badge for the UI.
    pub fn engine_label(&self, exact: bool) -> &'static str {
        match self.engine {
            Engine::Neural { .. } if exact => "Exact · ITRANS / IAST",
            Engine::Neural { .. } => "AI · IndicXlit",
            Engine::Rime => "Dictionary · Rime",
            Engine::Kkc => "Dictionary · libkkc",
            Engine::Rules => "Rules",
            Engine::Keyboard => "Keyboard",
        }
    }
}

const AI_NOTE: &str = "Type the way you text. Click a word or press Alt+1…9 for alternatives.";
/// Typing hint for the exact (ITRANS / IAST) mode of Indic languages.
pub const EXACT_NOTE: &str = "ITRANS / IAST (case-sensitive): aa A = आ · ii I = ई · T Th D Dh N = retroflex · sh = श · Sh = ष · ~n = ञ · M = ं · H = ः · .h = ् · Dravidian: e o short, E O long";
macro_rules! indic {
    ($id:expr, $name:expr, $native:expr, $keyboard:expr, $model:expr, $script:expr, $example:expr) => {
        LanguageInfo {
            id: $id,
            name: $name,
            native: $native,
            group: Group::India,
            keyboard: $keyboard,
            engine: Engine::Neural { model: $model },
            script: Script::Indic($script),
            rtl: false,
            example: $example,
            note: AI_NOTE,
        }
    };
}

pub const LANGUAGES: &[LanguageInfo] = &[
    indic!("hi", "Hindi", "हिन्दी", "devanagari", "hi", IndicScript::Devanagari, "namaste, mera naam abhisht hai"),
    indic!("mr", "Marathi", "मराठी", "devanagari", "mr", IndicScript::Devanagari, "namaskar, tumhi kase aahat"),
    indic!("ne", "Nepali", "नेपाली", "devanagari", "ne", IndicScript::Devanagari, "namaste, tapailai kasto chha"),
    indic!("sa", "Sanskrit", "संस्कृतम्", "devanagari", "sa", IndicScript::Devanagari, "vasudhaiva kutumbakam"),
    indic!("mai", "Maithili", "मैथिली", "devanagari", "mai", IndicScript::Devanagari, "pranam, aha kona chhi"),
    indic!("brx", "Bodo", "बड़ो", "devanagari", "brx", IndicScript::Devanagari, "nwng mabwrwi dong"),
    indic!("gom", "Konkani", "कोंकणी", "devanagari", "gom", IndicScript::Devanagari, "dev borem korum"),
    LanguageInfo {
        id: "doi",
        name: "Dogri",
        native: "डोगरी",
        group: Group::India,
        keyboard: "devanagari",
        engine: Engine::Neural { model: "hi" },
        script: Script::Indic(IndicScript::Devanagari),
        rtl: false,
        example: "tusen kiyan o",
        note: "Uses the Hindi model, which writes Devanagari. Exact mode follows ITRANS.",
    },
    indic!("bn", "Bengali", "বাংলা", "bengali", "bn", IndicScript::Bengali, "ami tomake bhalobashi"),
    indic!("as", "Assamese", "অসমীয়া", "assamese", "as", IndicScript::Assamese, "apuni kene asse"),
    LanguageInfo {
        id: "mni",
        name: "Manipuri (Meetei Mayek)",
        native: "ꯃꯤꯇꯩꯂꯣꯟ",
        group: Group::India,
        keyboard: "meetei",
        engine: Engine::Neural { model: "mni" },
        script: Script::MeeteiMayek,
        rtl: false,
        example: "manipur",
        note: AI_NOTE,
    },
    indic!("gu", "Gujarati", "ગુજરાતી", "gujarati", "gu", IndicScript::Gujarati, "kem chho, majama"),
    indic!("pa", "Punjabi", "ਪੰਜਾਬੀ", "gurmukhi", "pa", IndicScript::Gurmukhi, "sat sri akal, ki haal hai"),
    indic!("or", "Odia", "ଓଡ଼ିଆ", "odia", "or", IndicScript::Odia, "namaskar, apana kemiti achhanti"),
    indic!("ta", "Tamil", "தமிழ்", "tamil", "ta", IndicScript::Tamil, "vanakkam, eppadi irukkeenga"),
    indic!("te", "Telugu", "తెలుగు", "telugu", "te", IndicScript::Telugu, "namaskaram, ela unnaru"),
    indic!("kn", "Kannada", "ಕನ್ನಡ", "kannada", "kn", IndicScript::Kannada, "namaskara, hegiddira"),
    indic!("ml", "Malayalam", "മലയാളം", "malayalam", "ml", IndicScript::Malayalam, "namaskaram, sukhamano"),
    indic!("si", "Sinhala", "සිංහල", "sinhala", "si", IndicScript::Sinhala, "ayubowan"),
    LanguageInfo {
        id: "ur",
        name: "Urdu",
        native: "اردو",
        group: Group::India,
        keyboard: "perso_arabic",
        engine: Engine::Neural { model: "ur" },
        script: Script::Nastaliq,
        rtl: true,
        example: "aap kaise hain",
        note: AI_NOTE,
    },
    LanguageInfo {
        id: "ks",
        name: "Kashmiri",
        native: "کٲشُر",
        group: Group::India,
        keyboard: "perso_arabic",
        engine: Engine::Neural { model: "ks" },
        script: Script::Nastaliq,
        rtl: true,
        example: "toh kyah chhiv karan",
        note: AI_NOTE,
    },
    LanguageInfo {
        id: "sd",
        name: "Sindhi",
        native: "سنڌي",
        group: Group::India,
        keyboard: "perso_arabic",
        engine: Engine::Neural { model: "sd" },
        script: Script::Arabic,
        rtl: true,
        example: "kiyan aahiyo",
        note: AI_NOTE,
    },
    LanguageInfo {
        id: "sat",
        name: "Santali (Ol Chiki)",
        native: "ᱥᱟᱱᱛᱟᱲᱤ",
        group: Group::India,
        keyboard: "olchiki",
        engine: Engine::Rules,
        script: Script::OlChiki,
        rtl: false,
        example: "johar, santaRi",
        note: "Ol Chiki: a ᱟ · o ᱚ · O ᱳ · e ᱮ · T D N R = retroflex · kh gh … = aspirates · ~ = nasal · | = ᱾",
    },
    LanguageInfo {
        id: "zh",
        name: "Mandarin (Pinyin)",
        native: "中文",
        group: Group::EastAsia,
        keyboard: "chinese",
        engine: Engine::Rime,
        script: Script::Han,
        rtl: false,
        example: "ni hao, shijie",
        note: "Pinyin with or without spaces; tone marks are fine; ü = v; ' splits syllables (xi'an).",
    },
    LanguageInfo {
        id: "ja",
        name: "Japanese (romaji)",
        native: "日本語",
        group: Group::EastAsia,
        keyboard: "japanese",
        engine: Engine::Kkc,
        script: Script::Japanese,
        rtl: false,
        example: "watashi ha gakusei desu",
        note: "Hepburn romaji; a phrase is converted as one sentence; n' = ん; - = ー; alternatives per phrase part.",
    },
    LanguageInfo {
        id: "ko",
        name: "Korean (romanized)",
        native: "한국어",
        group: Group::EastAsia,
        keyboard: "korean",
        engine: Engine::Rules,
        script: Script::Hangul,
        rtl: false,
        example: "annyeonghaseyo, saranghae",
        note: "Revised Romanization (eo, eu, ae, ya …); a - or ' splits syllables: jung-ang = 중앙.",
    },
    LanguageInfo {
        id: "vi",
        name: "Vietnamese (Telex)",
        native: "Tiếng Việt",
        group: Group::SoutheastAsia,
        keyboard: "vietnamese",
        engine: Engine::Rules,
        script: Script::Latin,
        rtl: false,
        example: "xin chaof, tieengs Vieetj",
        note: "Telex: aa â · aw ă · ee ê · oo ô · ow ơ · uw ư · dd đ · tones s f r x j · z clears.",
    },
    LanguageInfo {
        id: "th",
        name: "Thai (keyboard)",
        native: "ไทย",
        group: Group::SoutheastAsia,
        keyboard: "thai",
        engine: Engine::Keyboard,
        script: Script::Thai,
        rtl: false,
        example: "",
        note: "Type with the on-screen keyboard or your system Thai layout; text is kept as typed.",
    },
    LanguageInfo {
        id: "ar",
        name: "Arabic",
        native: "العربية",
        group: Group::MiddleEast,
        keyboard: "arabic",
        engine: Engine::Rules,
        script: Script::Arabic,
        rtl: true,
        example: "mar7aba, salaam",
        note: "Arabizi: 7 ح · 3 ع · 2 ء · 5 خ · H S D T Z = emphatics · aa = ا. Short a is not written.",
    },
    LanguageInfo {
        id: "fa",
        name: "Persian",
        native: "فارسی",
        group: Group::MiddleEast,
        keyboard: "persian",
        engine: Engine::Rules,
        script: Script::Arabic,
        rtl: true,
        example: "salaam, khaane",
        note: "Finglish: aa ا · kh خ · gh غ · sh ش · ch چ · zh ژ · final e ه.",
    },
    LanguageInfo {
        id: "he",
        name: "Hebrew",
        native: "עברית",
        group: Group::MiddleEast,
        keyboard: "hebrew",
        engine: Engine::Rules,
        script: Script::Hebrew,
        rtl: true,
        example: "shalom, toda",
        note: "t ת · T ט · k כ · q ק · ch ח · sh ש · tz צ · 3 ע; final letters are automatic.",
    },
    LanguageInfo {
        id: "ru",
        name: "Russian",
        native: "Русский",
        group: Group::Europe,
        keyboard: "cyrillic",
        engine: Engine::Rules,
        script: Script::Cyrillic,
        rtl: false,
        example: "privet, kak dela",
        note: "zh ж · kh х · ts ц · ch ч · sh ш · shch щ · ya я · yu ю · yo ё · e' э · ' ь · '' ъ",
    },
    LanguageInfo {
        id: "uk",
        name: "Ukrainian",
        native: "Українська",
        group: Group::Europe,
        keyboard: "ukrainian",
        engine: Engine::Rules,
        script: Script::Cyrillic,
        rtl: false,
        example: "dyakuyu, Kyjiv",
        note: "y и · i і · ji ї · je є · h г · g ґ · ' ь · ` apostrophe",
    },
    LanguageInfo {
        id: "be",
        name: "Belarusian",
        native: "Беларуская",
        group: Group::Europe,
        keyboard: "belarusian",
        engine: Engine::Rules,
        script: Script::Cyrillic,
        rtl: false,
        example: "dzyakuj, Minsk",
        note: "y ы · i і · w ў · h г · eh э · ' ь · ` apostrophe",
    },
    LanguageInfo {
        id: "bg",
        name: "Bulgarian",
        native: "Български",
        group: Group::Europe,
        keyboard: "cyrillic",
        engine: Engine::Rules,
        script: Script::Cyrillic,
        rtl: false,
        example: "zdravei, Bylgariya",
        note: "y ъ · sht щ · zh ж · ch ч · sh ш · ya я · yu ю",
    },
    LanguageInfo {
        id: "sr",
        name: "Serbian",
        native: "Српски",
        group: Group::Europe,
        keyboard: "serbian",
        engine: Engine::Rules,
        script: Script::Cyrillic,
        rtl: false,
        example: "zdravo, Beograd",
        note: "Latinica → ћирилица: č ć š ž đ dž lj nj (or ch cj sh zh dj dzh).",
    },
    LanguageInfo {
        id: "mk",
        name: "Macedonian",
        native: "Македонски",
        group: Group::Europe,
        keyboard: "macedonian",
        engine: Engine::Rules,
        script: Script::Cyrillic,
        rtl: false,
        example: "zdravo, Makedonija",
        note: "gj ѓ · kj ќ · dz ѕ · lj љ · nj њ · dzh џ · j ј",
    },
    LanguageInfo {
        id: "el",
        name: "Greek",
        native: "Ελληνικά",
        group: Group::Europe,
        keyboard: "greek",
        engine: Engine::Rules,
        script: Script::Greek,
        rtl: false,
        example: "kalhme'ra, Athina",
        note: "th θ · ch χ · ps ψ · h η · w ω · x ξ · ' after a vowel adds a tonos · final σ → ς",
    },
    LanguageInfo {
        id: "hy",
        name: "Armenian",
        native: "Հայերեն",
        group: Group::Europe,
        keyboard: "armenian",
        engine: Engine::Rules,
        script: Script::Armenian,
        rtl: false,
        example: "barev, Hayastan",
        note: "y յ · u ու · ev և · gh ղ · kh խ · rr ռ · t' p' k' = aspirates · @ ը",
    },
    LanguageInfo {
        id: "ka",
        name: "Georgian",
        native: "ქართული",
        group: Group::Europe,
        keyboard: "georgian",
        engine: Engine::Rules,
        script: Script::Georgian,
        rtl: false,
        example: "gamarjoba, sakartvelo",
        note: "k ქ · k' კ · t თ · t' ტ · p ფ · p' პ · q ყ · ts ც · ts' წ · ch ჩ · ch' ჭ",
    },
    LanguageInfo {
        id: "am",
        name: "Amharic",
        native: "አማርኛ",
        group: Group::Africa,
        keyboard: "ethiopic",
        engine: Engine::Rules,
        script: Script::Ethiopic,
        rtl: false,
        example: "selam, amarnya",
        note: "e ä · u · i · a · ie é · o; a consonant alone takes the 6th order. T C P S = ejectives.",
    },
    LanguageInfo {
        id: "ti",
        name: "Tigrinya",
        native: "ትግርኛ",
        group: Group::Africa,
        keyboard: "ethiopic",
        engine: Engine::Rules,
        script: Script::Ethiopic,
        rtl: false,
        example: "selam, tgrnya",
        note: "e ä · u · i · a · ie é · o; a consonant alone takes the 6th order. Q = ቐ, K = ኸ.",
    },
];

pub const DEFAULT_LANGUAGE: &str = "hi";

pub fn language(id: &str) -> Option<&'static LanguageInfo> {
    LANGUAGES.iter().find(|language| language.id == id)
}

pub fn language_index(id: &str) -> Option<usize> {
    LANGUAGES.iter().position(|language| language.id == id)
}

/// Which characters form convertible words for a language and mode.
pub fn word_rule(language: &LanguageInfo, exact: bool) -> WordRule {
    match language.engine {
        Engine::Neural { .. } if exact && language.supports_exact() => {
            WordRule::new(WordKind::Letters, ".~^'_", "~", false)
        }
        Engine::Neural { .. } => WordRule::new(WordKind::AsciiLetters, "", "", false),
        Engine::Rime => WordRule::new(WordKind::Pinyin, "'", "", false),
        Engine::Kkc => WordRule::new(WordKind::AsciiPhrase, "'-", "-", false),
        Engine::Keyboard => WordRule::NONE,
        Engine::Rules => match language.id {
            "ko" => WordRule::new(WordKind::Letters, "-'", "", false),
            "vi" | "am" | "ti" => WordRule::new(WordKind::AsciiLetters, "", "", false),
            "sat" => WordRule::new(WordKind::AsciiLetters, "~", "~", false),
            "ar" => WordRule::new(WordKind::Letters, "'-", "", true),
            "fa" | "he" => WordRule::new(WordKind::Letters, "'", "", true),
            "ru" => WordRule::new(WordKind::Letters, "'\"", "'\"", false),
            "uk" | "be" => WordRule::new(WordKind::Letters, "'`", "'`", false),
            "el" | "bg" | "ka" => WordRule::new(WordKind::Letters, "'", "'", false),
            "hy" => WordRule::new(WordKind::Letters, "'@", "'", false),
            _ => WordRule::new(WordKind::Letters, "", "", false),
        },
    }
}

/// Exact (ITRANS / IAST) conversion of one word into the language's script.
pub fn exact_indic(language: &LanguageInfo, word: &str) -> Option<String> {
    let script = language.indic_script()?;
    let dravidian = matches!(
        script,
        IndicScript::Tamil | IndicScript::Telugu | IndicScript::Kannada | IndicScript::Malayalam
    );
    let devanagari = brahmic::itrans_to_devanagari_with(word, language.id == "sa", dravidian);
    Some(brahmic::from_devanagari(&devanagari, script))
}

/// Rule-based candidates for one word, best first. For neural languages this
/// is the exact ITRANS reading (used while the model is busy and offered as
/// an alternative). Falls back to the word itself.
pub fn rule_candidates(language: &LanguageInfo, word: &str) -> Vec<String> {
    let mut candidates = match (language.engine, language.id) {
        (Engine::Neural { .. }, _) => exact_indic(language, word).into_iter().collect(),
        (_, "ko") => hangul::candidates(word),
        (_, "vi") => vietnamese::telex(word).into_iter().collect(),
        (_, "am" | "ti") => vec![ethiopic::convert(word)],
        (_, "sat") => vec![olchiki::convert(word)],
        (Engine::Rules, id) => tables::candidates(id, word),
        _ => Vec::new(),
    };
    candidates.retain(|candidate| !candidate.is_empty());
    if candidates.is_empty() {
        candidates.push(word.to_owned());
    }
    candidates
}

fn native_digit_zero(language: &LanguageInfo) -> Option<u32> {
    match (language.id, language.script) {
        ("ar", _) => Some(0x0660),
        ("fa" | "ur" | "ks" | "sd", _) => Some(0x06F0),
        (_, Script::Indic(IndicScript::Devanagari)) => Some(0x0966),
        (_, Script::Indic(IndicScript::Bengali | IndicScript::Assamese)) => Some(0x09E6),
        (_, Script::Indic(IndicScript::Gurmukhi)) => Some(0x0A66),
        (_, Script::Indic(IndicScript::Gujarati)) => Some(0x0AE6),
        (_, Script::Indic(IndicScript::Odia)) => Some(0x0B66),
        (_, Script::Indic(IndicScript::Tamil)) => Some(0x0BE6),
        (_, Script::Indic(IndicScript::Telugu)) => Some(0x0C66),
        (_, Script::Indic(IndicScript::Kannada)) => Some(0x0CE6),
        (_, Script::Indic(IndicScript::Malayalam)) => Some(0x0D66),
        (_, Script::Thai) => Some(0x0E50),
        (_, Script::OlChiki) => Some(0x1C50),
        (_, Script::MeeteiMayek) => Some(0xABF0),
        _ => None,
    }
}

/// Whether the language has native digits that can replace 0–9.
pub fn has_native_digits(language: &LanguageInfo) -> bool {
    native_digit_zero(language).is_some()
}

/// Maps punctuation (and optionally digits) in unconverted text to the
/// conventions of the output script.
pub fn punctuate(language: &LanguageInfo, text: &str, native_digits: bool) -> String {
    let chars: Vec<char> = text.chars().collect();
    let digit_zero = if native_digits {
        native_digit_zero(language)
    } else {
        None
    };
    let mut output = String::with_capacity(text.len());
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        let next = chars.get(index + 1).copied();
        let previous = index.checked_sub(1).map(|position| chars[position]);
        let between_digits = previous.is_some_and(|value| value.is_ascii_digit())
            && next.is_some_and(|value| value.is_ascii_digit());
        let mapped: Option<&str> = match (language.script, ch) {
            (Script::Indic(_), '|') if next == Some('|') => {
                index += 1;
                Some("॥")
            }
            (Script::Indic(_), '|') => Some("।"),
            (Script::OlChiki, '|') if next == Some('|') => {
                index += 1;
                Some("᱿")
            }
            (Script::OlChiki, '|') => Some("᱾"),
            (Script::MeeteiMayek, '|') => Some("꯫"),
            (Script::Han, ',') => Some("，"),
            (Script::Han, '.') if !between_digits => Some("。"),
            (Script::Han, '?') => Some("？"),
            (Script::Han, '!') => Some("！"),
            (Script::Han, ':') if !between_digits => Some("："),
            (Script::Han, ';') => Some("；"),
            (Script::Han, '(') => Some("（"),
            (Script::Han, ')') => Some("）"),
            (Script::Japanese, ',') => Some("、"),
            (Script::Japanese, '.') if !between_digits => Some("。"),
            (Script::Japanese, '?') => Some("？"),
            (Script::Japanese, '!') => Some("！"),
            (Script::Arabic | Script::Nastaliq, ',') => Some("،"),
            (Script::Arabic | Script::Nastaliq, '?') => Some("؟"),
            (Script::Arabic | Script::Nastaliq, ';') => Some("؛"),
            (Script::Nastaliq, '.') if !between_digits && language.id == "ur" => Some("۔"),
            (Script::Ethiopic, '.') if !between_digits => Some("።"),
            (Script::Ethiopic, ',') => Some("፣"),
            (Script::Ethiopic, ';') => Some("፤"),
            (Script::Greek, '?') => Some(";"),
            (Script::Armenian, '.') if !between_digits => Some("։"),
            _ => None,
        };
        match (mapped, digit_zero) {
            (Some(value), _) => output.push_str(value),
            (None, Some(zero)) if ch.is_ascii_digit() => {
                let digit = char::from_u32(zero + (ch as u32 - '0' as u32)).unwrap_or(ch);
                output.push(digit);
            }
            _ => output.push(ch),
        }
        index += 1;
    }
    output
}

/// Converts Pinyin with tone marks to the plain ASCII spelling Rime expects.
pub fn normalize_pinyin_input(input: &str) -> String {
    input
        .nfc()
        .map(|ch| match ch {
            'ā' | 'á' | 'ǎ' | 'à' => 'a',
            'Ā' | 'Á' | 'Ǎ' | 'À' => 'A',
            'ē' | 'é' | 'ě' | 'è' => 'e',
            'Ē' | 'É' | 'Ě' | 'È' => 'E',
            'ī' | 'í' | 'ǐ' | 'ì' => 'i',
            'Ī' | 'Í' | 'Ǐ' | 'Ì' => 'I',
            'ō' | 'ó' | 'ǒ' | 'ò' => 'o',
            'Ō' | 'Ó' | 'Ǒ' | 'Ò' => 'O',
            'ū' | 'ú' | 'ǔ' | 'ù' => 'u',
            'Ū' | 'Ú' | 'Ǔ' | 'Ù' => 'U',
            'ü' | 'ǖ' | 'ǘ' | 'ǚ' | 'ǜ' => 'v',
            'Ü' | 'Ǖ' | 'Ǘ' | 'Ǚ' | 'Ǜ' => 'V',
            'ń' | 'ň' | 'ǹ' => 'n',
            'Ń' | 'Ň' | 'Ǹ' => 'N',
            'ḿ' => 'm',
            'Ḿ' => 'M',
            other => other,
        })
        .collect()
}

/// Converts a whole text with the language's rule-based engine. Neural
/// languages use their exact (ITRANS / IAST) reading; Mandarin, Japanese and
/// keyboard-only languages are returned unchanged because they need a
/// dictionary or direct typing.
pub fn transliterate(input: &str, language_id: &str) -> String {
    let Some(language) = language(language_id) else {
        return input.to_owned();
    };
    if matches!(
        language.engine,
        Engine::Rime | Engine::Kkc | Engine::Keyboard
    ) {
        return input.to_owned();
    }
    let rule = word_rule(language, true);
    let mut output = String::with_capacity(input.len() * 2);
    for token in tokenize::tokenize(input, &rule) {
        match token.kind {
            TokenKind::Word => {
                let word = token.text(input);
                let mut candidates = rule_candidates(language, word);
                output.push_str(&candidates.swap_remove(0));
            }
            TokenKind::Text => output.push_str(&punctuate(language, token.text(input), false)),
            TokenKind::Protected => output.push_str(token.inner(input)),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_consistent() {
        let mut ids = std::collections::HashSet::new();
        for language in LANGUAGES {
            assert!(ids.insert(language.id), "duplicate {}", language.id);
            assert!(!language.name.is_empty() && !language.native.is_empty());
        }
        assert_eq!(language(DEFAULT_LANGUAGE).map(|value| value.id), Some("hi"));
        // All 22 scheduled languages of India are present.
        for id in [
            "as", "bn", "brx", "doi", "gu", "hi", "kn", "ks", "gom", "mai", "ml", "mni", "mr",
            "ne", "or", "pa", "sa", "sat", "sd", "ta", "te", "ur",
        ] {
            assert!(language(id).is_some(), "missing {id}");
        }
    }

    #[test]
    fn hindi_common_words() {
        assert_eq!(transliterate("namaste", "hi"), "नमस्ते");
        assert_eq!(transliterate("shakti", "hi"), "शक्ति");
        assert_eq!(transliterate("bhaarat", "hi"), "भारत");
    }

    #[test]
    fn bengali_common_word() {
        assert_eq!(transliterate("namaskaar", "bn"), "নমস্কার");
    }

    #[test]
    fn preserves_unmatched_text() {
        assert_eq!(transliterate("hello 2026!", "th"), "hello 2026!");
    }

    #[test]
    fn longest_match_wins() {
        assert_eq!(transliterate("shch", "ru"), "щ");
    }

    #[test]
    fn whole_text_rules() {
        assert_eq!(
            transliterate("kyaa haal hai | {OK}", "hi"),
            "क्या हाल है । OK"
        );
        assert_eq!(transliterate("tieengs Vieetj", "vi"), "tiếng Việt");
        assert_eq!(transliterate("annyeonghaseyo", "ko"), "안녕하세요");
        assert_eq!(transliterate("selam.", "am"), "ሰላም።");
    }

    #[test]
    fn punctuation_and_digits() {
        let hindi = language("hi").unwrap();
        assert_eq!(punctuate(hindi, " 2026 ||", true), " २०२६ ॥");
        let chinese = language("zh").unwrap();
        assert_eq!(punctuate(chinese, ", 3.14.", false), "， 3.14。");
        let arabic = language("ar").unwrap();
        assert_eq!(punctuate(arabic, "? 12", true), "؟ ١٢");
    }

    #[test]
    fn tone_marked_pinyin_is_normalized_without_losing_separators() {
        assert_eq!(normalize_pinyin_input("nǐ hǎo! lǜ 2026"), "ni hao! lv 2026");
        assert_eq!(normalize_pinyin_input("nu\u{0308}\u{030c}"), "nv");
    }
}
