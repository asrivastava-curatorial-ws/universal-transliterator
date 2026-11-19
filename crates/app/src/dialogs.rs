#![forbid(unsafe_code)]
//! Dialogs: unsaved changes, files, preferences, script converter, help.

use crate::app::{self, SharedRef, REPOSITORY};
use crate::theme::Theme;
use gtk::prelude::*;
use gtk::{gio, glib};
use ut_transliteration::brahmic::{self, IndicScript, Target};

/// Asks what to do with unsaved output before it is replaced. Calls `then`
/// with `true` when it is safe to continue.
pub fn confirm_unsaved<F: FnOnce(bool) + 'static>(shared: &SharedRef, then: F) {
    if !app::is_dirty(shared) {
        then(true);
        return;
    }
    let name = shared.app.borrow().document_name();
    let dialog = gtk::AlertDialog::builder()
        .message(format!("Save changes to “{name}”?"))
        .detail("The output has changes that are not saved to a file.")
        .buttons(["Cancel", "Don’t save", "Save…"])
        .cancel_button(0)
        .default_button(2)
        .modal(true)
        .build();
    let target = shared.clone();
    dialog.choose(
        Some(&shared.ui.window),
        gio::Cancellable::NONE,
        move |result| match result {
            Ok(1) => then(true),
            Ok(2) => {
                let path = target.app.borrow().current_path.clone();
                match path {
                    Some(path) => {
                        let saved = app::save_to(&target, &path);
                        then(saved);
                    }
                    None => save_as(&target, then),
                }
            }
            _ => then(false),
        },
    );
}

fn text_filters() -> gio::ListStore {
    let text = gtk::FileFilter::new();
    text.set_name(Some("Text files"));
    text.add_mime_type("text/plain");
    text.add_suffix("txt");
    text.add_suffix("md");
    let all = gtk::FileFilter::new();
    all.set_name(Some("All files"));
    all.add_pattern("*");
    let filters = gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&text);
    filters.append(&all);
    filters
}

pub fn open_file(shared: &SharedRef) {
    let dialog = gtk::FileDialog::builder()
        .title("Open a UTF-8 text file")
        .modal(true)
        .filters(&text_filters())
        .build();
    let target = shared.clone();
    dialog.open(
        Some(&shared.ui.window),
        gio::Cancellable::NONE,
        move |result| {
            if let Ok(file) = result {
                match file.path() {
                    Some(path) => app::open_document(&target, &path),
                    None => app::toast(&target, "Only local files can be opened"),
                }
            }
        },
    );
}

fn suggested_name(shared: &SharedRef, extension: &str) -> String {
    let app = shared.app.borrow();
    let stem = app
        .current_path
        .as_ref()
        .and_then(|path| path.file_stem())
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| {
            format!(
                "{}-text",
                app.language
                    .name
                    .split(' ')
                    .next()
                    .unwrap_or("output")
                    .to_lowercase()
            )
        });
    format!("{stem}.{extension}")
}

pub fn save_as<F: FnOnce(bool) + 'static>(shared: &SharedRef, then: F) {
    let dialog = gtk::FileDialog::builder()
        .title("Save the output as UTF-8 text")
        .modal(true)
        .initial_name(suggested_name(shared, "txt"))
        .build();
    let target = shared.clone();
    dialog.save(
        Some(&shared.ui.window),
        gio::Cancellable::NONE,
        move |result| match result.ok().and_then(|file| file.path()) {
            Some(path) => {
                let saved = app::save_to(&target, &path);
                then(saved);
            }
            None => then(false),
        },
    );
}

pub fn export_docx(shared: &SharedRef) {
    let dialog = gtk::FileDialog::builder()
        .title("Export a Word document")
        .modal(true)
        .initial_name(suggested_name(shared, "docx"))
        .build();
    let target = shared.clone();
    dialog.save(
        Some(&shared.ui.window),
        gio::Cancellable::NONE,
        move |result| {
            if let Some(path) = result.ok().and_then(|file| file.path()) {
                let text = app::text_of(&target.ui.output);
                let language = target.app.borrow().language;
                let style = ut_export::DocxStyle {
                    rtl: language.rtl,
                    complex_font: Some(language.script.font_family().to_owned()),
                };
                match ut_export::write_docx_styled(&path, &text, true, &style) {
                    Ok(()) => app::toast(&target, "Exported the Word document"),
                    Err(error) => app::toast(
                        &target,
                        &app::public_io_error("export the document", &error),
                    ),
                }
            }
        },
    );
}

pub fn export_roman(shared: &SharedRef) {
    let dialog = gtk::FileDialog::builder()
        .title("Save the Roman input")
        .modal(true)
        .initial_name("roman-input.txt")
        .build();
    let target = shared.clone();
    dialog.save(
        Some(&shared.ui.window),
        gio::Cancellable::NONE,
        move |result| {
            if let Some(path) = result.ok().and_then(|file| file.path()) {
                let text = app::text_of(&target.ui.roman);
                match ut_storage::atomic_write_private(&path, text.as_bytes(), true) {
                    Ok(()) => app::toast(&target, "Saved the Roman input"),
                    Err(error) => app::toast(
                        &target,
                        &app::public_io_error("save the Roman input", &error),
                    ),
                }
            }
        },
    );
}

// ---------------------------------------------------------------------------
// Preferences
// ---------------------------------------------------------------------------

fn dialog_window(shared: &SharedRef, title: &str, width: i32) -> gtk::Window {
    let window = gtk::Window::builder()
        .title(title)
        .transient_for(&shared.ui.window)
        .modal(true)
        .default_width(width)
        .resizable(true)
        .build();
    let close = gtk::EventControllerKey::new();
    let target = window.clone();
    close.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            target.close();
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    window.add_controller(close);
    window
}

fn section(grid: &gtk::Grid, row: &mut i32, title: &str) {
    let label = gtk::Label::new(Some(title));
    label.add_css_class("section-title");
    label.set_xalign(0.0);
    grid.attach(&label, 0, *row, 2, 1);
    *row += 1;
}

fn setting_row(
    grid: &gtk::Grid,
    row: &mut i32,
    title: &str,
    detail: &str,
    control: &impl IsA<gtk::Widget>,
) {
    let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let label = gtk::Label::new(Some(title));
    label.set_xalign(0.0);
    text.append(&label);
    if !detail.is_empty() {
        let detail = gtk::Label::new(Some(detail));
        detail.set_xalign(0.0);
        detail.set_wrap(true);
        detail.set_max_width_chars(46);
        detail.add_css_class("dim");
        text.append(&detail);
    }
    text.set_hexpand(true);
    grid.attach(&text, 0, *row, 1, 1);
    control.set_valign(gtk::Align::Center);
    control.set_halign(gtk::Align::End);
    grid.attach(control, 1, *row, 1, 1);
    *row += 1;
}

pub fn preferences(shared: &SharedRef) {
    let settings = shared.app.borrow().settings.clone();
    let window = dialog_window(shared, "Preferences", 560);
    let grid = gtk::Grid::builder()
        .row_spacing(12)
        .column_spacing(24)
        .build();
    grid.add_css_class("dialog-content");
    let mut row = 0;

    section(&grid, &mut row, "Appearance");
    let theme = gtk::DropDown::from_strings(&["GitHub Dark", "GitHub Light", "Follow system"]);
    theme.set_selected(match settings.theme.as_str() {
        "light" => 1,
        "system" => 2,
        _ => 0,
    });
    let target = shared.clone();
    theme.connect_selected_notify(move |dropdown| {
        let value = match dropdown.selected() {
            1 => "light",
            2 => "system",
            _ => "dark",
        };
        {
            let mut app = target.app.borrow_mut();
            app.settings.theme = value.into();
            app.save_settings();
        }
        app::apply_theme(&target, Theme::from_setting(value));
    });
    setting_row(&grid, &mut row, "Theme", "", &theme);

    let size = gtk::SpinButton::with_range(9.0, 48.0, 1.0);
    size.set_value(f64::from(settings.font_size));
    let target = shared.clone();
    size.connect_value_changed(move |spin| {
        let wanted = spin.value() as i32;
        let current = i32::from(target.app.borrow().settings.font_size);
        if wanted != current {
            app::zoom(&target, wanted - current);
        }
    });
    setting_row(
        &grid,
        &mut row,
        "Text size",
        "Points; also Ctrl+= and Ctrl+-",
        &size,
    );

    let layout = gtk::DropDown::from_strings(&["Side by side", "Stacked"]);
    layout.set_selected(u32::from(settings.vertical_layout));
    let target = shared.clone();
    layout.connect_selected_notify(move |dropdown| {
        let vertical = dropdown.selected() == 1;
        {
            let mut app = target.app.borrow_mut();
            app.settings.vertical_layout = vertical;
            app.save_settings();
        }
        app::apply_layout(&target, vertical);
    });
    setting_row(&grid, &mut row, "Panes", "", &layout);

    section(&grid, &mut row, "Typing");
    let learn = gtk::Switch::new();
    learn.set_active(settings.learning_enabled);
    let target = shared.clone();
    learn.connect_active_notify(move |switch| {
        let mut app = target.app.borrow_mut();
        app.settings.learning_enabled = switch.is_active();
        app.save_settings();
        app.save_learned();
    });
    setting_row(
        &grid,
        &mut row,
        "Remember my choices",
        "Alternatives you pick are reused next time (stored only on this computer).",
        &learn,
    );
    let count = shared.app.borrow().converter.learned.len();
    let forget = gtk::Button::with_label(&format!(
        "Forget {count} learned word{}",
        if count == 1 { "" } else { "s" }
    ));
    forget.add_css_class("destructive-action");
    forget.set_sensitive(count > 0);
    let target = shared.clone();
    forget.connect_clicked(move |button| {
        {
            let mut app = target.app.borrow_mut();
            app.converter.learned = ut_storage::Learned::default();
            let _ = std::fs::remove_file(app.paths.learned_file());
        }
        button.set_label("Forgot all learned words");
        button.set_sensitive(false);
        app::render_now(&target);
    });
    setting_row(&grid, &mut row, "Learned words", "", &forget);

    let digits = gtk::Switch::new();
    digits.set_active(settings.native_digits);
    let target = shared.clone();
    digits.connect_active_notify(move |switch| {
        {
            let mut app = target.app.borrow_mut();
            app.settings.native_digits = switch.is_active();
            app.save_settings();
        }
        app::render_now(&target);
    });
    setting_row(
        &grid,
        &mut row,
        "Native digits",
        "Write 0–9 with the script’s own digits (Devanagari, Kannada, Arabic …).",
        &digits,
    );

    section(&grid, &mut row, "Session");
    let restore = gtk::Switch::new();
    restore.set_active(settings.autosave);
    let target = shared.clone();
    restore.connect_active_notify(move |switch| {
        {
            let mut app = target.app.borrow_mut();
            app.settings.autosave = switch.is_active();
            app.save_settings();
        }
        app::save_session_now(&target);
    });
    setting_row(
        &grid,
        &mut row,
        "Restore my work on start",
        "Both panes are kept privately between runs, so closing never loses text.",
        &restore,
    );
    let keyboard = gtk::Switch::new();
    keyboard.set_active(settings.show_keyboard);
    let target = shared.clone();
    keyboard.connect_active_notify(move |switch| {
        target.ui.keyboard_button.set_active(switch.is_active());
    });
    setting_row(
        &grid,
        &mut row,
        "Show the on-screen keyboard",
        "",
        &keyboard,
    );

    let close = gtk::Button::with_label("Close");
    close.add_css_class("suggested-action");
    close.set_halign(gtk::Align::End);
    close.set_margin_top(10);
    let target = window.clone();
    close.connect_clicked(move |_| target.close());
    grid.attach(&close, 1, row, 1, 1);

    window.set_child(Some(&grid));
    window.present();
}

// ---------------------------------------------------------------------------
// Script converter
// ---------------------------------------------------------------------------

pub fn script_converter(shared: &SharedRef) {
    let buffer = shared.ui.output.buffer();
    let (source, selection) = match buffer.selection_bounds() {
        Some((start, end)) => (buffer.text(&start, &end, true).to_string(), true),
        None => (app::text_of(&shared.ui.output), false),
    };
    let detected = brahmic::detect(&source);
    let window = dialog_window(shared, "Convert script", 720);
    window.set_default_height(480);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    content.add_css_class("dialog-content");

    let targets = Target::all();
    let names: Vec<&str> = targets.iter().map(|target| target.name()).collect();
    let dropdown = gtk::DropDown::from_strings(&names);
    let default_target = match detected {
        Some(IndicScript::Devanagari) => Target::Roman(brahmic::Romanization::Iast),
        _ => Target::Script(IndicScript::Devanagari),
    };
    if let Some(position) = targets.iter().position(|target| *target == default_target) {
        dropdown.set_selected(position as u32);
    }
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let from = gtk::Label::new(Some(&format!(
        "{} · {}  →",
        if selection {
            "Selection"
        } else {
            "Whole output"
        },
        detected.map_or("no Indic script found", |script| script.name())
    )));
    from.add_css_class("dim");
    header.append(&from);
    header.append(&dropdown);
    let explanation = gtk::Label::new(Some(
        "Converts between Indic scripts (Devanagari, Bengali, Gurmukhi, Gujarati, Odia, Tamil, Telugu, Kannada, Malayalam, Sinhala) and romanizes with IAST, ISO 15919, ITRANS or simple Roman spelling.",
    ));
    explanation.set_wrap(true);
    explanation.set_xalign(0.0);
    explanation.add_css_class("dim");
    content.append(&header);
    content.append(&explanation);

    let preview = gtk::TextView::builder()
        .wrap_mode(gtk::WrapMode::WordChar)
        .editable(false)
        .left_margin(14)
        .right_margin(14)
        .top_margin(12)
        .bottom_margin(12)
        .vexpand(true)
        .build();
    preview.add_css_class("preview-editor");
    let scroll = gtk::ScrolledWindow::builder()
        .child(&preview)
        .vexpand(true)
        .min_content_height(260)
        .build();
    scroll.add_css_class("panel");
    content.append(&scroll);

    let update = {
        let preview = preview.clone();
        let source = source.clone();
        let targets = targets.clone();
        move |index: u32| {
            if let Some(target) = targets.get(index as usize) {
                preview
                    .buffer()
                    .set_text(&brahmic::convert(&source, *target));
            }
        }
    };
    update(dropdown.selected());
    let refresh = update.clone();
    dropdown.connect_selected_notify(move |dropdown| refresh(dropdown.selected()));

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::End);
    let copy = gtk::Button::with_label("Copy");
    let replace = gtk::Button::with_label(if selection {
        "Replace selection"
    } else {
        "Replace output"
    });
    replace.add_css_class("suggested-action");
    let close = gtk::Button::with_label("Close");
    buttons.append(&close);
    buttons.append(&copy);
    buttons.append(&replace);
    content.append(&buttons);

    let target = shared.clone();
    let preview_for_copy = preview.clone();
    copy.connect_clicked(move |_| {
        let text = app::text_of(&preview_for_copy);
        target.ui.window.clipboard().set_text(&text);
        app::toast(&target, "Copied the converted text");
    });
    let target = shared.clone();
    let preview_for_replace = preview.clone();
    let dialog = window.clone();
    replace.connect_clicked(move |_| {
        let converted = app::text_of(&preview_for_replace);
        let buffer = target.ui.output.buffer();
        buffer.begin_user_action();
        let (mut start, mut end) = buffer
            .selection_bounds()
            .unwrap_or_else(|| (buffer.start_iter(), buffer.end_iter()));
        buffer.delete(&mut start, &mut end);
        buffer.insert(&mut start, &converted);
        buffer.end_user_action();
        app::toast(
            &target,
            "Replaced with the converted text (Ctrl+Z in the output undoes it)",
        );
        dialog.close();
    });
    let dialog = window.clone();
    close.connect_clicked(move |_| dialog.close());

    window.set_child(Some(&content));
    window.present();
}

// ---------------------------------------------------------------------------
// Help
// ---------------------------------------------------------------------------

const SHORTCUTS: &[(&str, &[(&str, &str)])] = &[
    (
        "Typing",
        &[
            (
                "Alt+1 … Alt+9",
                "Pick an alternative for the word at the cursor",
            ),
            (
                "{text}",
                "Keep text exactly as typed (English words, names, code)",
            ),
            ("Ctrl+L", "Choose language"),
            ("Ctrl+Shift+L", "Switch back to the previous language"),
            ("Ctrl+M", "AI ↔ Exact (ITRANS / IAST) for Indic languages"),
            ("Ctrl+K", "On-screen keyboard"),
            ("Ctrl+Z / Ctrl+Shift+Z", "Undo / redo in the focused pane"),
        ],
    ),
    (
        "Output",
        &[
            ("Ctrl+Shift+C", "Copy the output (or its selection)"),
            ("Ctrl+F / Ctrl+H", "Find / replace in the output"),
            ("F3 / Shift+F3", "Next / previous match"),
            ("Ctrl+Shift+K", "Convert script or romanize"),
        ],
    ),
    (
        "Files",
        &[
            ("Ctrl+N", "New document"),
            ("Ctrl+O", "Open (drag and drop works too)"),
            ("Ctrl+S / Ctrl+Shift+S", "Save / save as UTF-8 text"),
            ("Ctrl+E", "Export a Word document (.docx)"),
        ],
    ),
    (
        "View",
        &[
            (
                "Ctrl+= / Ctrl+- / Ctrl+0",
                "Bigger / smaller / default text",
            ),
            ("Ctrl+Shift+T", "Light / dark theme"),
            ("Ctrl+Shift+V", "Side by side / stacked panes"),
            ("Ctrl+,", "Preferences"),
            ("F1", "This list"),
            ("Ctrl+Q", "Quit"),
        ],
    ),
];

pub fn shortcuts(shared: &SharedRef) {
    let window = dialog_window(shared, "Keyboard shortcuts", 640);
    let grid = gtk::Grid::builder()
        .row_spacing(6)
        .column_spacing(18)
        .build();
    grid.add_css_class("dialog-content");
    let mut row = 0;
    for (title, items) in SHORTCUTS {
        section(&grid, &mut row, title);
        for (keys, description) in *items {
            let key = gtk::Label::new(Some(keys));
            key.add_css_class("kbd");
            key.set_halign(gtk::Align::Start);
            let text = gtk::Label::new(Some(description));
            text.set_xalign(0.0);
            text.set_hexpand(true);
            grid.attach(&key, 0, row, 1, 1);
            grid.attach(&text, 1, row, 1, 1);
            row += 1;
        }
    }
    let scroll = gtk::ScrolledWindow::builder()
        .child(&grid)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .min_content_height(520)
        .build();
    window.set_child(Some(&scroll));
    window.present();
}

pub fn about(shared: &SharedRef) {
    let about = gtk::AboutDialog::builder()
        .transient_for(&shared.ui.window)
        .modal(true)
        .program_name("Universal Transliterator")
        .version(env!("CARGO_PKG_VERSION"))
        .comments("Type in Roman letters, get Unicode text in 42 languages.\nFully offline: no cloud, no telemetry, no downloads.")
        .website(REPOSITORY)
        .website_label("Source code on GitHub")
        .license_type(gtk::License::Gpl30)
        .authors(vec!["Abhisht Srivastava".to_owned()])
        .logo_icon_name(app::APP_ID)
        .build();
    about.add_credit_section(
        "Offline engines",
        &[
            "AI4Bharat IndicXlit model (MIT)",
            "ONNX Runtime (MIT)",
            "Rime / librime Pinyin data",
            "libkkc and the SKK dictionary",
        ],
    );
    about.add_credit_section(
        "Fonts",
        &["JetBrains Mono (OFL)", "Google Noto fonts (OFL)"],
    );
    about.present();
}
