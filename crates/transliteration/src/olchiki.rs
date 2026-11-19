//! Santali in Ol Chiki script.
//!
//! Ol Chiki is a true alphabet (no inherent vowel), so each Roman letter maps
//! to one letter. Aspirates take the "h" sign: `kh` → ᱠᱷ. Following Indian
//! typing habits, `c`/`ch` is ᱪ and `chh` is ᱪᱷ. `o` is ᱚ (the common open
//! o) and `O` is ᱳ. Retroflex letters use capitals or doubling: `T`/`tt` ᱴ,
//! `D`/`dd` ᱰ, `N`/`nn` ᱬ, `R`/`rr` ᱲ. `~` adds nasalization (ᱸ).
//! Example: `santaRi` → ᱥᱟᱱᱛᱟᱲᱤ, `ol chiki` → ᱚᱞ ᱪᱤᱠᱤ.

const TABLE: &[(&str, &str)] = &[
    ("chh", "ᱪᱷ"),
    ("kh", "ᱠᱷ"),
    ("gh", "ᱜᱷ"),
    ("jh", "ᱡᱷ"),
    ("th", "ᱛᱷ"),
    ("dh", "ᱫᱷ"),
    ("ph", "ᱯᱷ"),
    ("bh", "ᱵᱷ"),
    ("Th", "ᱴᱷ"),
    ("Dh", "ᱰᱷ"),
    ("ng", "ᱝ"),
    ("ny", "ᱧ"),
    ("ch", "ᱪ"),
    ("c", "ᱪ"),
    ("k", "ᱠ"),
    ("g", "ᱜ"),
    ("j", "ᱡ"),
    ("tt", "ᱴ"),
    ("T", "ᱴ"),
    ("t", "ᱛ"),
    ("dd", "ᱰ"),
    ("D", "ᱰ"),
    ("d", "ᱫ"),
    ("nn", "ᱬ"),
    ("N", "ᱬ"),
    ("n", "ᱱ"),
    ("p", "ᱯ"),
    ("b", "ᱵ"),
    ("m", "ᱢ"),
    ("y", "ᱭ"),
    ("rr", "ᱲ"),
    ("R", "ᱲ"),
    ("r", "ᱨ"),
    ("l", "ᱞ"),
    ("w", "ᱣ"),
    ("v", "ᱣ"),
    ("s", "ᱥ"),
    ("h", "ᱦ"),
    ("aa", "ᱟ"),
    ("a", "ᱟ"),
    ("i", "ᱤ"),
    ("ee", "ᱤ"),
    ("u", "ᱩ"),
    ("oo", "ᱩ"),
    ("e", "ᱮ"),
    ("O", "ᱳ"),
    ("o", "ᱚ"),
    ("~", "ᱸ"),
];

/// Converts one romanized Santali word to Ol Chiki.
pub fn convert(word: &str) -> String {
    let mut output = String::with_capacity(word.len() * 3);
    let mut offset = 0;
    while offset < word.len() {
        let rest = &word[offset..];
        if let Some((latin, native)) = TABLE
            .iter()
            .filter(|(latin, _)| rest.starts_with(latin))
            .max_by_key(|(latin, _)| latin.len())
        {
            output.push_str(native);
            offset += latin.len();
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
    fn santali_words() {
        assert_eq!(convert("santaRi"), "ᱥᱟᱱᱛᱟᱲᱤ");
        assert_eq!(convert("ol"), "ᱚᱞ");
        assert_eq!(convert("chiki"), "ᱪᱤᱠᱤ");
        assert_eq!(convert("johar"), "ᱡᱚᱦᱟᱨ");
    }
}
