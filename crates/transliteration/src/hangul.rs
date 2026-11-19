//! Korean: Revised-Romanization-style typing → Hangul syllable blocks.
//!
//! Romanized Korean is ambiguous at syllable boundaries ("hangeul" could be
//! 한글 or 항을), so [`candidates`] returns every plausible syllabification,
//! best first. A hyphen or apostrophe forces a boundary: `jung-ang` → 중앙.

const INITIALS: &[(&str, u32)] = &[
    ("g", 0),
    ("kk", 1),
    ("gg", 1),
    ("n", 2),
    ("d", 3),
    ("tt", 4),
    ("dd", 4),
    ("r", 5),
    ("l", 5),
    ("m", 6),
    ("b", 7),
    ("v", 7),
    ("pp", 8),
    ("bb", 8),
    ("s", 9),
    ("ss", 10),
    ("j", 12),
    ("z", 12),
    ("jj", 13),
    ("ch", 14),
    ("c", 14),
    ("k", 15),
    ("t", 16),
    ("p", 17),
    ("f", 17),
    ("h", 18),
];
const SILENT_INITIAL: u32 = 11;

const VOWELS: &[(&str, u32)] = &[
    ("a", 0),
    ("ae", 1),
    ("ya", 2),
    ("yae", 3),
    ("eo", 4),
    ("e", 5),
    ("yeo", 6),
    ("ye", 7),
    ("o", 8),
    ("wa", 9),
    ("wae", 10),
    ("oe", 11),
    ("yo", 12),
    ("u", 13),
    ("oo", 13),
    ("wo", 14),
    ("weo", 14),
    ("we", 15),
    ("wi", 16),
    ("yu", 17),
    ("eu", 18),
    ("ui", 19),
    ("eui", 19),
    ("i", 20),
    ("ee", 20),
];

const FINALS: &[(&str, u32)] = &[
    ("g", 1),
    ("k", 1),
    ("kk", 2),
    ("gs", 3),
    ("ks", 3),
    ("n", 4),
    ("nj", 5),
    ("nh", 6),
    ("d", 7),
    ("l", 8),
    ("r", 8),
    ("lg", 9),
    ("lk", 9),
    ("lm", 10),
    ("lb", 11),
    ("ls", 12),
    ("lt", 13),
    ("lp", 14),
    ("lh", 15),
    ("m", 16),
    ("b", 17),
    ("p", 17),
    ("bs", 18),
    ("ps", 18),
    ("s", 19),
    ("t", 19),
    ("ss", 20),
    ("ng", 21),
    ("j", 22),
    ("ch", 23),
    ("c", 23),
    ("h", 27),
];

const MAX_CANDIDATES: usize = 8;

fn lookup(table: &[(&str, u32)], key: &str) -> Option<u32> {
    table
        .iter()
        .find(|(token, _)| *token == key)
        .map(|(_, value)| *value)
}

fn initial(cluster: &str) -> Option<u32> {
    if cluster.is_empty() {
        Some(SILENT_INITIAL)
    } else {
        lookup(INITIALS, cluster)
    }
}

fn final_consonant(cluster: &str) -> Option<u32> {
    if cluster.is_empty() {
        Some(0)
    } else {
        lookup(FINALS, cluster)
    }
}

fn compose(initial: u32, vowel: u32, final_index: u32) -> char {
    char::from_u32(0xAC00 + (initial * 21 + vowel) * 28 + final_index).unwrap_or('\u{FFFD}')
}

/// One way to split the consonants between two vowels.
#[derive(Clone, Copy)]
struct Split {
    final_index: u32,
    initial: u32,
    score: i32,
}

fn splits(cluster: &str) -> Vec<Split> {
    let mut result = Vec::new();
    for cut in 0..=cluster.len() {
        let (coda, onset) = cluster.split_at(cut);
        let (Some(final_index), Some(initial_index)) = (final_consonant(coda), initial(onset))
        else {
            continue;
        };
        let mut score =
            if onset.is_empty() { 0 } else { 20 } + if coda.is_empty() { 10 } else { 0 };
        // Past-tense "-ss-eo" (있어, 했어) is far more common than initial ㅆ.
        if cluster == "ss" && coda == "ss" {
            score += 40;
        }
        // Prefer short onsets on ties ("ngg" → ㅇ + ㄱ rather than ㄴ + ㄲ).
        score -= onset.len() as i32;
        result.push(Split {
            final_index,
            initial: initial_index,
            score,
        });
    }
    result.sort_by_key(|split| std::cmp::Reverse(split.score));
    result
}

/// Splits a lowercase segment into (consonant cluster, vowel) pairs plus the
/// trailing consonant cluster. Returns `None` for anything that is not
/// romanized Korean.
fn parse(segment: &str) -> Option<(Vec<(String, u32)>, String)> {
    let mut pairs = Vec::new();
    let mut cluster = String::new();
    let mut offset = 0;
    while offset < segment.len() {
        let rest = &segment[offset..];
        if let Some((token, vowel)) = VOWELS
            .iter()
            .filter(|(token, _)| rest.starts_with(token))
            .max_by_key(|(token, _)| token.len())
        {
            pairs.push((std::mem::take(&mut cluster), *vowel));
            offset += token.len();
            continue;
        }
        let ch = rest.chars().next()?;
        if !matches!(
            ch,
            'g' | 'k'
                | 'n'
                | 'd'
                | 't'
                | 'r'
                | 'l'
                | 'm'
                | 'b'
                | 'p'
                | 's'
                | 'j'
                | 'c'
                | 'h'
                | 'f'
                | 'v'
                | 'z'
        ) {
            return None;
        }
        cluster.push(ch);
        offset += ch.len_utf8();
    }
    if pairs.is_empty() {
        return None;
    }
    Some((pairs, cluster))
}

fn segment_candidates(segment: &str) -> Option<Vec<(String, i32)>> {
    let (pairs, trailing) = parse(segment)?;
    let first_initial = initial(&pairs[0].0)?;
    let last_final = final_consonant(&trailing)?;
    // Choices for every cluster between two vowels.
    let mut options: Vec<Vec<Split>> = Vec::new();
    for (cluster, _) in pairs.iter().skip(1) {
        let choices = splits(cluster);
        if choices.is_empty() {
            return None;
        }
        options.push(choices);
    }
    let mut results: Vec<(Vec<usize>, i32)> = vec![(Vec::new(), 0)];
    for choices in &options {
        let mut next = Vec::new();
        for (path, score) in &results {
            for (index, choice) in choices.iter().enumerate().take(3) {
                let mut path = path.clone();
                path.push(index);
                next.push((path, score + choice.score));
            }
        }
        next.sort_by_key(|entry| std::cmp::Reverse(entry.1));
        next.truncate(MAX_CANDIDATES * 2);
        results = next;
    }
    let mut output = Vec::new();
    for (path, score) in results {
        let mut text = String::new();
        for (position, (_, vowel)) in pairs.iter().enumerate() {
            let onset = if position == 0 {
                first_initial
            } else {
                options[position - 1][path[position - 1]].initial
            };
            let coda = if position + 1 == pairs.len() {
                last_final
            } else {
                options[position][path[position]].final_index
            };
            text.push(compose(onset, *vowel, coda));
        }
        output.push((text, score));
    }
    Some(output)
}

/// Returns Hangul spellings for one romanized word, most likely first, or an
/// empty list if the word is not romanized Korean.
pub fn candidates(word: &str) -> Vec<String> {
    let lower = word.to_lowercase();
    let segments: Vec<&str> = lower
        .split(&['-', '\''][..])
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.is_empty() {
        return Vec::new();
    }
    let mut combined: Vec<(String, i32)> = vec![(String::new(), 0)];
    for segment in segments {
        let Some(options) = segment_candidates(segment) else {
            return Vec::new();
        };
        let mut next = Vec::new();
        for (prefix, score) in &combined {
            for (text, extra) in options.iter().take(4) {
                next.push((format!("{prefix}{text}"), score + extra));
            }
        }
        next.sort_by_key(|entry| std::cmp::Reverse(entry.1));
        next.truncate(MAX_CANDIDATES);
        combined = next;
    }
    let mut unique = Vec::new();
    for (text, _) in combined {
        if !unique.contains(&text) {
            unique.push(text);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first(word: &str) -> String {
        candidates(word).into_iter().next().unwrap_or_default()
    }

    #[test]
    fn common_words() {
        assert_eq!(first("annyeonghaseyo"), "안녕하세요");
        assert_eq!(first("saranghae"), "사랑해");
        assert_eq!(first("hangeul"), "한글");
        assert_eq!(first("hanguk"), "한국");
        assert_eq!(first("seoul"), "서울");
        assert_eq!(first("kimchi"), "킴치");
        assert_eq!(first("isseoyo"), "있어요");
        assert_eq!(first("akka"), "아까");
        assert_eq!(first("hoesa"), "회사");
        assert_eq!(first("gwa"), "과");
    }

    #[test]
    fn separators_force_boundaries() {
        assert_eq!(first("jung-ang"), "중앙");
        assert_eq!(first("yeong'eo"), "영어");
    }

    #[test]
    fn alternatives_are_offered() {
        let options = candidates("yeongeo");
        assert!(options.contains(&"영어".to_string()), "{options:?}");
    }

    #[test]
    fn non_korean_is_rejected() {
        assert!(candidates("xyz").is_empty());
        assert!(candidates("strength").is_empty());
    }
}
