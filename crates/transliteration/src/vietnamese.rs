//! Vietnamese Telex typing (`tieengs Vieetj` → `tiếng Việt`).
//!
//! Each word is processed like a Telex keyboard would: `aa ee oo` add a
//! circumflex, `aw ow uw` add a breve or horn, `dd` makes `đ`, and the keys
//! `s f r x j` set the tone (`z` clears it). Typing a modifier twice undoes
//! it. Words that do not form a valid Vietnamese syllable are left exactly as
//! typed, so English words survive.

use unicode_normalization::UnicodeNormalization;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tone {
    None,
    Acute,
    Grave,
    Hook,
    Tilde,
    Dot,
}

impl Tone {
    fn mark(self) -> Option<char> {
        match self {
            Tone::None => None,
            Tone::Acute => Some('\u{301}'),
            Tone::Grave => Some('\u{300}'),
            Tone::Hook => Some('\u{309}'),
            Tone::Tilde => Some('\u{303}'),
            Tone::Dot => Some('\u{323}'),
        }
    }

    fn from_key(key: char) -> Option<Tone> {
        match key {
            's' => Some(Tone::Acute),
            'f' => Some(Tone::Grave),
            'r' => Some(Tone::Hook),
            'x' => Some(Tone::Tilde),
            'j' => Some(Tone::Dot),
            _ => None,
        }
    }
}

fn is_vowel(ch: char) -> bool {
    matches!(
        ch,
        'a' | 'ă' | 'â' | 'e' | 'ê' | 'i' | 'o' | 'ô' | 'ơ' | 'u' | 'ư' | 'y'
    )
}

fn has_diacritic(ch: char) -> bool {
    matches!(ch, 'ă' | 'â' | 'ê' | 'ô' | 'ơ' | 'ư')
}

const INITIALS: &[&str] = &[
    "", "b", "c", "ch", "d", "đ", "g", "gh", "gi", "h", "k", "kh", "l", "m", "n", "ng", "ngh",
    "nh", "p", "ph", "qu", "r", "s", "t", "th", "tr", "v", "x",
];
const FINALS: &[&str] = &["", "c", "ch", "m", "n", "ng", "nh", "p", "t"];
const NUCLEI: &[&str] = &[
    "a", "ă", "â", "e", "ê", "i", "o", "ô", "ơ", "u", "ư", "y", "ai", "ao", "au", "ay", "âu", "ây",
    "eo", "êu", "ia", "iê", "iu", "oa", "oă", "oe", "oi", "ôi", "ơi", "oo", "ua", "uâ", "uê", "ui",
    "uô", "uơ", "ưa", "ưi", "ươ", "ưu", "uy", "yê", "iêu", "yêu", "oai", "oay", "oeo", "uây",
    "uôi", "ươi", "ươu", "uya", "uyê", "uyu",
];

/// Splits lowercase letters into (initial, nucleus, final) if they form a
/// valid Vietnamese syllable.
fn syllable(letters: &[char]) -> Option<(usize, usize)> {
    let text: String = letters.iter().collect();
    // Longest valid initial whose remainder starts with a vowel.
    let mut initial_len = None;
    for candidate in INITIALS {
        let count = candidate.chars().count();
        if text.starts_with(candidate)
            && letters.get(count).copied().is_some_and(is_vowel)
            && !initial_len.is_some_and(|current| count <= current)
        {
            initial_len = Some(count);
        }
    }
    // "gi" / "qu" only count as initials when a vowel follows, so a bare
    // "gì" keeps its i as the nucleus.
    let start = initial_len?;
    let mut end = start;
    while end < letters.len() && is_vowel(letters[end]) {
        end += 1;
    }
    let nucleus: String = letters[start..end].iter().collect();
    let coda: String = letters[end..].iter().collect();
    (NUCLEI.contains(&nucleus.as_str()) && FINALS.contains(&coda.as_str())).then_some((start, end))
}

fn tone_position(letters: &[char], start: usize, end: usize) -> usize {
    let nucleus = &letters[start..end];
    if let Some(position) = nucleus.iter().position(|ch| *ch == 'ơ') {
        return start + position;
    }
    if let Some(position) = nucleus.iter().position(|ch| has_diacritic(*ch)) {
        return start + position;
    }
    let has_final = end < letters.len();
    match nucleus.len() {
        0 | 1 => start,
        2 if has_final => start + 1,
        2 => start,
        _ => start + 1,
    }
}

/// Converts one Telex-typed word. Returns `None` when the result would not be
/// a valid Vietnamese syllable.
pub fn telex(word: &str) -> Option<String> {
    let mut letters: Vec<char> = Vec::with_capacity(word.len());
    let mut upper: Vec<bool> = Vec::with_capacity(word.len());
    let mut tone = Tone::None;
    let mut changed = false;
    for raw in word.chars() {
        if !raw.is_ascii_alphabetic() {
            return None;
        }
        let key = raw.to_ascii_lowercase();
        let is_upper = raw.is_ascii_uppercase();
        let has_vowel = letters.iter().any(|ch| is_vowel(*ch));
        match key {
            'a' | 'e' | 'o' => {
                let hat = match key {
                    'a' => 'â',
                    'e' => 'ê',
                    _ => 'ô',
                };
                if let Some(position) = letters.iter().rposition(|ch| *ch == key || *ch == hat) {
                    let in_nucleus = letters[position + 1..].iter().all(|ch| !is_vowel(*ch))
                        || letters[position + 1..].iter().all(|ch| is_vowel(*ch));
                    if in_nucleus && letters[position] == key {
                        letters[position] = hat;
                        changed = true;
                        continue;
                    }
                    if in_nucleus && letters[position] == hat {
                        letters[position] = key;
                        letters.push(key);
                        upper.push(is_upper);
                        continue;
                    }
                }
                letters.push(key);
                upper.push(is_upper);
            }
            'w' => {
                let len = letters.len();
                let uo = (0..len.saturating_sub(1))
                    .rev()
                    .find(|&index| letters[index] == 'u' && letters[index + 1] == 'o');
                if let Some(index) = uo {
                    letters[index] = 'ư';
                    letters[index + 1] = 'ơ';
                    changed = true;
                } else if let Some(index) = letters
                    .iter()
                    .rposition(|ch| matches!(ch, 'a' | 'o' | 'u' | 'ă' | 'ơ' | 'ư'))
                {
                    match letters[index] {
                        'a' => letters[index] = 'ă',
                        'o' => letters[index] = 'ơ',
                        'u' => letters[index] = 'ư',
                        undone => {
                            letters[index] = match undone {
                                'ă' => 'a',
                                'ơ' => 'o',
                                _ => 'u',
                            };
                            letters.push('w');
                            upper.push(is_upper);
                            continue;
                        }
                    }
                    changed = true;
                } else {
                    letters.push('ư');
                    upper.push(is_upper);
                    changed = true;
                }
            }
            'd' if letters.first() == Some(&'d') && letters.len() == 1 => {
                letters[0] = 'đ';
                changed = true;
            }
            'd' if letters.first() == Some(&'đ') && letters.len() == 1 => {
                letters[0] = 'd';
                letters.push('d');
                upper.push(is_upper);
            }
            'z' if tone != Tone::None => {
                tone = Tone::None;
                changed = true;
            }
            _ if has_vowel && Tone::from_key(key).is_some() => {
                let requested = Tone::from_key(key).unwrap_or(Tone::None);
                if tone == requested {
                    tone = Tone::None;
                    letters.push(key);
                    upper.push(is_upper);
                } else {
                    tone = requested;
                    changed = true;
                }
            }
            _ => {
                letters.push(key);
                upper.push(is_upper);
            }
        }
    }
    let (start, end) = syllable(&letters)?;
    if !changed && tone == Tone::None {
        return Some(word.to_owned());
    }
    let tone_at = tone_position(&letters, start, end);
    let mut output = String::with_capacity(word.len() * 2);
    for (index, ch) in letters.iter().enumerate() {
        let mut piece = String::new();
        piece.push(*ch);
        if index == tone_at {
            if let Some(mark) = tone.mark() {
                piece.push(mark);
            }
        }
        let piece: String = piece.nfc().collect();
        if upper.get(index).copied().unwrap_or(false) {
            output.extend(piece.chars().flat_map(char::to_uppercase));
        } else {
            output.push_str(&piece);
        }
    }
    Some(output.nfc().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(word: &str) -> String {
        telex(word).unwrap_or_else(|| word.to_owned())
    }

    #[test]
    fn basic_words() {
        assert_eq!(t("tieengs"), "tiếng");
        assert_eq!(t("Vieetj"), "Việt");
        assert_eq!(t("xin"), "xin");
        assert_eq!(t("chaof"), "chào");
        assert_eq!(t("ddaau"), "đâu");
        assert_eq!(t("nguwowif"), "người");
        assert_eq!(t("hoaf"), "hòa");
        assert_eq!(t("toans"), "toán");
        assert_eq!(t("quas"), "quá");
        assert_eq!(t("gias"), "giá");
        assert_eq!(t("Haf"), "Hà");
        assert_eq!(t("Nooij"), "Nội");
        assert_eq!(t("cuwr"), "cử");
        assert_eq!(t("khuyeen"), "khuyên");
    }

    #[test]
    fn double_key_escapes_to_the_typed_word() {
        // "ass" undoes the tone and is not a Vietnamese syllable, so the word
        // is kept exactly as typed.
        assert_eq!(telex("ass"), None);
        assert_eq!(t("Vieejt"), "Việt");
        assert_eq!(t("dduwowngf"), "đường");
    }

    #[test]
    fn english_words_are_left_alone() {
        assert_eq!(telex("browser"), None);
        assert_eq!(telex("class"), None);
        assert_eq!(t("hello"), "hello");
    }
}
