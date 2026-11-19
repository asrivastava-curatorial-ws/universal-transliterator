#![forbid(unsafe_code)]
//! Universal Transliterator — offline Roman-to-Unicode editor.

mod app;
mod dialogs;
mod indic;
mod keyboard;
mod pipeline;
mod selftest;
mod theme;

use gtk::prelude::*;
use gtk::{gio, glib};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

const HELP: &str = "Universal Transliterator — type in Roman letters, get Unicode text, offline.

Usage:
  universal-transliterator [FILE]          open the editor (optionally with a text file)
  universal-transliterator --to LANG [TEXT…] [--exact] [--native-digits]
                                           convert TEXT (or standard input) and print it
  universal-transliterator --list-languages
  universal-transliterator --version | --diagnostics | --self-test

Editor options:
  --safe-mode       use the conservative Cairo renderer
  --new-window      start a separate instance

Examples:
  universal-transliterator --to hi \"mera naam abhisht hai\"
  echo \"vanakkam\" | universal-transliterator --to ta
  universal-transliterator --to sa --exact \"vasudhaiva kuTumbakam\"";

fn running_as_root() -> bool {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find(|line| line.starts_with("Uid:"))
                .and_then(|line| line.split_whitespace().nth(2))
                .and_then(|value| value.parse::<u32>().ok())
        })
        == Some(0)
}

fn main() -> glib::ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let has = |flag: &str| arguments.iter().any(|argument| argument == flag);
    if has("--version") {
        println!("Universal Transliterator {}", env!("CARGO_PKG_VERSION"));
        return glib::ExitCode::SUCCESS;
    }
    if has("--help") || has("-h") {
        println!("{HELP}");
        return glib::ExitCode::SUCCESS;
    }
    if has("--diagnostics") {
        selftest::diagnostics();
        return glib::ExitCode::SUCCESS;
    }
    if has("--self-test") {
        std::process::exit(selftest::run());
    }
    if has("--list-languages") {
        selftest::list_languages();
        return glib::ExitCode::SUCCESS;
    }
    if has("--to") || has("-t") {
        std::process::exit(selftest::convert_command(&arguments));
    }
    if has("--safe-mode") {
        std::env::set_var("GSK_RENDERER", "cairo");
    }
    let smoke_test = has("--ui-smoke-test");
    if running_as_root() && !smoke_test {
        eprintln!("Universal Transliterator does not run its window as root.");
        return glib::ExitCode::FAILURE;
    }
    let files: Vec<String> = arguments
        .iter()
        .filter(|argument| !argument.starts_with("--"))
        .cloned()
        .collect();

    // WM_CLASS on X11 comes from the program name. Using the application ID
    // lets desktops match the window to its .desktop file (taskbar icon,
    // pinning), as Wayland already does through the app ID.
    glib::set_prgname(Some(app::APP_ID));
    glib::set_application_name("Universal Transliterator");

    let mut flags = gio::ApplicationFlags::HANDLES_OPEN;
    if smoke_test || has("--new-window") {
        flags |= gio::ApplicationFlags::NON_UNIQUE;
    }
    let application = gtk::Application::builder()
        .application_id(app::APP_ID)
        .flags(flags)
        .build();
    let state: Rc<RefCell<Option<app::SharedRef>>> = Rc::new(RefCell::new(None));

    let activate_state = state.clone();
    application.connect_activate(move |application| {
        let existing = activate_state.borrow().clone();
        match existing {
            Some(shared) => shared.ui.window.present(),
            None => {
                let built = app::build(application);
                *activate_state.borrow_mut() = built;
            }
        }
    });
    let open_state = state.clone();
    application.connect_open(move |application, files, _| {
        if open_state.borrow().is_none() {
            let built = app::build(application);
            *open_state.borrow_mut() = built;
        }
        let shared = open_state.borrow().clone();
        if let (Some(shared), Some(path)) = (shared, files.first().and_then(gio::File::path)) {
            shared.ui.window.present();
            app::open_with_confirmation(&shared, path);
        }
    });

    if smoke_test {
        // Scripted launch for automated checks and screenshots:
        // UT_SMOKE_LANG, UT_SMOKE_TEXT, UT_SMOKE_MS.
        let smoke_state = state.clone();
        application.connect_activate(move |application| {
            let language = std::env::var("UT_SMOKE_LANG").ok();
            let text = std::env::var("UT_SMOKE_TEXT").ok();
            let duration = std::env::var("UT_SMOKE_MS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(2500);
            if let Some(shared) = smoke_state.borrow().clone() {
                if let Some(language) = language {
                    app::set_language(&shared, &language, false);
                }
                if let Some(text) = text {
                    shared
                        .ui
                        .roman
                        .buffer()
                        .set_text(&text.replace("\\n", "\n"));
                }
            }
            // UT_SMOKE_ACTIONS: comma-separated application actions to run.
            if let Ok(actions) = std::env::var("UT_SMOKE_ACTIONS") {
                let application = application.clone();
                glib::timeout_add_local_once(Duration::from_millis(1800), move || {
                    for action in actions.split(',').filter(|action| !action.is_empty()) {
                        let name = action.strip_prefix("app.").unwrap_or(action);
                        application.activate_action(name, None);
                    }
                });
            }
            let application = application.clone();
            let quit_state = smoke_state.clone();
            glib::timeout_add_local_once(Duration::from_millis(duration), move || {
                if let Some(shared) = quit_state.borrow().clone() {
                    println!("UI-SMOKE output: {}", app::text_of(&shared.ui.output));
                }
                application.quit();
            });
        });
    }

    let mut argv = vec!["universal-transliterator".to_owned()];
    argv.extend(files.into_iter().map(|file| {
        PathBuf::from(&file)
            .canonicalize()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or(file)
    }));
    application.run_with_args(&argv)
}
