//! Splits Roman input into words to convert, text to keep, and protected
//! spans. Byte offsets always fall on UTF-8 character boundaries.
//!
//! Protected spans are never converted:
//! * `{anything}` — braces are dropped from the output (`{{` gives `{`);
//!   an unclosed brace protects the rest of the line while you type;
//! * URLs (`https://…`, `www.…`) and e-mail addresses.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordKind {
    /// Nothing is converted (direct-keyboard languages).
    None,
    /// A–Z / a–z only (the neural model's alphabet).
    AsciiLetters,
    /// Any alphabetic character, including Latin letters with diacritics.
    Letters,
    /// Letters plus spaces between them: Pinyin syllables form one segment.
    Pinyin,
    /// A–Z / a–z plus spaces between them: a romaji phrase converted as a
    /// whole sentence (Japanese).
    AsciiPhrase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WordRule {
    pub kind: WordKind,
    /// Characters allowed inside a word when a letter follows.
    pub inner: &'static str,
    /// Characters allowed right after a letter, including at a word's end.
    pub trailing: &'static str,
    /// Digits count as letters when touching a Roman letter (Arabizi `7abibi`).
    pub digits_in_words: bool,
}

impl WordRule {
    pub const NONE: WordRule = WordRule::new(WordKind::None, "", "", false);

    pub const fn new(
        kind: WordKind,
        inner: &'static str,
        trailing: &'static str,
        digits_in_words: bool,
    ) -> Self {
        Self {
            kind,
            inner,
            trailing,
            digits_in_words,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// Convertible word: `text = input[start..end]`.
    Word,
    /// Kept verbatim except for punctuation and digit mapping.
    Text,
    /// Kept verbatim; the visible content is `input[inner_start..inner_end]`.
    Protected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: usize,
    pub end: usize,
    pub inner_start: usize,
    pub inner_end: usize,
}

impl Token {
    fn new(kind: TokenKind, start: usize, end: usize) -> Self {
        Self {
            kind,
            start,
            end,
            inner_start: start,
            inner_end: end,
        }
    }

    pub fn text<'a>(&self, input: &'a str) -> &'a str {
        &input[self.start..self.end]
    }

    pub fn inner<'a>(&self, input: &'a str) -> &'a str {
        &input[self.inner_start..self.inner_end]
    }
}

fn is_combining(ch: char) -> bool {
    matches!(ch as u32, 0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF)
}

fn is_base_letter(rule: &WordRule, ch: char) -> bool {
    match rule.kind {
        WordKind::None => false,
        WordKind::AsciiLetters | WordKind::AsciiPhrase => ch.is_ascii_alphabetic(),
        WordKind::Letters | WordKind::Pinyin => ch.is_alphabetic() || is_combining(ch),
    }
}

/// Length in bytes of a URL or e-mail address starting at `start`, if any.
fn link_length(input: &str, start: usize) -> Option<usize> {
    let rest = &input[start..];
    let chunk_len = rest
        .find(|ch: char| ch.is_whitespace() || matches!(ch, '{' | '}' | '<' | '>' | '"'))
        .unwrap_or(rest.len());
    let chunk = &rest[..chunk_len];
    let lower = chunk.to_ascii_lowercase();
    let is_url = [
        "http://", "https://", "ftp://", "file://", "mailto:", "www.",
    ]
    .iter()
    .any(|prefix| lower.starts_with(prefix) && chunk.len() > prefix.len());
    let is_email = chunk.find('@').is_some_and(|at| {
        at > 0
            && chunk[at + 1..].contains('.')
            && chunk[..at]
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || "._%+-".contains(ch))
    });
    if !is_url && !is_email {
        return None;
    }
    // Trailing sentence punctuation is not part of the link.
    let trimmed = chunk.trim_end_matches(|ch: char| {
        matches!(ch, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '\'')
    });
    (!trimmed.is_empty()).then_some(trimmed.len())
}

/// Tokenizes `input` according to `rule`.
pub fn tokenize(input: &str, rule: &WordRule) -> Vec<Token> {
    let chars: Vec<(usize, char)> = input.char_indices().collect();
    let mut tokens: Vec<Token> = Vec::new();
    let mut index = 0;
    let push_text = |tokens: &mut Vec<Token>, start: usize, end: usize| {
        if start >= end {
            return;
        }
        if let Some(last) = tokens.last_mut() {
            if last.kind == TokenKind::Text && last.end == start {
                last.end = end;
                last.inner_end = end;
                return;
            }
        }
        tokens.push(Token::new(TokenKind::Text, start, end));
    };
    let byte_at = |position: usize| chars.get(position).map_or(input.len(), |(byte, _)| *byte);

    while index < chars.len() {
        let (start, ch) = chars[index];
        let at_chunk_start = index == 0 || chars[index - 1].1.is_whitespace();

        // {protected} spans.
        if ch == '{' {
            if chars.get(index + 1).map(|(_, next)| *next) == Some('{') {
                tokens.push(Token {
                    kind: TokenKind::Protected,
                    start,
                    end: byte_at(index + 2),
                    inner_start: byte_at(index + 1),
                    inner_end: byte_at(index + 2),
                });
                index += 2;
                continue;
            }
            let mut close = index + 1;
            while close < chars.len() && chars[close].1 != '}' && chars[close].1 != '\n' {
                close += 1;
            }
            let has_brace = close < chars.len() && chars[close].1 == '}';
            tokens.push(Token {
                kind: TokenKind::Protected,
                start,
                end: byte_at(if has_brace { close + 1 } else { close }),
                inner_start: byte_at(index + 1),
                inner_end: byte_at(close),
            });
            index = if has_brace { close + 1 } else { close };
            continue;
        }

        // Links.
        if at_chunk_start {
            if let Some(length) = link_length(input, start) {
                let end = start + length;
                tokens.push(Token::new(TokenKind::Protected, start, end));
                while index < chars.len() && chars[index].0 < end {
                    index += 1;
                }
                continue;
            }
        }

        // Words.
        if starts_word(rule, &chars, index) {
            index += 1;
            while index < chars.len() && continues_word(rule, &chars, index) {
                index += 1;
            }
            tokens.push(Token::new(TokenKind::Word, start, byte_at(index)));
            continue;
        }

        push_text(&mut tokens, start, byte_at(index + 1));
        index += 1;
    }
    tokens
}

fn char_at(chars: &[(usize, char)], index: usize) -> Option<char> {
    chars.get(index).map(|(_, ch)| *ch)
}

fn starts_word(rule: &WordRule, chars: &[(usize, char)], index: usize) -> bool {
    let ch = chars[index].1;
    if rule.kind == WordKind::None || is_combining(ch) {
        return false;
    }
    if is_base_letter(rule, ch) {
        return true;
    }
    if rule.digits_in_words && ch.is_ascii_digit() {
        return char_at(chars, index + 1).is_some_and(|next| next.is_ascii_alphabetic());
    }
    // ITRANS-style tokens such as ".D" or "~N" may begin a word.
    rule.inner.contains(ch)
        && matches!(ch, '.' | '~')
        && char_at(chars, index + 1).is_some_and(|next| next.is_ascii_alphabetic())
}

fn continues_word(rule: &WordRule, chars: &[(usize, char)], index: usize) -> bool {
    let ch = chars[index].1;
    let next = char_at(chars, index + 1);
    if is_base_letter(rule, ch) {
        return true;
    }
    if rule.digits_in_words && ch.is_ascii_digit() {
        let previous_letter =
            char_at(chars, index - 1).is_some_and(|previous| previous.is_ascii_alphabetic());
        return previous_letter || next.is_some_and(|next| next.is_ascii_alphabetic());
    }
    if matches!(rule.kind, WordKind::Pinyin | WordKind::AsciiPhrase) && (ch == ' ' || ch == '\t') {
        // Spaces join Pinyin syllables when another letter follows.
        let mut lookahead = index + 1;
        while char_at(chars, lookahead).is_some_and(|value| value == ' ' || value == '\t') {
            lookahead += 1;
        }
        return char_at(chars, lookahead).is_some_and(|value| is_base_letter(rule, value));
    }
    if rule.inner.contains(ch)
        && next.is_some_and(|value| {
            is_base_letter(rule, value)
                || rule.inner.contains(value)
                // Arabizi digit letters: "al-3arabiya".
                || (rule.digits_in_words
                    && value.is_ascii_digit()
                    && char_at(chars, index + 2).is_some_and(|after| after.is_ascii_alphabetic()))
        })
    {
        return true;
    }
    rule.trailing.contains(ch)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(input: &str, rule: &WordRule) -> Vec<String> {
        tokenize(input, rule)
            .iter()
            .map(|token| match token.kind {
                TokenKind::Word => format!("W:{}", token.text(input)),
                TokenKind::Text => format!("T:{}", token.text(input)),
                TokenKind::Protected => format!("P:{}", token.inner(input)),
            })
            .collect()
    }

    const ASCII: WordRule = WordRule::new(WordKind::AsciiLetters, "", "", false);

    #[test]
    fn splits_words_and_text() {
        assert_eq!(
            words("mera naam, abhisht!", &ASCII),
            ["W:mera", "T: ", "W:naam", "T:, ", "W:abhisht", "T:!"]
        );
    }

    #[test]
    fn braces_protect_text() {
        assert_eq!(
            words("mera {laptop} hai", &ASCII),
            ["W:mera", "T: ", "P:laptop", "T: ", "W:hai"]
        );
        assert_eq!(words("a {open", &ASCII), ["W:a", "T: ", "P:open"]);
        assert_eq!(words("{{x", &ASCII), ["P:{", "W:x"]);
    }

    #[test]
    fn links_are_protected() {
        assert_eq!(
            words("see https://vitm.gov.in/a. ok", &ASCII),
            ["W:see", "T: ", "P:https://vitm.gov.in/a", "T:. ", "W:ok"]
        );
        assert_eq!(
            words("mail abc@vitm.gov.in", &ASCII),
            ["W:mail", "T: ", "P:abc@vitm.gov.in"]
        );
    }

    #[test]
    fn pinyin_segments_span_spaces() {
        let rule = WordRule::new(WordKind::Pinyin, "'", "", false);
        assert_eq!(
            words("ni hao, xi'an ", &rule),
            ["W:ni hao", "T:, ", "W:xi'an", "T: "]
        );
    }

    #[test]
    fn romaji_phrases_span_spaces() {
        let rule = WordRule::new(WordKind::AsciiPhrase, "'-", "-", false);
        assert_eq!(
            words("watashi ha gakusei desu. ra-men", &rule),
            ["W:watashi ha gakusei desu", "T:. ", "W:ra-men"]
        );
    }

    #[test]
    fn itrans_inner_marks() {
        let rule = WordRule::new(WordKind::Letters, ".~^'_", "~", false);
        assert_eq!(
            words("ha.Nsa R^ishi hai. kṛṣṇa", &rule),
            [
                "W:ha.Nsa",
                "T: ",
                "W:R^ishi",
                "T: ",
                "W:hai",
                "T:. ",
                "W:kṛṣṇa"
            ]
        );
    }

    #[test]
    fn arabizi_digits() {
        let rule = WordRule::new(WordKind::Letters, "'", "", true);
        assert_eq!(words("7abibi 2024", &rule), ["W:7abibi", "T: 2024"]);
    }
}
