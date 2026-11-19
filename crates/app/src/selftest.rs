#![forbid(unsafe_code)]
//! Headless self-test, diagnostics and the command-line converter.

use crate::indic;
use crate::pipeline::{self, Converter, Options};
use std::io::Read;
use std::path::Path;
use ut_storage::Learned;
use ut_transliteration::brahmic::{self, IndicScript, Romanization, Target};
use ut_transliteration::{language, Engine, LANGUAGES};

fn availability(path: &Path) -> &'static str {
    if path.exists() {
        "present"
    } else {
        "missing"
    }
}

pub fn diagnostics() {
    println!("Universal Transliterator {}", env!("CARGO_PKG_VERSION"));
    println!(
        "target: {}-{}",
        std::env::consts::ARCH,
        std::env::consts::OS
    );
    println!(
        "AppImage: {}",
        std::env::var_os("APPIMAGE").map_or("no", |_| "yes")
    );
    println!("languages: {}", LANGUAGES.len());
    let rime = pipeline::rime_data_dir();
    println!("Rime data: {} ({})", rime.display(), availability(&rime));
    for (label, path) in [
        ("Japanese dictionary", pipeline::kkc_dictionary()),
        ("Japanese model", pipeline::kkc_model()),
        ("Japanese rules", pipeline::kkc_rule()),
    ] {
        println!("{label}: {} ({})", path.display(), availability(&path));
    }
    let indic_root = indic::resource_root();
    println!(
        "IndicXlit model: {} ({})",
        indic_root.display(),
        if indic::model_available() {
            "present"
        } else {
            "missing"
        }
    );
    println!("network listener: none; the application opens no sockets");
}

pub fn list_languages() {
    for language in LANGUAGES {
        let engine = match language.engine {
            Engine::Neural { .. } if language.supports_exact() => "AI + exact",
            Engine::Neural { .. } => "AI",
            Engine::Rime => "Rime",
            Engine::Kkc => "libkkc",
            Engine::Rules => "rules",
            Engine::Keyboard => "keyboard",
        };
        println!(
            "{:<5} {:<28} {:<14} {}",
            language.id, language.name, engine, language.native
        );
    }
}

/// `universal-transliterator --to hi [--exact] [TEXT…]` (reads stdin when no
/// text is given).
pub fn convert_command(arguments: &[String]) -> i32 {
    let mut language_id = None;
    let mut text_parts = Vec::new();
    let mut options = Options::default();
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--to" | "-t" => {
                language_id = arguments.get(index + 1).cloned();
                index += 1;
            }
            "--exact" => options.exact = true,
            "--native-digits" => options.native_digits = true,
            "--traditional" => options.traditional = true,
            "--katakana" => options.katakana = true,
            value if value.starts_with("--") => {}
            value => text_parts.push(value.to_owned()),
        }
        index += 1;
    }
    let Some(language) = language_id.as_deref().and_then(language) else {
        eprintln!("Unknown language. Run --list-languages to see the codes.");
        return 2;
    };
    let text = if text_parts.is_empty() {
        let mut input = String::new();
        if std::io::stdin()
            .take(16 * 1024 * 1024)
            .read_to_string(&mut input)
            .is_err()
        {
            eprintln!("Standard input is not UTF-8 text.");
            return 2;
        }
        input
    } else {
        text_parts.join(" ")
    };
    let (rime_dir, learned) = match ut_storage::Paths::discover() {
        Ok(paths) => {
            let settings = ut_storage::load_settings(&paths.settings_file()).unwrap_or_default();
            let learned = if settings.learning_enabled {
                ut_storage::load_learned(&paths.learned_file()).unwrap_or_default()
            } else {
                Learned::default()
            };
            (paths.rime_user_dir(), learned)
        }
        Err(_) => (
            std::env::temp_dir().join("universal-transliterator-rime"),
            Learned::default(),
        ),
    };
    options.exact = options.exact && language.supports_exact();
    let mut converter = Converter::new(rime_dir, learned, false);
    converter.prepare(language, &options);
    let rendered = converter.render(language, &text, &options);
    print!("{}", rendered.text);
    if !text.ends_with('\n') {
        println!();
    }
    0
}

fn check(failures: &mut Vec<String>, label: &str, actual: &str, expected: &str) {
    if actual == expected {
        println!("PASS {label}");
    } else {
        failures.push(format!("{label}: expected {expected:?}, got {actual:?}"));
    }
}

pub fn run() -> i32 {
    let started = std::time::Instant::now();
    let temporary = std::env::temp_dir().join(format!(
        "universal-transliterator-self-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ));
    if let Err(error) = std::fs::create_dir_all(&temporary) {
        eprintln!("SELF-TEST FAILED: cannot create a private temporary directory: {error}");
        return 1;
    }
    let mut failures = Vec::new();

    match ut_keyboards::builtins() {
        Ok(layouts) if layouts.len() >= 30 => {
            println!("PASS keyboard layouts: {} validated", layouts.len())
        }
        Ok(layouts) => failures.push(format!("only {} keyboard layouts loaded", layouts.len())),
        Err(error) => failures.push(format!("keyboard validation: {error}")),
    }
    for language in LANGUAGES {
        if ut_keyboards::find(
            &ut_keyboards::builtins().unwrap_or_default(),
            language.keyboard,
        )
        .is_none()
        {
            failures.push(format!(
                "{} has no keyboard layout {}",
                language.id, language.keyboard
            ));
        }
    }

    // Rule-based engines.
    check(
        &mut failures,
        "ITRANS Hindi",
        &ut_transliteration::transliterate("namaste bhaarat", "hi"),
        "नमस्ते भारत",
    );
    check(
        &mut failures,
        "Korean romanization",
        &ut_transliteration::transliterate("annyeonghaseyo", "ko"),
        "안녕하세요",
    );
    check(
        &mut failures,
        "Vietnamese Telex",
        &ut_transliteration::transliterate("tieengs Vieetj", "vi"),
        "tiếng Việt",
    );
    check(
        &mut failures,
        "Amharic",
        &ut_transliteration::transliterate("selam", "am"),
        "ሰላም",
    );
    check(
        &mut failures,
        "Santali Ol Chiki",
        &ut_transliteration::transliterate("johar", "sat"),
        "ᱡᱚᱦᱟᱨ",
    );
    check(
        &mut failures,
        "Russian",
        &ut_transliteration::transliterate("privet", "ru"),
        "привет",
    );
    check(
        &mut failures,
        "Arabic",
        &ut_transliteration::transliterate("mar7aba", "ar"),
        "مرحبا",
    );
    check(
        &mut failures,
        "script conversion",
        &brahmic::convert("नमस्ते", Target::Script(IndicScript::Kannada)),
        "ನಮಸ್ತೇ",
    );
    check(
        &mut failures,
        "romanization",
        &brahmic::convert("संस्कृतम्", Target::Roman(Romanization::Iast)),
        "saṃskṛtam",
    );
    check(
        &mut failures,
        "edit-preserving merge",
        &ut_text::merge3("मेरा नाम अभिष्ट", "मेरा नाम अभीष्ट", "मेरा नाम अभिष्ट है"),
        "मेरा नाम अभीष्ट है",
    );

    // Neural model through the full pipeline.
    let mut converter = Converter::new(temporary.join("rime"), Learned::default(), false);
    if let Some(hindi) = language("hi") {
        let rendered = converter.render(hindi, "Abhisht Srivastava {VITM}", &Options::default());
        check(
            &mut failures,
            "offline IndicXlit pipeline",
            &rendered.text,
            "अभिष्ट श्रीवास्तव VITM",
        );
        match rendered
            .words
            .first()
            .and_then(|span| converter.alternatives(hindi, &Options::default(), span))
        {
            Some(list) if list.len() >= 3 => {
                println!("PASS IndicXlit alternatives: {}", list.len())
            }
            other => failures.push(format!("IndicXlit alternatives: {other:?}")),
        }
    }

    // Mandarin through Rime.
    if let Some(chinese) = language("zh") {
        let options = Options::default();
        converter.prepare(chinese, &options);
        let rendered = converter.render(chinese, "ni hao, shijie", &options);
        check(&mut failures, "Rime Mandarin", &rendered.text, "你好，世界");
        match rendered
            .words
            .first()
            .and_then(|span| converter.alternatives(chinese, &options, span))
        {
            Some(list) if list.iter().any(|item| item.text == "你好") => {
                println!("PASS Rime alternatives: {}", list.len())
            }
            other => failures.push(format!("Rime alternatives: {other:?}")),
        }
    }

    // Japanese through libkkc.
    if let Some(japanese) = language("ja") {
        let options = Options::default();
        converter.prepare(japanese, &options);
        let rendered = converter.render(japanese, "toukyou", &options);
        if rendered.text.contains("東京") {
            println!("PASS libkkc Japanese");
        } else {
            failures.push(format!("libkkc Japanese output was {:?}", rendered.text));
        }
        let sentence = converter.render(japanese, "watashi ha gakusei desu.", &options);
        check(
            &mut failures,
            "libkkc sentence",
            &sentence.text,
            "私は学生です。",
        );
        match sentence
            .words
            .first()
            .and_then(|span| converter.alternatives(japanese, &options, span))
        {
            Some(list) if list.iter().any(|item| item.text == "私は学制です") => {
                println!("PASS libkkc per-segment alternatives: {}", list.len())
            }
            other => failures.push(format!("libkkc alternatives: {other:?}")),
        }
    }

    // Storage and export.
    let text_path = temporary.join("roundtrip.txt");
    match ut_storage::atomic_write_private(&text_path, "বাংলা 日本語 中文".as_bytes(), false)
        .and_then(|()| ut_storage::read_utf8(&text_path))
    {
        Ok(text) if text == "বাংলা 日本語 中文" => {
            println!("PASS atomic UTF-8 storage")
        }
        Ok(_) => failures.push("UTF-8 storage changed document content".into()),
        Err(error) => failures.push(format!("UTF-8 storage: {error}")),
    }
    let docx_path = temporary.join("self-test.docx");
    match ut_export::write_docx(&docx_path, "বাংলা\n日本語\n中文", false) {
        Ok(())
            if docx_path
                .metadata()
                .map(|metadata| metadata.len() > 0)
                .unwrap_or(false) =>
        {
            println!("PASS offline DOCX export")
        }
        Ok(()) => failures.push("DOCX export was empty".into()),
        Err(error) => failures.push(format!("DOCX export: {error}")),
    }

    let _ = std::fs::remove_dir_all(&temporary);
    if failures.is_empty() {
        println!("SELF-TEST PASSED in {} ms", started.elapsed().as_millis());
        0
    } else {
        for failure in failures {
            eprintln!("FAIL {failure}");
        }
        eprintln!("SELF-TEST FAILED in {} ms", started.elapsed().as_millis());
        1
    }
}
