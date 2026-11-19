//! Standard spellings for very common words whose AI reading is usually
//! wrong. IndicXlit converts each word without context, so short chat words
//! are often read as English acronyms (`aap` → एएपी, `dil` → डीआईएल) or lose
//! their nasalisation (`nahi` → नही). These defaults apply only in AI mode;
//! the model's own candidates stay available as alternatives, and a choice
//! the user makes always wins.
//!
//! Keys are lowercase ASCII and each table is sorted by key.

const HINDI: &[(&str, &str)] = &[
    ("aap", "आप"),
    ("ab", "अब"),
    ("abhi", "अभी"),
    ("acha", "अच्छा"),
    ("achha", "अच्छा"),
    ("aisa", "ऐसा"),
    ("aise", "ऐसे"),
    ("ap", "आप"),
    ("bahot", "बहुत"),
    ("bas", "बस"),
    ("bohot", "बहुत"),
    ("chal", "चल"),
    ("chalo", "चलो"),
    ("de", "दे"),
    ("dil", "दिल"),
    ("do", "दो"),
    ("dono", "दोनों"),
    ("gya", "गया"),
    ("haa", "हाँ"),
    ("han", "हाँ"),
    ("hu", "हूँ"),
    ("hume", "हमें"),
    ("humein", "हमें"),
    ("hun", "हूँ"),
    ("inka", "इनका"),
    ("iska", "इसका"),
    ("isse", "इससे"),
    ("jate", "जाते"),
    ("kahan", "कहाँ"),
    ("kam", "कम"),
    ("kaun", "कौन"),
    ("ki", "की"),
    ("kon", "कौन"),
    ("kuch", "कुछ"),
    ("kyu", "क्यों"),
    ("kyuki", "क्योंकि"),
    ("kyun", "क्यों"),
    ("kyunki", "क्योंकि"),
    ("log", "लोग"),
    ("mai", "मैं"),
    ("main", "मैं"),
    ("me", "में"),
    ("muje", "मुझे"),
    ("nahi", "नहीं"),
    ("nhi", "नहीं"),
    ("or", "और"),
    ("pata", "पता"),
    ("pta", "पता"),
    ("sahi", "सही"),
    ("sirf", "सिर्फ"),
    ("subah", "सुबह"),
    ("taaki", "ताकि"),
    ("teen", "तीन"),
    ("thik", "ठीक"),
    ("to", "तो"),
    ("toh", "तो"),
    ("tu", "तू"),
    ("tum", "तुम"),
    ("tumhare", "तुम्हारे"),
    ("tumhe", "तुम्हें"),
    ("tuu", "तू"),
    ("unhe", "उन्हें"),
    ("usse", "उससे"),
    ("voh", "वह"),
    ("wahan", "वहाँ"),
    ("wahi", "वही"),
    ("warna", "वरना"),
    ("woh", "वह"),
    ("yahan", "यहाँ"),
    ("yahi", "यही"),
    ("yeh", "यह"),
];

fn table(language: &str) -> &'static [(&'static str, &'static str)] {
    match language {
        "hi" => HINDI,
        _ => &[],
    }
}

/// The standard spelling of a very common word, if the language has one.
pub fn common_spelling(language: &str, key: &str) -> Option<&'static str> {
    let table = table(language);
    table
        .binary_search_by(|(word, _)| (*word).cmp(key))
        .ok()
        .map(|index| table[index].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_are_sorted_and_unique() {
        for pair in HINDI.windows(2) {
            assert!(
                pair[0].0 < pair[1].0,
                "{} must sort before {}",
                pair[0].0,
                pair[1].0
            );
        }
    }

    #[test]
    fn looks_up_hindi_words() {
        assert_eq!(common_spelling("hi", "main"), Some("मैं"));
        assert_eq!(common_spelling("hi", "nahi"), Some("नहीं"));
        assert_eq!(common_spelling("hi", "woh"), Some("वह"));
        assert_eq!(common_spelling("hi", "namaste"), None);
        assert_eq!(common_spelling("mr", "main"), None);
    }
}
