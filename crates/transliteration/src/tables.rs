//! Table-driven phonetic typing for alphabetic scripts: Greek, the Cyrillic
//! languages, Armenian, Georgian, Hebrew, Arabic and Persian.
//!
//! Every converter works on a single word so that word-final rules (Greek
//! final sigma, Hebrew final letters, Arabic/Persian vowel spelling) can be
//! applied. Abjad scripts return more than one candidate because Roman
//! spelling does not say which vowels are written.

type Pairs = &'static [(&'static str, &'static str)];

/// Longest-match replacement. With `fold_case`, input is matched
/// case-insensitively and the case of the first input letter is copied to
/// the output.
fn replace(word: &str, table: Pairs, fold_case: bool) -> String {
    let mut output = String::with_capacity(word.len() * 2);
    let chars: Vec<char> = word.chars().collect();
    let lower: Vec<char> = if fold_case {
        chars
            .iter()
            .map(|ch| ch.to_lowercase().next().unwrap_or(*ch))
            .collect()
    } else {
        chars.clone()
    };
    let mut index = 0;
    while index < chars.len() {
        let mut best: Option<(usize, &str)> = None;
        for (latin, native) in table {
            let count = latin.chars().count();
            if index + count <= lower.len()
                && lower[index..index + count]
                    .iter()
                    .copied()
                    .eq(latin.chars())
                && best.map_or(true, |(length, _)| count > length)
            {
                best = Some((count, native));
            }
        }
        match best {
            Some((count, native)) => {
                if fold_case && chars[index].is_uppercase() {
                    let mut letters = native.chars();
                    if let Some(first) = letters.next() {
                        output.extend(first.to_uppercase());
                        output.extend(letters);
                    }
                } else {
                    output.push_str(native);
                }
                index += count;
            }
            None => {
                output.push(chars[index]);
                index += 1;
            }
        }
    }
    output
}

// --- Greek ---------------------------------------------------------------

const GREEK: Pairs = &[
    ("th", "θ"),
    ("ps", "ψ"),
    ("ph", "φ"),
    ("ch", "χ"),
    ("ks", "ξ"),
    ("x", "ξ"),
    ("a", "α"),
    ("b", "β"),
    ("v", "β"),
    ("g", "γ"),
    ("d", "δ"),
    ("e", "ε"),
    ("z", "ζ"),
    ("h", "η"),
    ("i", "ι"),
    ("k", "κ"),
    ("c", "κ"),
    ("l", "λ"),
    ("m", "μ"),
    ("n", "ν"),
    ("o", "ο"),
    ("p", "π"),
    ("r", "ρ"),
    ("s", "σ"),
    ("t", "τ"),
    ("u", "υ"),
    ("y", "υ"),
    ("f", "φ"),
    ("w", "ω"),
    ("a'", "ά"),
    ("e'", "έ"),
    ("h'", "ή"),
    ("i'", "ί"),
    ("o'", "ό"),
    ("u'", "ύ"),
    ("y'", "ύ"),
    ("w'", "ώ"),
];

fn greek(word: &str) -> String {
    let mut output = replace(word, GREEK, true);
    if output.ends_with('σ') {
        output.pop();
        output.push('ς');
    }
    output
}

// --- Cyrillic --------------------------------------------------------------

const RUSSIAN: Pairs = &[
    ("shch", "щ"),
    ("sch", "щ"),
    ("zh", "ж"),
    ("kh", "х"),
    ("ts", "ц"),
    ("ch", "ч"),
    ("sh", "ш"),
    ("yo", "ё"),
    ("jo", "ё"),
    ("yu", "ю"),
    ("ju", "ю"),
    ("ya", "я"),
    ("ja", "я"),
    ("ye", "е"),
    ("je", "е"),
    ("e'", "э"),
    ("''", "ъ"),
    ("\"", "ъ"),
    ("'", "ь"),
    ("a", "а"),
    ("b", "б"),
    ("v", "в"),
    ("w", "в"),
    ("g", "г"),
    ("d", "д"),
    ("e", "е"),
    ("z", "з"),
    ("i", "и"),
    ("j", "й"),
    ("k", "к"),
    ("q", "к"),
    ("l", "л"),
    ("m", "м"),
    ("n", "н"),
    ("o", "о"),
    ("p", "п"),
    ("r", "р"),
    ("s", "с"),
    ("t", "т"),
    ("u", "у"),
    ("f", "ф"),
    ("h", "х"),
    ("x", "х"),
    ("c", "ц"),
    ("y", "ы"),
];

const UKRAINIAN: Pairs = &[
    ("shch", "щ"),
    ("zh", "ж"),
    ("kh", "х"),
    ("ts", "ц"),
    ("ch", "ч"),
    ("sh", "ш"),
    ("ye", "є"),
    ("je", "є"),
    ("yi", "ї"),
    ("ji", "ї"),
    ("yu", "ю"),
    ("ju", "ю"),
    ("ya", "я"),
    ("ja", "я"),
    ("'", "ь"),
    ("`", "ʼ"),
    ("a", "а"),
    ("b", "б"),
    ("v", "в"),
    ("w", "в"),
    ("h", "г"),
    ("g", "ґ"),
    ("d", "д"),
    ("e", "е"),
    ("z", "з"),
    ("y", "и"),
    ("i", "і"),
    ("j", "й"),
    ("k", "к"),
    ("l", "л"),
    ("m", "м"),
    ("n", "н"),
    ("o", "о"),
    ("p", "п"),
    ("r", "р"),
    ("s", "с"),
    ("t", "т"),
    ("u", "у"),
    ("f", "ф"),
    ("x", "х"),
    ("c", "ц"),
];

const BELARUSIAN: Pairs = &[
    ("dzh", "дж"),
    ("dz", "дз"),
    ("zh", "ж"),
    ("kh", "х"),
    ("ts", "ц"),
    ("ch", "ч"),
    ("sh", "ш"),
    ("eh", "э"),
    ("ye", "е"),
    ("je", "е"),
    ("yo", "ё"),
    ("jo", "ё"),
    ("yu", "ю"),
    ("ju", "ю"),
    ("ya", "я"),
    ("ja", "я"),
    ("'", "ь"),
    ("`", "’"),
    ("ŭ", "ў"),
    ("w", "ў"),
    ("a", "а"),
    ("b", "б"),
    ("v", "в"),
    ("h", "г"),
    ("g", "ґ"),
    ("d", "д"),
    ("e", "е"),
    ("z", "з"),
    ("i", "і"),
    ("j", "й"),
    ("k", "к"),
    ("l", "л"),
    ("m", "м"),
    ("n", "н"),
    ("o", "о"),
    ("p", "п"),
    ("r", "р"),
    ("s", "с"),
    ("t", "т"),
    ("u", "у"),
    ("f", "ф"),
    ("x", "х"),
    ("c", "ц"),
    ("y", "ы"),
];

const BULGARIAN: Pairs = &[
    ("sht", "щ"),
    ("zh", "ж"),
    ("ts", "ц"),
    ("ch", "ч"),
    ("sh", "ш"),
    ("yu", "ю"),
    ("ju", "ю"),
    ("ya", "я"),
    ("ja", "я"),
    ("'", "ь"),
    ("a", "а"),
    ("b", "б"),
    ("v", "в"),
    ("w", "в"),
    ("g", "г"),
    ("d", "д"),
    ("e", "е"),
    ("z", "з"),
    ("i", "и"),
    ("j", "й"),
    ("k", "к"),
    ("l", "л"),
    ("m", "м"),
    ("n", "н"),
    ("o", "о"),
    ("p", "п"),
    ("r", "р"),
    ("s", "с"),
    ("t", "т"),
    ("u", "у"),
    ("f", "ф"),
    ("h", "х"),
    ("x", "х"),
    ("c", "ц"),
    ("y", "ъ"),
];

const SERBIAN: Pairs = &[
    ("dzh", "џ"),
    ("dž", "џ"),
    ("dj", "ђ"),
    ("đ", "ђ"),
    ("lj", "љ"),
    ("nj", "њ"),
    ("ch", "ч"),
    ("č", "ч"),
    ("cj", "ћ"),
    ("ć", "ћ"),
    ("sh", "ш"),
    ("š", "ш"),
    ("zh", "ж"),
    ("ž", "ж"),
    ("a", "а"),
    ("b", "б"),
    ("v", "в"),
    ("w", "в"),
    ("g", "г"),
    ("d", "д"),
    ("e", "е"),
    ("z", "з"),
    ("i", "и"),
    ("j", "ј"),
    ("y", "ј"),
    ("k", "к"),
    ("q", "к"),
    ("l", "л"),
    ("m", "м"),
    ("n", "н"),
    ("o", "о"),
    ("p", "п"),
    ("r", "р"),
    ("s", "с"),
    ("t", "т"),
    ("u", "у"),
    ("f", "ф"),
    ("h", "х"),
    ("x", "кс"),
    ("c", "ц"),
];

const MACEDONIAN: Pairs = &[
    ("dzh", "џ"),
    ("dž", "џ"),
    ("dz", "ѕ"),
    ("gj", "ѓ"),
    ("ǵ", "ѓ"),
    ("kj", "ќ"),
    ("ḱ", "ќ"),
    ("lj", "љ"),
    ("nj", "њ"),
    ("ch", "ч"),
    ("č", "ч"),
    ("sh", "ш"),
    ("š", "ш"),
    ("zh", "ж"),
    ("ž", "ж"),
    ("a", "а"),
    ("b", "б"),
    ("v", "в"),
    ("w", "в"),
    ("g", "г"),
    ("d", "д"),
    ("e", "е"),
    ("z", "з"),
    ("i", "и"),
    ("j", "ј"),
    ("y", "ј"),
    ("k", "к"),
    ("l", "л"),
    ("m", "м"),
    ("n", "н"),
    ("o", "о"),
    ("p", "п"),
    ("r", "р"),
    ("s", "с"),
    ("t", "т"),
    ("u", "у"),
    ("f", "ф"),
    ("h", "х"),
    ("c", "ц"),
];

fn is_cyrillic_vowel(ch: char) -> bool {
    matches!(
        ch.to_lowercase().next().unwrap_or(ch),
        'а' | 'е' | 'ё' | 'и' | 'о' | 'у' | 'ы' | 'э' | 'ю' | 'я' | 'і' | 'ї' | 'є'
    )
}

/// After a vowel, Roman "y" is the semivowel й rather than ы / и.
fn semivowel_after_vowel(text: &str, from: char, upper_from: char) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut output = String::with_capacity(text.len());
    for (index, ch) in chars.iter().enumerate() {
        let previous_vowel = index > 0 && is_cyrillic_vowel(chars[index - 1]);
        if previous_vowel && *ch == from {
            output.push('й');
        } else if previous_vowel && *ch == upper_from {
            output.push('Й');
        } else {
            output.push(*ch);
        }
    }
    output
}

// --- Armenian and Georgian -----------------------------------------------

const ARMENIAN: Pairs = &[
    ("tch", "ճ"),
    ("ch'", "չ"),
    ("gh", "ղ"),
    ("dz", "ձ"),
    ("ts", "ց"),
    ("tz", "ծ"),
    ("zh", "ժ"),
    ("kh", "խ"),
    ("sh", "շ"),
    ("ch", "չ"),
    ("rr", "ռ"),
    ("ye", "ե"),
    ("ev", "և"),
    ("t'", "թ"),
    ("p'", "փ"),
    ("k'", "ք"),
    ("a", "ա"),
    ("b", "բ"),
    ("g", "գ"),
    ("d", "դ"),
    ("e", "ե"),
    ("e'", "է"),
    ("z", "զ"),
    ("@", "ը"),
    ("ə", "ը"),
    ("t", "տ"),
    ("i", "ի"),
    ("l", "լ"),
    ("x", "խ"),
    ("k", "կ"),
    ("h", "հ"),
    ("m", "մ"),
    ("y", "յ"),
    ("n", "ն"),
    ("o", "ո"),
    ("o'", "օ"),
    ("p", "պ"),
    ("j", "ջ"),
    ("r", "ր"),
    ("s", "ս"),
    ("v", "վ"),
    ("w", "վ"),
    ("c", "ց"),
    ("u", "ու"),
    ("q", "ք"),
    ("f", "ֆ"),
];

fn armenian(word: &str) -> String {
    let output = replace(word, ARMENIAN, true);
    // Word-initial "o" is written օ in modern orthography.
    let mut chars = output.chars();
    match chars.next() {
        Some('ո') if !output.starts_with("ու") => format!("օ{}", chars.as_str()),
        Some('Ո') if !output.starts_with("Ու") => format!("Օ{}", chars.as_str()),
        _ => output,
    }
}

const GEORGIAN: Pairs = &[
    ("ch'", "ჭ"),
    ("ts'", "წ"),
    ("k'", "კ"),
    ("t'", "ტ"),
    ("p'", "პ"),
    ("q'", "ყ"),
    ("gh", "ღ"),
    ("kh", "ხ"),
    ("sh", "შ"),
    ("ch", "ჩ"),
    ("ts", "ც"),
    ("dz", "ძ"),
    ("zh", "ჟ"),
    ("a", "ა"),
    ("b", "ბ"),
    ("g", "გ"),
    ("d", "დ"),
    ("e", "ე"),
    ("v", "ვ"),
    ("w", "ვ"),
    ("z", "ზ"),
    ("t", "თ"),
    ("i", "ი"),
    ("k", "ქ"),
    ("l", "ლ"),
    ("m", "მ"),
    ("n", "ნ"),
    ("o", "ო"),
    ("p", "ფ"),
    ("r", "რ"),
    ("s", "ს"),
    ("u", "უ"),
    ("f", "ფ"),
    ("q", "ყ"),
    ("j", "ჯ"),
    ("h", "ჰ"),
    ("x", "ხ"),
    ("c", "ც"),
    ("y", "ი"),
];

// --- Abjads: Hebrew, Arabic, Persian ---------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Abjad {
    Hebrew,
    Arabic,
    Persian,
}

/// Consonant table for each abjad (vowels are handled separately).
fn abjad_consonants(kind: Abjad) -> Pairs {
    match kind {
        Abjad::Hebrew => &[
            ("ch", "ח"),
            ("kh", "כ"),
            ("tz", "צ"),
            ("ts", "צ"),
            ("sh", "ש"),
            ("b", "ב"),
            ("g", "ג"),
            ("d", "ד"),
            ("h", "ה"),
            ("v", "ו"),
            ("w", "ו"),
            ("z", "ז"),
            ("T", "ט"),
            ("t", "ת"),
            ("y", "י"),
            ("k", "כ"),
            ("l", "ל"),
            ("m", "מ"),
            ("n", "נ"),
            ("s", "ס"),
            ("3", "ע"),
            ("'", "ע"),
            ("p", "פ"),
            ("f", "פ"),
            ("q", "ק"),
            ("K", "ק"),
            ("r", "ר"),
            ("x", "ח"),
            ("j", "ג׳"),
        ],
        Abjad::Arabic => &[
            ("th", "ث"),
            ("kh", "خ"),
            ("dh", "ذ"),
            ("sh", "ش"),
            ("gh", "غ"),
            ("b", "ب"),
            ("t", "ت"),
            ("j", "ج"),
            ("H", "ح"),
            ("7", "ح"),
            ("d", "د"),
            ("r", "ر"),
            ("z", "ز"),
            ("s", "س"),
            ("S", "ص"),
            ("D", "ض"),
            ("T", "ط"),
            ("6", "ط"),
            ("Z", "ظ"),
            ("f", "ف"),
            ("v", "ف"),
            ("q", "ق"),
            ("9", "ق"),
            ("k", "ك"),
            ("l", "ل"),
            ("m", "م"),
            ("n", "ن"),
            ("h", "ه"),
            ("w", "و"),
            ("y", "ي"),
            ("'", "ء"),
            ("2", "ء"),
            ("3", "ع"),
            ("5", "خ"),
            ("p", "ب"),
            ("g", "ج"),
        ],
        Abjad::Persian => &[
            ("kh", "خ"),
            ("zh", "ژ"),
            ("sh", "ش"),
            ("ch", "چ"),
            ("gh", "غ"),
            ("b", "ب"),
            ("p", "پ"),
            ("t", "ت"),
            ("T", "ط"),
            ("s", "س"),
            ("S", "ص"),
            ("j", "ج"),
            ("H", "ح"),
            ("h", "ه"),
            ("d", "د"),
            ("z", "ز"),
            ("Z", "ظ"),
            ("r", "ر"),
            ("f", "ف"),
            ("q", "ق"),
            ("k", "ک"),
            ("g", "گ"),
            ("l", "ل"),
            ("m", "م"),
            ("n", "ن"),
            ("v", "و"),
            ("w", "و"),
            ("y", "ی"),
            ("'", "ع"),
            ("3", "ع"),
        ],
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Vowel {
    /// a / e: usually unwritten inside a word.
    Short(char),
    /// aa, ee/ii, oo/uu, i, u, o: written with a mater lectionis.
    Long(char),
}

fn vowel_at(rest: &[char]) -> Option<(usize, Vowel)> {
    let pair: String = rest.iter().take(2).collect();
    let long = match pair.as_str() {
        "aa" | "â" => Some('a'),
        "ee" | "ii" => Some('i'),
        "oo" | "uu" | "ou" => Some('u'),
        "ai" | "ay" | "ei" | "ey" => Some('y'),
        _ => None,
    };
    if let Some(kind) = long {
        return Some((2, Vowel::Long(kind)));
    }
    match rest.first()? {
        'a' | 'e' => Some((1, Vowel::Short(rest[0]))),
        'â' => Some((1, Vowel::Long('a'))),
        'i' => Some((1, Vowel::Long('i'))),
        'u' => Some((1, Vowel::Long('u'))),
        'o' => Some((1, Vowel::Long('o'))),
        _ => None,
    }
}

fn abjad_vowel(
    kind: Abjad,
    vowel: Vowel,
    initial: bool,
    final_: bool,
    keep_short_a: bool,
) -> &'static str {
    match (kind, vowel) {
        // Hebrew.
        (Abjad::Hebrew, Vowel::Short(_)) if initial => "א",
        (Abjad::Hebrew, Vowel::Short('a')) if final_ => "ה",
        (Abjad::Hebrew, Vowel::Short(_)) if keep_short_a => "א",
        (Abjad::Hebrew, Vowel::Short(_)) => "",
        (Abjad::Hebrew, Vowel::Long('a')) => "א",
        (Abjad::Hebrew, Vowel::Long('i' | 'y')) if initial => "אי",
        (Abjad::Hebrew, Vowel::Long('i' | 'y')) => "י",
        (Abjad::Hebrew, Vowel::Long(_)) if initial => "או",
        (Abjad::Hebrew, Vowel::Long(_)) => "ו",
        // Arabic.
        (Abjad::Arabic, Vowel::Short(_)) if initial => "أ",
        (Abjad::Arabic, Vowel::Short('a')) if final_ => "ا",
        (Abjad::Arabic, Vowel::Short(_)) if final_ => "ي",
        (Abjad::Arabic, Vowel::Short(_)) if keep_short_a => "ا",
        (Abjad::Arabic, Vowel::Short(_)) => "",
        (Abjad::Arabic, Vowel::Long('a')) if initial => "آ",
        (Abjad::Arabic, Vowel::Long('a')) => "ا",
        (Abjad::Arabic, Vowel::Long('i')) if initial => "إي",
        (Abjad::Arabic, Vowel::Long('i' | 'y')) => "ي",
        (Abjad::Arabic, Vowel::Long(_)) if initial => "أو",
        (Abjad::Arabic, Vowel::Long(_)) => "و",
        // Persian.
        (Abjad::Persian, Vowel::Short(_)) if initial => "ا",
        (Abjad::Persian, Vowel::Short('e')) if final_ => "ه",
        (Abjad::Persian, Vowel::Short(_)) if final_ => "ا",
        (Abjad::Persian, Vowel::Short('a')) if keep_short_a => "ا",
        (Abjad::Persian, Vowel::Short(_)) => "",
        (Abjad::Persian, Vowel::Long('a')) if initial => "آ",
        (Abjad::Persian, Vowel::Long('a')) => "ا",
        (Abjad::Persian, Vowel::Long('i' | 'y')) if initial => "ای",
        (Abjad::Persian, Vowel::Long('i' | 'y')) => "ی",
        (Abjad::Persian, Vowel::Long('o')) if initial => "ا",
        (Abjad::Persian, Vowel::Long('o')) if !final_ => "",
        (Abjad::Persian, Vowel::Long(_)) if initial => "او",
        (Abjad::Persian, Vowel::Long(_)) => "و",
    }
}

/// `short_iu`: single *i*, *u* and *o* inside a word are short vowels and
/// are not written (Arabic "3alaykum" → عليكم rather than عليكوم).
fn abjad(word: &str, kind: Abjad, keep_short_a: bool, short_iu: bool) -> String {
    let chars: Vec<char> = word.chars().collect();
    let consonants = abjad_consonants(kind);
    let mut output = String::with_capacity(word.len() * 2);
    let mut index = 0;
    let mut last_consonant: Option<&str> = None;
    // A long i or u written as ي / و already spells a following y or w
    // ("3arabiya" → عربية, "siyah" → سیاه, "huwa" → هو).
    let mut last_vowel_letter: Option<&str> = None;
    if kind == Abjad::Arabic {
        if let Some(skip) = arabic_article(&chars) {
            output.push_str("ال");
            index = skip;
            // Not a plain ل, so "allah" keeps its second lam.
            last_consonant = Some("ل-article");
        }
    }
    while index < chars.len() {
        let rest = &chars[index..];
        let initial = index == 0;
        if let Some((count, vowel)) = vowel_at(rest) {
            let final_ = index + count == chars.len();
            let short = short_iu
                && count == 1
                && !initial
                && !final_
                && matches!(vowel, Vowel::Long('i' | 'u' | 'o'));
            let letter = if short {
                ""
            } else {
                abjad_vowel(kind, vowel, initial, final_, keep_short_a)
            };
            output.push_str(letter);
            last_consonant = None;
            last_vowel_letter = matches!(letter, "ي" | "ی" | "و").then_some(letter);
            index += count;
            continue;
        }
        let mut best: Option<(usize, &str)> = None;
        for (latin, native) in consonants {
            let count = latin.chars().count();
            if index + count <= chars.len()
                && chars[index..index + count]
                    .iter()
                    .copied()
                    .eq(latin.chars())
                && best.map_or(true, |(length, _)| count > length)
            {
                best = Some((count, native));
            }
        }
        match best {
            Some((count, native)) => {
                // Gemination is not written ("mm" → م).
                let doubled = last_consonant == Some(native)
                    || (kind != Abjad::Hebrew && last_vowel_letter == Some(native));
                if !doubled {
                    output.push_str(native);
                }
                last_consonant = Some(native);
                index += count;
            }
            None => {
                output.push(chars[index]);
                last_consonant = None;
                index += 1;
            }
        }
        last_vowel_letter = None;
    }
    if kind == Abjad::Hebrew {
        output = hebrew_final_forms(&output);
    }
    output
}

/// Length of a leading Arabic definite article: `al` (`al-`, `el-`) or an
/// assimilated one before a sun letter (`ash-shams`, `as-salam`, `an-nur`).
fn arabic_article(chars: &[char]) -> Option<usize> {
    let first = *chars.first()?;
    if first != 'a' && first != 'e' {
        return None;
    }
    let hyphen = chars.iter().position(|&ch| ch == '-');
    if let Some(hyphen @ 2..=3) = hyphen {
        let consonant = &chars[1..hyphen];
        let after = &chars[hyphen + 1..];
        let lam = consonant == ['l'];
        let sun = [
            "t", "th", "d", "dh", "r", "z", "s", "sh", "S", "D", "T", "Z", "n",
        ]
        .iter()
        .any(|letter| consonant.iter().copied().eq(letter.chars()))
            && after.starts_with(consonant);
        if (lam || sun) && !after.is_empty() {
            return Some(hyphen + 1);
        }
    }
    // Unhyphenated "al…" (alqamar, allah) at the start of a longer word.
    (first == 'a' && chars.len() > 2 && chars[1] == 'l').then_some(2)
}

/// Arabic feminine ending: a final short *a* after a consonant as ة.
fn taa_marbuta(word: &str, converted: &str) -> Option<String> {
    let lower: Vec<char> = word.to_lowercase().chars().collect();
    let n = lower.len();
    if n < 3 || lower[n - 1] != 'a' || matches!(lower[n - 2], 'a' | 'e' | 'i' | 'o' | 'u' | '-') {
        return None;
    }
    let stem = converted.strip_suffix('ا')?;
    (!stem.is_empty()).then(|| format!("{stem}ة"))
}

fn hebrew_final_forms(text: &str) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    if let Some(last) = chars.last_mut() {
        *last = match *last {
            'כ' => 'ך',
            'מ' => 'ם',
            'נ' => 'ן',
            'פ' => 'ף',
            'צ' => 'ץ',
            other => other,
        };
    }
    chars.into_iter().collect()
}

fn unique(values: Vec<String>) -> Vec<String> {
    let mut result: Vec<String> = Vec::with_capacity(values.len());
    for value in values {
        if !value.is_empty() && !result.contains(&value) {
            result.push(value);
        }
    }
    result
}

/// Candidate conversions of one word, best first. Returns an empty list for
/// languages this module does not handle.
pub fn candidates(language: &str, word: &str) -> Vec<String> {
    match language {
        "el" => vec![greek(word)],
        "ru" => vec![semivowel_after_vowel(
            &replace(word, RUSSIAN, true),
            'ы',
            'Ы',
        )],
        "uk" => vec![semivowel_after_vowel(
            &replace(word, UKRAINIAN, true),
            'и',
            'И',
        )],
        "be" => vec![semivowel_after_vowel(
            &replace(word, BELARUSIAN, true),
            'ы',
            'Ы',
        )],
        "bg" => vec![semivowel_after_vowel(
            &replace(word, BULGARIAN, true),
            'ъ',
            'Ъ',
        )],
        "sr" => vec![replace(word, SERBIAN, true)],
        "mk" => vec![replace(word, MACEDONIAN, true)],
        "hy" => vec![armenian(word)],
        "ka" => vec![replace(&word.to_lowercase(), GEORGIAN, false)],
        "he" => unique(vec![
            abjad(word, Abjad::Hebrew, false, false),
            abjad(word, Abjad::Hebrew, true, false),
        ]),
        "ar" => {
            let plain = abjad(word, Abjad::Arabic, false, false);
            let with_a = abjad(word, Abjad::Arabic, true, false);
            let short = abjad(word, Abjad::Arabic, false, true);
            let marbuta = taa_marbuta(word, &plain).unwrap_or_default();
            // "-iya" / "-iyya" is almost always ية (عربية, حرية).
            let lower = word.to_lowercase();
            if lower.ends_with("iya") || lower.ends_with("iyya") {
                unique(vec![marbuta, plain, with_a, short])
            } else {
                unique(vec![plain, with_a, marbuta, short])
            }
        }
        "fa" => {
            // Writers who mark long vowels by doubling ("salaam", "mamnoon")
            // use a single "a" for the short, unwritten vowel. Otherwise a
            // single "a" is usually the long ا ("farsi", "iran").
            let lower = word.to_lowercase();
            let marks_length = ["aa", "ee", "ii", "oo", "uu", "â"]
                .iter()
                .any(|marker| lower.contains(marker));
            unique(vec![
                abjad(word, Abjad::Persian, !marks_length, false),
                abjad(word, Abjad::Persian, marks_length, false),
            ])
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first(language: &str, word: &str) -> String {
        candidates(language, word).remove(0)
    }

    #[test]
    fn greek_words() {
        assert_eq!(first("el", "kalimera"), "καλιμερα");
        assert_eq!(first("el", "logos"), "λογος");
        assert_eq!(first("el", "Athina"), "Αθινα");
        assert_eq!(first("el", "kalhme'ra"), "καλημέρα");
    }

    #[test]
    fn cyrillic_words() {
        assert_eq!(first("ru", "privet"), "привет");
        assert_eq!(first("ru", "Moskva"), "Москва");
        assert_eq!(first("ru", "zdravstvuyte"), "здравствуйте");
        assert_eq!(first("uk", "Kyjiv"), "Київ");
        assert_eq!(first("uk", "dyakuyu"), "дякую");
        assert_eq!(first("bg", "Bylgariya"), "България");
        assert_eq!(first("bg", "zdravey"), "здравей");
        assert_eq!(first("sr", "Beograd"), "Београд");
        assert_eq!(first("sr", "ljubav"), "љубав");
        assert_eq!(first("mk", "Makedonija"), "Македонија");
        assert_eq!(first("be", "Minsk"), "Мінск");
    }

    #[test]
    fn caucasus_words() {
        assert_eq!(first("hy", "Hayastan"), "Հայաստան");
        assert_eq!(first("hy", "Yerevan"), "Երևան");
        assert_eq!(first("ka", "gamarjoba"), "გამარჯობა");
        assert_eq!(first("ka", "tbilisi"), "თბილისი");
        assert_eq!(first("ka", "sakartvelo"), "საქართველო");
    }

    #[test]
    fn abjad_words() {
        assert_eq!(first("he", "shalom"), "שלום");
        assert_eq!(first("he", "toda"), "תודה");
        assert_eq!(first("ar", "mar7aba"), "مرحبا");
        assert_eq!(first("ar", "7abibi"), "حبيبي");
        assert_eq!(first("ar", "salaam"), "سلام");
        assert_eq!(first("ar", "allah"), "الله");
        assert_eq!(first("ar", "al-3arabiya"), "العربية");
        assert_eq!(first("ar", "3arabiya"), "عربية");
        assert_eq!(first("ar", "ash-shams"), "الشمس");
        assert_eq!(first("ar", "al-qamar"), "القمر");
        assert_eq!(first("ar", "suwar"), "سور");
        assert!(candidates("ar", "3alaykum").contains(&"عليكم".to_string()));
        assert!(candidates("ar", "quwwa").contains(&"قوة".to_string()));
        assert!(candidates("ar", "madrasa").contains(&"مدرسة".to_string()));
        assert_eq!(first("fa", "siyah"), "سیاه");
        assert_eq!(first("fa", "salaam"), "سلام");
        assert_eq!(first("fa", "iran"), "ایران");
        assert_eq!(first("fa", "khaane"), "خانه");
        assert_eq!(first("fa", "farsi"), "فارسی");
        assert_eq!(first("fa", "mamnoon"), "ممنون");
        assert!(candidates("fa", "tehran").contains(&"تهران".to_string()));
    }
}
