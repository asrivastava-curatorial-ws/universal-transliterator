#![forbid(unsafe_code)]
//! Text diffing and three-way merging for the two-pane editor.
//!
//! The output pane is regenerated from the Roman pane, but people also edit
//! the output directly. [`merge3`] applies the change between the previous and
//! the new automatic output to the text the person actually has, so manual
//! edits elsewhere survive further typing. When both sides changed the same
//! place, the new automatic text wins for that place only.

use std::ops::Range;

/// One changed region: `base[base]` was replaced by `other[other]`
/// (character indices).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hunk {
    pub base: Range<usize>,
    pub other: Range<usize>,
}

/// Edits beyond this many characters fall back to one coarse hunk, which
/// keeps diffing fast for large documents.
const MAX_EDIT_DISTANCE: usize = 600;

fn common_prefix(a: &[char], b: &[char]) -> usize {
    a.iter().zip(b).take_while(|(x, y)| x == y).count()
}

fn common_suffix(a: &[char], b: &[char]) -> usize {
    a.iter()
        .rev()
        .zip(b.iter().rev())
        .take_while(|(x, y)| x == y)
        .count()
}

/// Character-level diff (Myers' O(ND) algorithm) as a list of hunks in
/// ascending order.
pub fn diff(base: &[char], other: &[char]) -> Vec<Hunk> {
    let prefix = common_prefix(base, other);
    let suffix = common_suffix(&base[prefix..], &other[prefix..]);
    let a = &base[prefix..base.len() - suffix];
    let b = &other[prefix..other.len() - suffix];
    if a.is_empty() && b.is_empty() {
        return Vec::new();
    }
    let shift = |hunk: Hunk| Hunk {
        base: hunk.base.start + prefix..hunk.base.end + prefix,
        other: hunk.other.start + prefix..hunk.other.end + prefix,
    };
    let coarse = Hunk {
        base: 0..a.len(),
        other: 0..b.len(),
    };
    if a.is_empty() || b.is_empty() {
        return vec![shift(coarse)];
    }
    match myers(a, b, MAX_EDIT_DISTANCE) {
        Some(hunks) => hunks.into_iter().map(shift).collect(),
        None => vec![shift(coarse)],
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Step {
    Equal,
    Delete,
    Insert,
}

fn myers(a: &[char], b: &[char], limit: usize) -> Option<Vec<Hunk>> {
    let n = a.len() as isize;
    let m = b.len() as isize;
    let max = (a.len() + b.len()).min(limit) as isize;
    let offset = max + 1;
    let mut v = vec![0isize; (2 * max + 3) as usize];
    let mut trace: Vec<Vec<isize>> = Vec::new();
    let mut found = false;
    'outer: for d in 0..=max {
        trace.push(v.clone());
        let mut k = -d;
        while k <= d {
            let index = (k + offset) as usize;
            let mut x = if k == -d || (k != d && v[index - 1] < v[index + 1]) {
                v[index + 1]
            } else {
                v[index - 1] + 1
            };
            let mut y = x - k;
            while x < n && y < m && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }
            v[index] = x;
            if x >= n && y >= m {
                found = true;
                break 'outer;
            }
            k += 2;
        }
    }
    if !found {
        return None;
    }
    // Walk back through the trace to recover the edit script.
    let mut steps = Vec::with_capacity((n + m) as usize);
    let (mut x, mut y) = (n, m);
    for (d, v) in trace.iter().enumerate().rev() {
        let d = d as isize;
        let k = x - y;
        let previous_k =
            if k == -d || (k != d && v[(k - 1 + offset) as usize] < v[(k + 1 + offset) as usize]) {
                k + 1
            } else {
                k - 1
            };
        let previous_x = v[(previous_k + offset) as usize];
        let previous_y = previous_x - previous_k;
        while x > previous_x && y > previous_y {
            steps.push(Step::Equal);
            x -= 1;
            y -= 1;
        }
        if d > 0 {
            steps.push(if x == previous_x {
                Step::Insert
            } else {
                Step::Delete
            });
        }
        x = previous_x;
        y = previous_y;
    }
    steps.reverse();
    let mut hunks: Vec<Hunk> = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    for step in steps {
        match step {
            Step::Equal => {
                i += 1;
                j += 1;
            }
            Step::Delete | Step::Insert => {
                let extend = hunks
                    .last()
                    .is_some_and(|last| last.base.end == i && last.other.end == j);
                if !extend {
                    hunks.push(Hunk {
                        base: i..i,
                        other: j..j,
                    });
                }
                let last = hunks.last_mut().expect("a hunk was just ensured");
                if step == Step::Delete {
                    i += 1;
                    last.base.end = i;
                } else {
                    j += 1;
                    last.other.end = j;
                }
            }
        }
    }
    Some(hunks)
}

/// Whether a character belongs to a word for merge purposes. Han characters
/// and kana are treated as words of their own because those scripts do not
/// separate words with spaces.
fn is_word_char(ch: char) -> bool {
    if ch.is_whitespace() || ch.is_ascii_punctuation() {
        return false;
    }
    let code = ch as u32;
    let cjk = matches!(
        code,
        0x3000..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF | 0x20000..=0x2FFFF
    );
    let punctuation = matches!(
        ch,
        '।' | '॥'
            | '؟'
            | '،'
            | '؛'
            | '۔'
            | '።'
            | '፣'
            | '፤'
            | '᱾'
            | '᱿'
            | '«'
            | '»'
            | '“'
            | '”'
            | '‘'
            | '’'
            | '—'
            | '–'
            | '…'
            | '·'
    );
    !cjk && !punctuation
}

/// Widens hunks that cut through a word so that they cover the whole word.
/// Conflicts are then detected per word: a manual fix inside a word is
/// replaced if the same word is retyped, instead of being spliced into it.
fn widen_to_words(hunks: Vec<Hunk>, base: &[char], other: &[char]) -> Vec<Hunk> {
    let mut widened: Vec<Hunk> = Vec::with_capacity(hunks.len());
    let count = hunks.len();
    for index in 0..count {
        let mut hunk = hunks[index].clone();
        let lower = widened.last().map_or(0, |previous| previous.base.end);
        let upper = hunks
            .get(index + 1)
            .map_or(base.len(), |next| next.base.start);
        let first_is_word = base[hunk.base.clone()]
            .first()
            .or_else(|| other[hunk.other.clone()].first())
            .is_some_and(|ch| is_word_char(*ch))
            || other[hunk.other.clone()]
                .first()
                .is_some_and(|ch| is_word_char(*ch));
        if hunk.base.start > lower && is_word_char(base[hunk.base.start - 1]) && first_is_word {
            while hunk.base.start > lower && is_word_char(base[hunk.base.start - 1]) {
                hunk.base.start -= 1;
                hunk.other.start -= 1;
            }
        }
        let last_is_word = base[hunk.base.clone()]
            .last()
            .is_some_and(|ch| is_word_char(*ch))
            || other[hunk.other.clone()]
                .last()
                .is_some_and(|ch| is_word_char(*ch));
        if hunk.base.end < upper && is_word_char(base[hunk.base.end]) && last_is_word {
            while hunk.base.end < upper && is_word_char(base[hunk.base.end]) {
                hunk.base.end += 1;
                hunk.other.end += 1;
            }
        }
        match widened.last_mut() {
            Some(previous) if previous.base.end >= hunk.base.start => {
                previous.base.end = hunk.base.end;
                previous.other.end = hunk.other.end;
            }
            _ => widened.push(hunk),
        }
    }
    widened
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Ours,
    Theirs,
}

/// Three-way merge of character sequences.
///
/// * `base` — the automatic output the person last saw;
/// * `ours` — what the output pane contains now (with manual edits);
/// * `theirs` — the new automatic output.
pub fn merge3(base: &str, ours: &str, theirs: &str) -> String {
    if ours == base {
        return theirs.to_owned();
    }
    if theirs == base || ours == theirs {
        return ours.to_owned();
    }
    let base_chars: Vec<char> = base.chars().collect();
    let ours_chars: Vec<char> = ours.chars().collect();
    let theirs_chars: Vec<char> = theirs.chars().collect();
    let ours_hunks = widen_to_words(diff(&base_chars, &ours_chars), &base_chars, &ours_chars);
    let theirs_hunks = widen_to_words(diff(&base_chars, &theirs_chars), &base_chars, &theirs_chars);
    let mut hunks: Vec<(Side, Hunk)> = ours_hunks
        .into_iter()
        .map(|hunk| (Side::Ours, hunk))
        .chain(theirs_hunks.into_iter().map(|hunk| (Side::Theirs, hunk)))
        .collect();
    // Order by position; at equal positions insertions come first and our
    // edits come before theirs, so text typed into an opened document is
    // appended after it.
    hunks.sort_by(|(side_a, a), (side_b, b)| {
        (a.base.start, a.base.end, *side_a == Side::Theirs).cmp(&(
            b.base.start,
            b.base.end,
            *side_b == Side::Theirs,
        ))
    });

    let mut output = String::with_capacity(ours.len().max(theirs.len()));
    let mut position = 0usize;
    let mut theirs_delta: isize = 0;
    let mut index = 0;
    while index < hunks.len() {
        // Gather a cluster of overlapping hunks.
        let start = hunks[index].1.base.start;
        let mut end = hunks[index].1.base.end;
        let mut members = vec![index];
        let mut next = index + 1;
        while next < hunks.len() && hunks[next].1.base.start < end {
            end = end.max(hunks[next].1.base.end);
            members.push(next);
            next += 1;
        }
        output.extend(&base_chars[position..start]);
        let has_ours = members.iter().any(|&member| hunks[member].0 == Side::Ours);
        let has_theirs = members
            .iter()
            .any(|&member| hunks[member].0 == Side::Theirs);
        let theirs_change: isize = members
            .iter()
            .filter(|&&member| hunks[member].0 == Side::Theirs)
            .map(|&member| {
                let hunk = &hunks[member].1;
                hunk.other.len() as isize - hunk.base.len() as isize
            })
            .sum();
        if has_ours && has_theirs {
            // Conflict: the new automatic text wins for this region only.
            let from = (start as isize + theirs_delta) as usize;
            let to = (end as isize + theirs_delta + theirs_change) as usize;
            output.extend(&theirs_chars[from..to]);
        } else {
            let (side, hunk) = &hunks[index];
            let source = if *side == Side::Ours {
                &ours_chars
            } else {
                &theirs_chars
            };
            output.extend(&source[hunk.other.clone()]);
        }
        theirs_delta += theirs_change;
        position = end;
        index = next;
    }
    output.extend(&base_chars[position..]);
    output
}

/// The smallest single replacement turning `old` into `new`, as
/// `(start, old_end, replacement)` in character offsets, or `None` when the
/// texts are equal.
pub fn minimal_edit(old: &str, new: &str) -> Option<(usize, usize, String)> {
    if old == new {
        return None;
    }
    let a: Vec<char> = old.chars().collect();
    let b: Vec<char> = new.chars().collect();
    let prefix = common_prefix(&a, &b);
    let suffix = common_suffix(&a[prefix..], &b[prefix..]);
    Some((
        prefix,
        a.len() - suffix,
        b[prefix..b.len() - suffix].iter().collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(text: &str) -> Vec<char> {
        text.chars().collect()
    }

    fn apply(base: &str, other: &str) -> String {
        let a = chars(base);
        let b = chars(other);
        let mut output = String::new();
        let mut position = 0;
        for hunk in diff(&a, &b) {
            output.extend(&a[position..hunk.base.start]);
            output.extend(&b[hunk.other.clone()]);
            position = hunk.base.end;
        }
        output.extend(&a[position..]);
        output
    }

    #[test]
    fn diff_reconstructs_the_other_text() {
        for (a, b) in [
            ("", "abc"),
            ("abc", ""),
            ("मेरा नाम अभिष्ट है", "मेरा नाम अभीष्ट है और"),
            ("the quick brown fox", "the quack brown fix!"),
            ("abcabba", "cbabac"),
        ] {
            assert_eq!(apply(a, b), b, "{a} -> {b}");
        }
    }

    #[test]
    fn untouched_output_follows_the_new_text() {
        assert_eq!(merge3("नमस्ते", "नमस्ते", "नमस्ते दुनिया"), "नमस्ते दुनिया");
    }

    #[test]
    fn manual_fix_survives_more_typing() {
        let base = "मेरा नाम अभिष्ट है";
        let ours = "मेरा नाम अभीष्ट है";
        let theirs = "मेरा नाम अभिष्ट है और मैं";
        assert_eq!(merge3(base, ours, theirs), "मेरा नाम अभीष्ट है और मैं");
    }

    #[test]
    fn retyping_the_same_word_replaces_the_manual_fix() {
        let base = "मेरा नाम अभिष्ट है";
        let ours = "मेरा नाम अभीष्ट है";
        let theirs = "मेरा नाम अभिषेक है";
        assert_eq!(merge3(base, ours, theirs), "मेरा नाम अभिषेक है");
    }

    #[test]
    fn opened_document_keeps_new_text_after_it() {
        assert_eq!(merge3("", "पुराना पाठ\n", "नया"), "पुराना पाठ\nनया");
        assert_eq!(
            merge3("नया", "पुराना पाठ\nनया", "नया शब्द"),
            "पुराना पाठ\nनया शब्द"
        );
        // Clearing the Roman pane removes only the generated text.
        assert_eq!(merge3("नया शब्द", "पुराना पाठ\nनया शब्द", ""), "पुराना पाठ\n");
    }

    #[test]
    fn edits_on_both_ends_are_kept() {
        let base = "one two three four five";
        let ours = "ONE two three four FIVE";
        let theirs = "one two 3 four five";
        assert_eq!(merge3(base, ours, theirs), "ONE two 3 four FIVE");
    }

    #[test]
    fn minimal_edit_finds_the_changed_middle() {
        assert_eq!(
            minimal_edit("नमस्ते दुनिया", "नमस्ते प्यारी दुनिया"),
            Some((7, 7, "प्यारी ".into()))
        );
        assert_eq!(minimal_edit("same", "same"), None);
        assert_eq!(minimal_edit("abc", ""), Some((0, 3, String::new())));
    }
}
