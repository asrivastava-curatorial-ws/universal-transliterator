//! Amharic and Tigrinya (Ge'ez / Ethiopic script) phonetic typing.
//!
//! A consonant followed by a vowel produces the matching syllable:
//! `e` 1st order (ä), `u` 2nd, `i` 3rd, `a` 4th, `ie`/`E` 5th, nothing 6th,
//! `o` 7th, `wa` the labialized form. Standalone vowels use the glottal
//! series (`a` አ, `u` ኡ, `i` ኢ, `aa` ኣ, `ie` ኤ, `e` እ, `o` ኦ). Doubled
//! consonants are written once because Ge'ez does not mark gemination.
//! Example: `selam` → ሰላም, `amarnya` → አማርኛ.

const CONSONANTS: &[(&str, u32)] = &[
    ("h", 0x1200),
    ("l", 0x1208),
    ("H", 0x1210),
    ("m", 0x1218),
    ("r", 0x1228),
    ("s", 0x1230),
    ("sh", 0x1238),
    ("q", 0x1240),
    ("Q", 0x1250),
    ("b", 0x1260),
    ("v", 0x1268),
    ("t", 0x1270),
    ("ch", 0x1278),
    ("c", 0x1278),
    ("x", 0x1280),
    ("n", 0x1290),
    ("ny", 0x1298),
    ("N", 0x1298),
    ("k", 0x12A8),
    ("kh", 0x12B8),
    ("K", 0x12B8),
    ("w", 0x12C8),
    ("z", 0x12D8),
    ("zh", 0x12E0),
    ("Z", 0x12E0),
    ("y", 0x12E8),
    ("d", 0x12F0),
    ("j", 0x1300),
    ("g", 0x1308),
    ("T", 0x1320),
    ("C", 0x1328),
    ("P", 0x1330),
    ("ts", 0x1338),
    ("S", 0x1338),
    ("f", 0x1348),
    ("p", 0x1350),
];

/// Vowel → order offset within a consonant row.
const VOWELS: &[(&str, u32)] = &[
    ("e", 0),
    ("u", 1),
    ("i", 2),
    ("a", 3),
    ("ie", 4),
    ("E", 4),
    ("o", 6),
];

const GLOTTAL: u32 = 0x12A0;

/// Labialized "-wa" syllables that do not sit at offset 7 of their row.
fn labialized(base: u32) -> Option<u32> {
    match base {
        0x1240 => Some(0x124B),
        0x1250 => Some(0x125B),
        0x1280 => Some(0x128B),
        0x12A8 => Some(0x12B3),
        0x12B8 => Some(0x12C3),
        0x1308 => Some(0x1313),
        0x1200 | 0x12C8 | 0x12D0 | 0x12E8 | 0x1340 => None,
        other => Some(other + 7),
    }
}

fn longest<'a>(rest: &str, table: &'a [(&'a str, u32)]) -> Option<(&'a str, u32)> {
    table
        .iter()
        .filter(|(token, _)| rest.starts_with(token))
        .max_by_key(|(token, _)| token.len())
        .map(|(token, value)| (*token, *value))
}

fn push(output: &mut String, code: u32) {
    if let Some(ch) = char::from_u32(code) {
        output.push(ch);
    }
}

/// Converts one romanized word to Ethiopic script.
pub fn convert(word: &str) -> String {
    let mut output = String::with_capacity(word.len() * 3);
    let mut offset = 0;
    while offset < word.len() {
        let rest = &word[offset..];
        if let Some((token, base)) = longest(rest, CONSONANTS) {
            offset += token.len();
            // Gemination: "mm" is written as one syllable.
            if word[offset..].starts_with(token)
                && longest(&word[offset..], CONSONANTS).map(|(next, _)| next) == Some(token)
            {
                continue;
            }
            let after = &word[offset..];
            if after.starts_with("wa") {
                if let Some(code) = labialized(base) {
                    push(&mut output, code);
                    offset += 2;
                    continue;
                }
            }
            if let Some((vowel, order)) = longest(after, VOWELS) {
                push(&mut output, base + order);
                offset += vowel.len();
            } else {
                push(&mut output, base + 5);
            }
            continue;
        }
        // Standalone vowels use the glottal-stop series.
        let standalone = [
            ("aa", 3),
            ("ie", 4),
            ("E", 4),
            ("a", 0),
            ("u", 1),
            ("i", 2),
            ("e", 5),
            ("I", 5),
            ("o", 6),
        ];
        if let Some((token, order)) = standalone
            .iter()
            .find(|(token, _)| rest.starts_with(token))
            .copied()
        {
            push(&mut output, GLOTTAL + order);
            offset += token.len();
            continue;
        }
        let ch = rest.chars().next().unwrap_or(' ');
        output.push(ch);
        offset += ch.len_utf8();
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amharic_words() {
        assert_eq!(convert("selam"), "ሰላም");
        assert_eq!(convert("amarnya"), "አማርኛ");
        assert_eq!(convert("amarinya"), "አማሪኛ");
        assert_eq!(convert("addis"), "አዲስ");
        assert_eq!(convert("abeba"), "አበባ");
        assert_eq!(convert("ityoPya"), "ኢትዮጵያ");
        assert_eq!(convert("gwa"), "ጓ");
        assert_eq!(convert("lwa"), "ሏ");
    }
}
