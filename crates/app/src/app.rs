#![forbid(unsafe_code)]
//! Main window: state, widgets and behaviour.

use crate::dialogs;
use crate::keyboard;
use crate::pipeline::{Candidate, Converter, Options, Rendered, StatusKind, WordSpan};
use crate::theme::{Providers, Theme};
use gtk::prelude::*;
use gtk::{gdk, gio, glib, pango};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;
use ut_storage::{Paths, Session, Settings};
use ut_transliteration::{
    language, Engine, Group, LanguageInfo, DEFAULT_LANGUAGE, EXACT_NOTE, LANGUAGES,
};

pub const APP_ID: &str = "org.universaltransliterator.UniversalTransliterator";
pub const REPOSITORY: &str =
    "https://github.com/asrivastava-curatorial-ws/universal-transliterator";
const RENDER_DELAY_MS: u64 = 90;
/// While more words than this wait for the AI model, the previous output
/// stays on screen instead of flashing rule-based guesses.
const MAX_PENDING_TO_SHOW: usize = 3;
const MAX_ROMAN_BYTES: usize = 1_000_000;

pub struct App {
    pub settings: Settings,
    pub paths: Paths,
    pub converter: Converter,
    pub language: &'static LanguageInfo,
    pub current_path: Option<PathBuf>,
    /// Output text at the last save/open (for the unsaved-changes marker).
    pub saved_output: String,
    /// Output last generated from the Roman text (base for merging edits).
    pub auto_output: String,
    pub rendered: Rendered,
    pub candidate_span: Option<WordSpan>,
    pub candidate_list: Vec<Candidate>,
    pub generation: u64,
    pub session_dirty: bool,
    pub theme: Theme,
    /// Waiting for the AI model before showing a restored session.
    pub restoring: bool,
}

impl App {
    pub fn options(&self) -> Options {
        Options {
            exact: self.language.supports_exact()
                && self
                    .settings
                    .exact_languages
                    .iter()
                    .any(|id| id == self.language.id),
            native_digits: self.settings.native_digits,
            traditional: self.settings.chinese_variant == "traditional",
            katakana: self.settings.japanese_mode == "katakana",
        }
    }

    pub fn document_name(&self) -> String {
        self.current_path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".into())
    }

    pub fn save_settings(&self) {
        let _ = ut_storage::save_settings(&self.paths.settings_file(), &self.settings);
    }

    pub fn save_learned(&self) {
        if self.settings.learning_enabled {
            let _ = ut_storage::save_learned(&self.paths.learned_file(), &self.converter.learned);
        }
    }
}

#[derive(Clone)]
pub struct Ui {
    pub window: gtk::ApplicationWindow,
    pub roman: gtk::TextView,
    pub output: gtk::TextView,
    pub roman_placeholder: gtk::Label,
    pub roman_note: gtk::Label,
    pub output_title: gtk::Label,
    pub output_count: gtk::Label,
    pub paned: gtk::Paned,
    pub language_button: gtk::MenuButton,
    pub language_popover: gtk::Popover,
    pub language_native: gtk::Label,
    pub language_engine: gtk::Label,
    pub language_list: gtk::ListBox,
    pub language_search: gtk::SearchEntry,
    pub mode_box: gtk::Box,
    pub ai_button: gtk::ToggleButton,
    pub exact_button: gtk::ToggleButton,
    pub chinese_variant: gtk::DropDown,
    pub japanese_mode: gtk::DropDown,
    pub candidate_word: gtk::Label,
    pub candidate_box: gtk::Box,
    pub keyboard_revealer: gtk::Revealer,
    pub keyboard_box: gtk::Box,
    pub keyboard_button: gtk::ToggleButton,
    pub theme_button: gtk::Button,
    pub layout_button: gtk::Button,
    pub find_revealer: gtk::Revealer,
    pub find_entry: gtk::SearchEntry,
    pub replace_entry: gtk::Entry,
    pub find_count: gtk::Label,
    pub status_dot: gtk::Label,
    pub status_text: gtk::Label,
    pub status_position: gtk::Label,
    pub status_zoom: gtk::Label,
    pub toast_revealer: gtk::Revealer,
    pub toast_label: gtk::Label,
    pub active_editor: Rc<RefCell<gtk::TextView>>,
}

pub struct Shared {
    pub app: RefCell<App>,
    pub ui: Ui,
    pub providers: Providers,
    /// A programmatic change to a buffer is in progress.
    pub applying: Cell<bool>,
    /// Controls are being synchronised with the state.
    pub updating_controls: Cell<bool>,
    pub polling: Cell<bool>,
    pub toast_generation: Cell<u64>,
    pub last_candidates: RefCell<(Option<WordSpan>, Vec<Candidate>, bool)>,
}

pub type SharedRef = Rc<Shared>;

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

pub fn build(application: &gtk::Application) -> Option<SharedRef> {
    let providers = Providers::install();
    let paths = match Paths::discover() {
        Ok(paths) => paths,
        Err(error) => {
            eprintln!("Universal Transliterator cannot initialize local storage: {error}");
            return None;
        }
    };
    let settings = ut_storage::load_settings(&paths.settings_file()).unwrap_or_default();
    let learned = if settings.learning_enabled {
        ut_storage::load_learned(&paths.learned_file()).unwrap_or_default()
    } else {
        ut_storage::Learned::default()
    };
    let converter = Converter::new(paths.rime_user_dir(), learned, true);
    let language = language(&settings.language)
        .or_else(|| language(DEFAULT_LANGUAGE))
        .unwrap_or(&LANGUAGES[0]);
    let theme = Theme::from_setting(&settings.theme);
    let ui = build_widgets(application, &settings);
    let shared = Rc::new(Shared {
        app: RefCell::new(App {
            settings,
            paths,
            converter,
            language,
            current_path: None,
            saved_output: String::new(),
            auto_output: String::new(),
            rendered: Rendered::default(),
            candidate_span: None,
            candidate_list: Vec::new(),
            generation: 0,
            session_dirty: false,
            theme,
            restoring: false,
        }),
        ui,
        providers,
        applying: Cell::new(false),
        updating_controls: Cell::new(false),
        polling: Cell::new(false),
        toast_generation: Cell::new(0),
        last_candidates: RefCell::new((None, Vec::new(), false)),
    });
    create_tags(&shared);
    apply_theme(&shared, theme);
    connect_signals(&shared);
    install_actions(application, &shared);
    restore_session(&shared);
    let language_id = shared.app.borrow().language.id;
    set_language(&shared, language_id, false);
    schedule_session_snapshots(&shared);
    {
        let settings = shared.app.borrow().settings.clone();
        if settings.window_maximized {
            shared.ui.window.maximize();
        }
    }
    shared.ui.window.present();
    shared.ui.roman.grab_focus();
    Some(shared)
}

fn icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    let button = gtk::Button::builder()
        .icon_name(icon)
        .tooltip_text(tooltip)
        .build();
    button.add_css_class("flat");
    button
}

fn panel_header(
    title: &gtk::Label,
    note: Option<&gtk::Label>,
    trailing: &[&gtk::Widget],
) -> gtk::Box {
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    header.add_css_class("panel-header");
    title.add_css_class("panel-title");
    title.set_xalign(0.0);
    header.append(title);
    if let Some(note) = note {
        note.add_css_class("panel-note");
        note.set_xalign(0.0);
        note.set_hexpand(true);
        note.set_ellipsize(pango::EllipsizeMode::End);
        header.append(note);
    } else {
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        header.append(&spacer);
    }
    for widget in trailing {
        header.append(*widget);
    }
    header
}

fn editor(css: &str) -> gtk::TextView {
    let view = gtk::TextView::builder()
        .wrap_mode(gtk::WrapMode::WordChar)
        .left_margin(18)
        .right_margin(18)
        .top_margin(14)
        .bottom_margin(14)
        .pixels_below_lines(4)
        .vexpand(true)
        .hexpand(true)
        .accepts_tab(false)
        .build();
    view.add_css_class(css);
    view.buffer().set_enable_undo(true);
    view
}

fn build_widgets(application: &gtk::Application, settings: &Settings) -> Ui {
    let window = gtk::ApplicationWindow::builder()
        .application(application)
        .title("Universal Transliterator")
        .icon_name(APP_ID)
        .default_width(settings.window_width)
        .default_height(settings.window_height)
        .build();

    // --- Header bar ---
    let header = gtk::HeaderBar::new();
    let new_button = icon_button("document-new-symbolic", "New document (Ctrl+N)");
    new_button.set_action_name(Some("app.new"));
    let open_button = icon_button("document-open-symbolic", "Open a text file (Ctrl+O)");
    open_button.set_action_name(Some("app.open"));
    let save_button = icon_button("document-save-symbolic", "Save the output (Ctrl+S)");
    save_button.set_action_name(Some("app.save"));
    header.pack_start(&new_button);
    header.pack_start(&open_button);
    header.pack_start(&save_button);

    let language_native = gtk::Label::new(None);
    language_native.add_css_class("native");
    let language_engine = gtk::Label::new(None);
    language_engine.add_css_class("engine");
    let language_content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    language_content.append(&language_native);
    language_content.append(&language_engine);
    language_content.append(&gtk::Image::from_icon_name("pan-down-symbolic"));
    let language_search = gtk::SearchEntry::builder()
        .placeholder_text(format!("Search {} languages…", LANGUAGES.len()))
        .build();
    let language_list = gtk::ListBox::new();
    language_list.set_selection_mode(gtk::SelectionMode::None);
    language_list.set_activate_on_single_click(true);
    let language_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .min_content_height(440)
        .max_content_height(520)
        .min_content_width(380)
        .child(&language_list)
        .build();
    let language_body = gtk::Box::new(gtk::Orientation::Vertical, 6);
    language_body.append(&language_search);
    language_body.append(&language_scroll);
    let language_popover = gtk::Popover::new();
    language_popover.set_child(Some(&language_body));
    let language_button = gtk::MenuButton::builder()
        .popover(&language_popover)
        .tooltip_text("Choose the output language (Ctrl+L)")
        .build();
    language_button.set_child(Some(&language_content));
    language_button.add_css_class("language-button");

    let ai_button = gtk::ToggleButton::with_label("AI");
    ai_button.set_tooltip_text(Some(
        "AI4Bharat IndicXlit: natural phonetic typing with alternatives",
    ));
    let exact_button = gtk::ToggleButton::with_label("Exact");
    exact_button.set_tooltip_text(Some(
        "Rule-based ITRANS / IAST: precise, case-sensitive spelling (Ctrl+M toggles)",
    ));
    exact_button.set_group(Some(&ai_button));
    let mode_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    mode_box.add_css_class("linked");
    mode_box.add_css_class("segmented");
    mode_box.append(&ai_button);
    mode_box.append(&exact_button);
    let chinese_variant = gtk::DropDown::from_strings(&["简体 Simplified", "繁體 Traditional"]);
    chinese_variant.set_tooltip_text(Some("Chinese characters to produce"));
    let japanese_mode = gtk::DropDown::from_strings(&["ひらがな Hiragana", "カタカナ Katakana"]);
    japanese_mode.set_tooltip_text(Some("Kana used for words without kanji"));
    let title = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    title.append(&language_button);
    title.append(&mode_box);
    title.append(&chinese_variant);
    title.append(&japanese_mode);
    header.set_title_widget(Some(&title));

    let menu = build_menu();
    let menu_button = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .menu_model(&menu)
        .tooltip_text("Menu")
        .build();
    menu_button.add_css_class("flat");
    let theme_button = icon_button(
        "weather-clear-night-symbolic",
        "Switch light / dark theme (Ctrl+Shift+T)",
    );
    theme_button.set_action_name(Some("app.toggle-theme"));
    let layout_button = icon_button(
        "view-dual-symbolic",
        "Side by side / stacked panes (Ctrl+Shift+V)",
    );
    layout_button.set_action_name(Some("app.toggle-layout"));
    let keyboard_button = gtk::ToggleButton::builder()
        .icon_name("input-keyboard-symbolic")
        .tooltip_text("On-screen keyboard (Ctrl+K)")
        .active(settings.show_keyboard)
        .build();
    keyboard_button.add_css_class("flat");
    let copy_button = gtk::Button::builder()
        .tooltip_text("Copy the output (Ctrl+Shift+C)")
        .action_name("app.copy-output")
        .build();
    let copy_content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    copy_content.append(&gtk::Image::from_icon_name("edit-copy-symbolic"));
    copy_content.append(&gtk::Label::new(Some("Copy")));
    copy_button.set_child(Some(&copy_content));
    copy_button.add_css_class("suggested-action");
    header.pack_end(&menu_button);
    header.pack_end(&theme_button);
    header.pack_end(&layout_button);
    header.pack_end(&keyboard_button);
    header.pack_end(&copy_button);
    window.set_titlebar(Some(&header));

    // --- Find bar ---
    let find_entry = gtk::SearchEntry::builder()
        .placeholder_text("Find in output")
        .hexpand(true)
        .build();
    let replace_entry = gtk::Entry::builder()
        .placeholder_text("Replace with")
        .hexpand(true)
        .build();
    let find_count = gtk::Label::new(None);
    find_count.add_css_class("dim");
    let find_previous = icon_button("go-up-symbolic", "Previous match (Shift+Enter)");
    find_previous.set_action_name(Some("app.find-previous"));
    let find_next = icon_button("go-down-symbolic", "Next match (Enter)");
    find_next.set_action_name(Some("app.find-next"));
    let replace_button = gtk::Button::with_label("Replace");
    replace_button.set_action_name(Some("app.replace-one"));
    let replace_all_button = gtk::Button::with_label("Replace all");
    replace_all_button.set_action_name(Some("app.replace-all"));
    let find_close = icon_button("window-close-symbolic", "Close (Esc)");
    find_close.set_action_name(Some("app.find-close"));
    let find_bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    find_bar.add_css_class("findbar");
    for widget in [
        find_entry.upcast_ref::<gtk::Widget>(),
        find_count.upcast_ref(),
        find_previous.upcast_ref(),
        find_next.upcast_ref(),
        replace_entry.upcast_ref(),
        replace_button.upcast_ref(),
        replace_all_button.upcast_ref(),
        find_close.upcast_ref(),
    ] {
        find_bar.append(widget);
    }
    let find_revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .child(&find_bar)
        .build();

    // --- Roman pane ---
    let roman = editor("roman-editor");
    let roman_placeholder = gtk::Label::new(None);
    roman_placeholder.add_css_class("placeholder");
    roman_placeholder.set_xalign(0.0);
    roman_placeholder.set_yalign(0.0);
    roman_placeholder.set_halign(gtk::Align::Start);
    roman_placeholder.set_valign(gtk::Align::Start);
    roman_placeholder.set_can_target(false);
    roman_placeholder.set_wrap(true);
    let roman_scroll = gtk::ScrolledWindow::builder()
        .child(&roman)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    let roman_overlay = gtk::Overlay::new();
    roman_overlay.set_child(Some(&roman_scroll));
    roman_overlay.add_overlay(&roman_placeholder);
    let roman_title = gtk::Label::new(Some("Roman input"));
    let roman_note = gtk::Label::new(None);
    let roman_clear = icon_button("edit-clear-all-symbolic", "Clear both panes (Ctrl+Shift+N)");
    roman_clear.set_action_name(Some("app.clear"));
    let roman_panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    roman_panel.add_css_class("panel");
    roman_panel.append(&panel_header(
        &roman_title,
        Some(&roman_note),
        &[roman_clear.upcast_ref()],
    ));
    roman_panel.append(&roman_overlay);

    // --- Output pane ---
    let output = editor("output-editor");
    let output_scroll = gtk::ScrolledWindow::builder()
        .child(&output)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    let output_title = gtk::Label::new(Some("Output"));
    let output_count = gtk::Label::new(None);
    output_count.add_css_class("panel-count");
    let output_convert = icon_button(
        "media-playlist-shuffle-symbolic",
        "Convert script / romanize (Ctrl+Shift+K)",
    );
    output_convert.set_action_name(Some("app.convert-script"));
    let output_find = icon_button("edit-find-symbolic", "Find and replace (Ctrl+F)");
    output_find.set_action_name(Some("app.find"));
    let output_copy = icon_button("edit-copy-symbolic", "Copy the output (Ctrl+Shift+C)");
    output_copy.set_action_name(Some("app.copy-output"));
    let output_panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    output_panel.add_css_class("panel");
    output_panel.append(&panel_header(
        &output_title,
        None,
        &[
            output_count.upcast_ref(),
            output_convert.upcast_ref(),
            output_find.upcast_ref(),
            output_copy.upcast_ref(),
        ],
    ));
    output_panel.append(&output_scroll);

    let paned = gtk::Paned::new(if settings.vertical_layout {
        gtk::Orientation::Vertical
    } else {
        gtk::Orientation::Horizontal
    });
    paned.add_css_class("workspace");
    paned.set_start_child(Some(&roman_panel));
    paned.set_end_child(Some(&output_panel));
    paned.set_resize_start_child(true);
    paned.set_resize_end_child(true);
    paned.set_shrink_start_child(false);
    paned.set_shrink_end_child(false);
    paned.set_vexpand(true);
    if settings.pane_position > 0 {
        paned.set_position(settings.pane_position);
    } else if settings.vertical_layout {
        paned.set_position(settings.window_height / 2 - 120);
    } else {
        paned.set_position(settings.window_width / 2 - 14);
    }

    // --- Alternatives bar ---
    let candidate_word = gtk::Label::new(None);
    candidate_word.add_css_class("candidate-word");
    let candidate_box = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    let candidate_scroll = gtk::ScrolledWindow::builder()
        .vscrollbar_policy(gtk::PolicyType::Never)
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&candidate_box)
        .hexpand(true)
        .build();
    let candidate_bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    candidate_bar.add_css_class("candidate-bar");
    candidate_bar.append(&candidate_word);
    candidate_bar.append(&candidate_scroll);

    // --- Keyboard ---
    let keyboard_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    keyboard_box.add_css_class("keyboard");
    let keyboard_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .max_content_height(260)
        .propagate_natural_height(true)
        .child(&keyboard_box)
        .build();
    let keyboard_revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::SlideUp)
        .reveal_child(settings.show_keyboard)
        .child(&keyboard_scroll)
        .build();

    // --- Status bar ---
    let status_dot = gtk::Label::new(Some("●"));
    status_dot.add_css_class("status-dot");
    let status_text = gtk::Label::new(Some("Ready"));
    status_text.set_xalign(0.0);
    status_text.set_hexpand(true);
    status_text.set_ellipsize(pango::EllipsizeMode::End);
    let status_position = gtk::Label::new(None);
    let status_zoom = gtk::Label::new(None);
    status_zoom.set_tooltip_text(Some("Text size — Ctrl+= / Ctrl+- / Ctrl+0"));
    let status = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    status.add_css_class("statusbar");
    status.append(&status_dot);
    status.append(&status_text);
    status.append(&status_position);
    status.append(&status_zoom);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&find_revealer);
    content.append(&paned);
    content.append(&candidate_bar);
    content.append(&keyboard_revealer);
    content.append(&status);

    let toast_label = gtk::Label::new(None);
    toast_label.set_wrap(true);
    toast_label.set_max_width_chars(80);
    let toast_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    toast_box.add_css_class("toast");
    toast_box.append(&toast_label);
    let toast_revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::Crossfade)
        .child(&toast_box)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::End)
        .can_target(false)
        .build();
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&content));
    overlay.add_overlay(&toast_revealer);
    window.set_child(Some(&overlay));

    Ui {
        window,
        roman: roman.clone(),
        output,
        roman_placeholder,
        roman_note,
        output_title,
        output_count,
        paned,
        language_button,
        language_popover,
        language_native,
        language_engine,
        language_list,
        language_search,
        mode_box,
        ai_button,
        exact_button,
        chinese_variant,
        japanese_mode,
        candidate_word,
        candidate_box,
        keyboard_revealer,
        keyboard_box,
        keyboard_button,
        theme_button,
        layout_button,
        find_revealer,
        find_entry,
        replace_entry,
        find_count,
        status_dot,
        status_text,
        status_position,
        status_zoom,
        toast_revealer,
        toast_label,
        active_editor: Rc::new(RefCell::new(roman)),
    }
}

fn build_menu() -> gio::Menu {
    let menu = gio::Menu::new();
    let file = gio::Menu::new();
    file.append(Some("New"), Some("app.new"));
    file.append(Some("Open…"), Some("app.open"));
    file.append(Some("Save"), Some("app.save"));
    file.append(Some("Save as…"), Some("app.save-as"));
    file.append(
        Some("Export Word document (.docx)…"),
        Some("app.export-docx"),
    );
    file.append(Some("Save Roman input as…"), Some("app.export-roman"));
    menu.append_section(None, &file);
    let edit = gio::Menu::new();
    edit.append(Some("Copy output"), Some("app.copy-output"));
    edit.append(Some("Find and replace"), Some("app.find"));
    edit.append(
        Some("Convert script / romanize…"),
        Some("app.convert-script"),
    );
    edit.append(Some("Clear both panes"), Some("app.clear"));
    menu.append_section(None, &edit);
    let view = gio::Menu::new();
    view.append(Some("Zoom in"), Some("app.zoom-in"));
    view.append(Some("Zoom out"), Some("app.zoom-out"));
    view.append(Some("Reset zoom"), Some("app.zoom-reset"));
    menu.append_section(None, &view);
    let app = gio::Menu::new();
    app.append(Some("Preferences"), Some("app.preferences"));
    app.append(Some("Keyboard shortcuts"), Some("app.shortcuts"));
    app.append(Some("About Universal Transliterator"), Some("app.about"));
    app.append(Some("Quit"), Some("app.quit"));
    menu.append_section(None, &app);
    menu
}

fn create_tags(shared: &SharedRef) {
    for view in [&shared.ui.roman, &shared.ui.output] {
        let buffer = view.buffer();
        buffer.create_tag(Some("current-word"), &[]);
        buffer.create_tag(Some("search-match"), &[]);
    }
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

pub fn text_of(view: &gtk::TextView) -> String {
    let buffer = view.buffer();
    buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), true)
        .to_string()
}

/// Replaces a buffer's text programmatically (not reported as a manual edit).
fn set_text_quietly(shared: &SharedRef, view: &gtk::TextView, text: &str) {
    shared.applying.set(true);
    view.buffer().set_text(text);
    shared.applying.set(false);
}

pub fn toast(shared: &SharedRef, message: &str) {
    let generation = shared.toast_generation.get() + 1;
    shared.toast_generation.set(generation);
    shared.ui.toast_label.set_text(message);
    shared.ui.toast_revealer.set_reveal_child(true);
    let weak = Rc::downgrade(shared);
    glib::timeout_add_local_once(Duration::from_millis(2600), move || {
        if let Some(shared) = weak.upgrade() {
            if shared.toast_generation.get() == generation {
                shared.ui.toast_revealer.set_reveal_child(false);
            }
        }
    });
}

pub fn is_dirty(shared: &SharedRef) -> bool {
    let output = text_of(&shared.ui.output);
    let app = shared.app.borrow();
    output != app.saved_output && !(output.is_empty() && app.current_path.is_none())
}

pub fn update_title(shared: &SharedRef) {
    let dirty = is_dirty(shared);
    let name = shared.app.borrow().document_name();
    let marker = if dirty { " •" } else { "" };
    shared
        .ui
        .window
        .set_title(Some(&format!("{name}{marker} — Universal Transliterator")));
}

fn count_words(text: &str) -> (usize, usize) {
    let mut words = 0;
    let mut in_word = false;
    for ch in text.chars() {
        let cjk = matches!(ch as u32, 0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF);
        if cjk {
            words += 1;
            in_word = false;
        } else if ch.is_whitespace()
            || ch.is_ascii_punctuation()
            || matches!(ch, '।' | '॥' | '，' | '。')
        {
            in_word = false;
        } else if !in_word {
            words += 1;
            in_word = true;
        }
    }
    (words, text.chars().filter(|ch| *ch != '\n').count())
}

pub fn update_counts(shared: &SharedRef) {
    let output = text_of(&shared.ui.output);
    let (words, chars) = count_words(&output);
    shared.ui.output_count.set_text(&format!(
        "{words} word{} · {chars} char{}",
        if words == 1 { "" } else { "s" },
        if chars == 1 { "" } else { "s" }
    ));
    update_position(shared);
}

fn update_position(shared: &SharedRef) {
    let view = shared.ui.active_editor.borrow().clone();
    let buffer = view.buffer();
    let iter = buffer.iter_at_offset(buffer.cursor_position());
    let pane = if view == shared.ui.roman {
        "Roman"
    } else {
        "Output"
    };
    shared.ui.status_position.set_text(&format!(
        "{pane} · Ln {}, Col {}",
        iter.line() + 1,
        iter.line_offset() + 1
    ));
}

pub fn update_status(shared: &SharedRef) {
    let (kind, message, pending) = {
        let app = shared.app.borrow();
        let options = app.options();
        let (kind, message) = app.converter.status(app.language, &options);
        (kind, message, app.rendered.pending)
    };
    let (class, text) = if pending > 0 && kind == StatusKind::Ready {
        (
            "busy",
            format!(
                "Transliterating {pending} word{}…",
                if pending == 1 { "" } else { "s" }
            ),
        )
    } else {
        (
            match kind {
                StatusKind::Ready => "ready",
                StatusKind::Busy => "busy",
                StatusKind::Warning => "warning",
            },
            message,
        )
    };
    shared.ui.status_dot.set_css_classes(&["status-dot", class]);
    shared.ui.status_text.set_text(&text);
    let size = shared.app.borrow().settings.font_size;
    shared
        .ui
        .status_zoom
        .set_text(&format!("{}%", (u32::from(size) * 100 + 8) / 16));
}

// ---------------------------------------------------------------------------
// Rendering and merging
// ---------------------------------------------------------------------------

pub fn schedule_render(shared: &SharedRef, delay_ms: u64) {
    let generation = {
        let mut app = shared.app.borrow_mut();
        app.generation = app.generation.wrapping_add(1);
        app.generation
    };
    let weak = Rc::downgrade(shared);
    glib::timeout_add_local_once(Duration::from_millis(delay_ms), move || {
        if let Some(shared) = weak.upgrade() {
            if shared.app.borrow().generation == generation {
                render_now(&shared);
            }
        }
    });
}

pub fn render_now(shared: &SharedRef) {
    let mut roman = text_of(&shared.ui.roman);
    if roman.len() > MAX_ROMAN_BYTES {
        let mut cut = MAX_ROMAN_BYTES;
        while !roman.is_char_boundary(cut) {
            cut -= 1;
        }
        roman.truncate(cut);
        toast(shared, "Only the first 1 MB of Roman input is converted.");
    }
    let rendered = {
        let mut app = shared.app.borrow_mut();
        let language = app.language;
        let options = app.options();
        app.converter.render(language, &roman, &options)
    };
    let pending = rendered.pending;
    let words = rendered.words.len();
    let show = {
        let mut app = shared.app.borrow_mut();
        app.rendered = rendered;
        let show = pending == 0
            || (!app.restoring && (pending <= MAX_PENDING_TO_SHOW || pending * 2 < words));
        if show && pending == 0 {
            app.restoring = false;
        }
        show
    };
    if show {
        apply_output(shared);
    }
    if shared.app.borrow().converter.has_pending() {
        ensure_polling(shared);
    }
    refresh_candidates(shared, true);
    update_status(shared);
    shared.app.borrow_mut().session_dirty = true;
}

/// Brings the output pane up to date, keeping manual edits (three-way merge).
fn apply_output(shared: &SharedRef) {
    let current = text_of(&shared.ui.output);
    let merged = {
        let mut app = shared.app.borrow_mut();
        let new_auto = app.rendered.text.clone();
        let merged = if current == app.auto_output {
            new_auto.clone()
        } else {
            ut_text::merge3(&app.auto_output, &current, &new_auto)
        };
        app.auto_output = new_auto;
        merged
    };
    if let Some((start, end, replacement)) = ut_text::minimal_edit(&current, &merged) {
        let buffer = shared.ui.output.buffer();
        shared.applying.set(true);
        buffer.begin_user_action();
        let mut from = buffer.iter_at_offset(start as i32);
        let mut to = buffer.iter_at_offset(end as i32);
        buffer.delete(&mut from, &mut to);
        buffer.insert(&mut from, &replacement);
        buffer.end_user_action();
        shared.applying.set(false);
    }
    update_title(shared);
    update_counts(shared);
}

fn ensure_polling(shared: &SharedRef) {
    if shared.polling.get() {
        return;
    }
    shared.polling.set(true);
    let weak = Rc::downgrade(shared);
    glib::timeout_add_local(Duration::from_millis(30), move || {
        let Some(shared) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        let (changed, pending) = {
            let mut app = shared.app.borrow_mut();
            let changed = app.converter.poll();
            (changed, app.converter.has_pending())
        };
        if changed {
            render_now(&shared);
        }
        if pending {
            glib::ControlFlow::Continue
        } else {
            shared.polling.set(false);
            update_status(&shared);
            glib::ControlFlow::Break
        }
    });
}

// ---------------------------------------------------------------------------
// Alternatives bar
// ---------------------------------------------------------------------------

fn current_span(shared: &SharedRef) -> Option<WordSpan> {
    let output_focused = shared.ui.output.has_focus();
    let roman_cursor = shared.ui.roman.buffer().cursor_position().max(0) as usize;
    let output_cursor = shared.ui.output.buffer().cursor_position().max(0) as usize;
    let output_matches =
        output_focused && text_of(&shared.ui.output) == shared.app.borrow().auto_output;
    let app = shared.app.borrow();
    if output_focused {
        if !output_matches {
            return None;
        }
        return app
            .rendered
            .words
            .iter()
            .find(|span| span.output.start <= output_cursor && output_cursor <= span.output.end)
            .cloned();
    }
    app.rendered
        .words
        .iter()
        .find(|span| span.roman.start <= roman_cursor && roman_cursor <= span.roman.end)
        .cloned()
        .or_else(|| {
            // Nothing under the cursor: offer the last word before it.
            app.rendered
                .words
                .iter()
                .rev()
                .find(|span| span.roman.end <= roman_cursor)
                .cloned()
        })
}

pub fn refresh_candidates(shared: &SharedRef, force: bool) {
    if shared.applying.get() {
        return;
    }
    let span = current_span(shared);
    let (list, pending) = match &span {
        Some(span) => {
            let mut app = shared.app.borrow_mut();
            let language = app.language;
            let options = app.options();
            match app.converter.alternatives(language, &options, span) {
                Some(list) => (list, false),
                None => (Vec::new(), true),
            }
        }
        None => (Vec::new(), false),
    };
    {
        let last = shared.last_candidates.borrow();
        if !force && last.0 == span && last.1 == list && last.2 == pending {
            return;
        }
    }
    *shared.last_candidates.borrow_mut() = (span.clone(), list.clone(), pending);
    {
        let mut app = shared.app.borrow_mut();
        app.candidate_span = span.clone();
        app.candidate_list = list.clone();
    }
    highlight_span(shared, span.as_ref());

    let container = &shared.ui.candidate_box;
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
    let Some(span) = span else {
        let hint = if shared.ui.output.has_focus()
            && text_of(&shared.ui.output) != shared.app.borrow().auto_output
        {
            "Output edited by hand — click a word in the Roman pane for alternatives"
        } else {
            "Alternatives appear here — click any word, or press Alt+1…9"
        };
        shared
            .ui
            .candidate_word
            .set_markup(&format!("<span>{}</span>", glib::markup_escape_text(hint)));
        return;
    };
    shared.ui.candidate_word.set_markup(&format!(
        "<span weight=\"bold\" foreground=\"#8d96a0\">{}</span> →",
        glib::markup_escape_text(&span.word)
    ));
    if pending {
        let label = gtk::Label::new(Some("asking the AI model…"));
        label.add_css_class("dim");
        container.append(&label);
        return;
    }
    for (index, candidate) in list.iter().enumerate() {
        let number = gtk::Label::new(Some(&(index + 1).to_string()));
        number.add_css_class("number");
        let text = gtk::Label::new(Some(&candidate.text));
        text.add_css_class("text");
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        content.append(&number);
        content.append(&text);
        if !candidate.hint.is_empty() {
            let hint = gtk::Label::new(Some(candidate.hint));
            hint.add_css_class("hint");
            content.append(&hint);
        }
        let button = gtk::Button::new();
        button.set_child(Some(&content));
        button.add_css_class("candidate");
        button.set_focus_on_click(false);
        if candidate.text == span.text {
            button.add_css_class("current");
        }
        button.set_tooltip_text(Some(&format!(
            "Use “{}” for “{}” (Alt+{})",
            candidate.text,
            span.word,
            index + 1
        )));
        let weak = Rc::downgrade(shared);
        button.connect_clicked(move |_| {
            if let Some(shared) = weak.upgrade() {
                choose_candidate(&shared, index);
            }
        });
        container.append(&button);
    }
}

fn highlight_span(shared: &SharedRef, span: Option<&WordSpan>) {
    for view in [&shared.ui.roman, &shared.ui.output] {
        let buffer = view.buffer();
        buffer.remove_tag_by_name("current-word", &buffer.start_iter(), &buffer.end_iter());
    }
    let Some(span) = span else {
        return;
    };
    let roman = shared.ui.roman.buffer();
    roman.apply_tag_by_name(
        "current-word",
        &roman.iter_at_offset(span.roman.start as i32),
        &roman.iter_at_offset(span.roman.end as i32),
    );
    let matches = text_of(&shared.ui.output) == shared.app.borrow().auto_output;
    if matches {
        let output = shared.ui.output.buffer();
        output.apply_tag_by_name(
            "current-word",
            &output.iter_at_offset(span.output.start as i32),
            &output.iter_at_offset(span.output.end as i32),
        );
    }
}

pub fn choose_candidate(shared: &SharedRef, index: usize) {
    let (span, text) = {
        let app = shared.app.borrow();
        match (app.candidate_span.clone(), app.candidate_list.get(index)) {
            (Some(span), Some(candidate)) => (span, candidate.text.clone()),
            _ => return,
        }
    };
    let learning = {
        let mut app = shared.app.borrow_mut();
        let language = app.language;
        let options = app.options();
        app.converter.choose(language, &options, &span, &text);
        app.save_learned();
        app.settings.learning_enabled
    };
    render_now(shared);
    toast(
        shared,
        &if learning {
            format!("{} → {}  ·  remembered", span.word, text)
        } else {
            format!("{} → {}", span.word, text)
        },
    );
}

// ---------------------------------------------------------------------------
// Language switching
// ---------------------------------------------------------------------------

pub fn set_language(shared: &SharedRef, id: &str, announce: bool) {
    let Some(language) = language(id) else {
        return;
    };
    {
        let mut app = shared.app.borrow_mut();
        app.language = language;
        app.settings.language = language.id.to_owned();
        app.settings
            .recent_languages
            .retain(|recent| recent != language.id);
        app.settings
            .recent_languages
            .insert(0, language.id.to_owned());
        app.settings.recent_languages.truncate(6);
        app.converter.warm_up(language);
        app.save_settings();
    }
    sync_language_controls(shared);
    let needs_preparation = {
        let app = shared.app.borrow();
        app.converter
            .needs_preparation(app.language, &app.options())
    };
    if needs_preparation {
        shared
            .ui
            .status_dot
            .set_css_classes(&["status-dot", "busy"]);
        shared
            .ui
            .status_text
            .set_text("Loading offline dictionaries…");
        let weak = Rc::downgrade(shared);
        // Let the status repaint before the (one-time) dictionary load.
        glib::timeout_add_local_once(Duration::from_millis(30), move || {
            if let Some(shared) = weak.upgrade() {
                {
                    let mut app = shared.app.borrow_mut();
                    let language = app.language;
                    let options = app.options();
                    app.converter.prepare(language, &options);
                }
                render_now(&shared);
            }
        });
    } else {
        render_now(shared);
    }
    if announce {
        toast(shared, &format!("{} · {}", language.native, language.name));
    }
}

pub fn sync_language_controls(shared: &SharedRef) {
    let (language, options, settings) = {
        let app = shared.app.borrow();
        (app.language, app.options(), app.settings.clone())
    };
    shared.updating_controls.set(true);
    let ui = &shared.ui;
    ui.language_native.set_text(language.native);
    ui.language_engine.set_text(language.name);
    ui.mode_box.set_visible(language.supports_exact());
    ui.ai_button.set_active(!options.exact);
    ui.exact_button.set_active(options.exact);
    ui.chinese_variant
        .set_visible(language.engine == Engine::Rime);
    ui.chinese_variant
        .set_selected(u32::from(settings.chinese_variant == "traditional"));
    ui.japanese_mode.set_visible(language.engine == Engine::Kkc);
    ui.japanese_mode
        .set_selected(u32::from(settings.japanese_mode == "katakana"));
    ui.output_title.set_text(&format!(
        "\u{200E}{} · {}",
        language.native,
        language.engine_label(options.exact)
    ));
    let note = if options.exact {
        EXACT_NOTE
    } else {
        language.note
    };
    ui.roman_note.set_text(note);
    ui.roman_note.set_tooltip_text(Some(note));
    let example = if language.example.is_empty() {
        "Use the on-screen keyboard (Ctrl+K) or your system layout.".to_owned()
    } else {
        format!("Start typing in Roman letters…\ne.g.  {}", language.example)
    };
    ui.roman_placeholder.set_text(&example);
    ui.output.set_direction(if language.rtl {
        gtk::TextDirection::Rtl
    } else {
        gtk::TextDirection::Ltr
    });
    shared
        .providers
        .apply_editor_fonts(language, options.traditional, settings.font_size);
    keyboard::render(shared, language.keyboard);
    shared.updating_controls.set(false);
    update_status(shared);
}

fn set_exact(shared: &SharedRef, exact: bool) {
    {
        let mut app = shared.app.borrow_mut();
        let id = app.language.id.to_owned();
        app.settings.exact_languages.retain(|value| value != &id);
        if exact {
            app.settings.exact_languages.push(id);
        }
        app.save_settings();
    }
    sync_language_controls(shared);
    render_now(shared);
}

fn rebuild_language_list(shared: &SharedRef) {
    let list = &shared.ui.language_list;
    list.remove_all();
    let (current, recent) = {
        let app = shared.app.borrow();
        (app.language.id, app.settings.recent_languages.clone())
    };
    let header = |text: &str| {
        let label = gtk::Label::new(Some(text));
        label.set_xalign(0.0);
        let row = gtk::ListBoxRow::new();
        row.set_child(Some(&label));
        row.set_activatable(false);
        row.set_selectable(false);
        row.add_css_class("group-header");
        row
    };
    let language_row = |language: &LanguageInfo, name: String| {
        let native = gtk::Label::new(Some(language.native));
        native.add_css_class("native");
        native.set_xalign(0.0);
        let english = gtk::Label::new(Some(language.name));
        english.add_css_class("name");
        english.set_xalign(0.0);
        english.set_hexpand(true);
        english.set_ellipsize(pango::EllipsizeMode::End);
        let (badge_text, badge_class) = match language.engine {
            Engine::Neural { .. } => ("AI", "ai"),
            Engine::Rime | Engine::Kkc => ("DICT", "dictionary"),
            Engine::Rules => ("RULES", "rules"),
            Engine::Keyboard => ("KEYS", "keys"),
        };
        let badge = gtk::Label::new(Some(badge_text));
        badge.add_css_class("badge");
        badge.add_css_class(badge_class);
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        content.add_css_class("language-row");
        content.append(&native);
        content.append(&english);
        content.append(&badge);
        let row = gtk::ListBoxRow::new();
        row.set_child(Some(&content));
        row.set_widget_name(&name);
        if language.id == current {
            content.add_css_class("current");
        }
        row
    };
    let recent: Vec<&LanguageInfo> = recent
        .iter()
        .filter_map(|id| language(id))
        .take(5)
        .collect();
    if !recent.is_empty() {
        list.append(&header("RECENT"));
        for language in recent {
            list.append(&language_row(language, format!("recent:{}", language.id)));
        }
    }
    for group in Group::ALL {
        list.append(&header(&group.label().to_uppercase()));
        for language in LANGUAGES.iter().filter(|language| language.group == group) {
            list.append(&language_row(language, language.id.to_owned()));
        }
    }
}

fn language_matches(name: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    if name.is_empty() || name.starts_with("recent:") {
        return false;
    }
    let Some(language) = language(name) else {
        return false;
    };
    let query = query.to_lowercase();
    language.name.to_lowercase().contains(&query)
        || language.native.to_lowercase().contains(&query)
        || language.id == query
        || format!("{:?}", language.group)
            .to_lowercase()
            .contains(&query)
}

fn activate_language_row(shared: &SharedRef, row: &gtk::ListBoxRow) {
    let name = row.widget_name().to_string();
    let id = name.strip_prefix("recent:").unwrap_or(&name).to_owned();
    if language(&id).is_none() {
        return;
    }
    shared.ui.language_popover.popdown();
    set_language(shared, &id, false);
    shared.ui.roman.grab_focus();
}

fn previous_language(shared: &SharedRef) {
    let previous = {
        let app = shared.app.borrow();
        app.settings.recent_languages.get(1).cloned()
    };
    if let Some(id) = previous {
        set_language(shared, &id, true);
    }
}

// ---------------------------------------------------------------------------
// Signals
// ---------------------------------------------------------------------------

fn connect_signals(shared: &SharedRef) {
    let ui = &shared.ui;

    for view in [&ui.roman, &ui.output] {
        let weak = Rc::downgrade(shared);
        let tracked = view.clone();
        view.connect_has_focus_notify(move |widget| {
            if let Some(shared) = weak.upgrade() {
                if widget.has_focus() {
                    *shared.ui.active_editor.borrow_mut() = tracked.clone();
                    update_position(&shared);
                    refresh_candidates(&shared, false);
                }
            }
        });
    }

    let weak = Rc::downgrade(shared);
    ui.roman.buffer().connect_changed(move |buffer| {
        let Some(shared) = weak.upgrade() else {
            return;
        };
        shared
            .ui
            .roman_placeholder
            .set_visible(buffer.char_count() == 0);
        if !shared.applying.get() {
            schedule_render(&shared, RENDER_DELAY_MS);
        }
    });
    let weak = Rc::downgrade(shared);
    ui.roman.buffer().connect_cursor_position_notify(move |_| {
        if let Some(shared) = weak.upgrade() {
            update_position(&shared);
            refresh_candidates(&shared, false);
        }
    });
    let weak = Rc::downgrade(shared);
    ui.output.buffer().connect_changed(move |_| {
        let Some(shared) = weak.upgrade() else {
            return;
        };
        if shared.applying.get() {
            return;
        }
        shared.app.borrow_mut().session_dirty = true;
        update_title(&shared);
        update_counts(&shared);
        refresh_candidates(&shared, true);
    });
    let weak = Rc::downgrade(shared);
    ui.output.buffer().connect_cursor_position_notify(move |_| {
        if let Some(shared) = weak.upgrade() {
            if shared.ui.output.has_focus() {
                update_position(&shared);
                refresh_candidates(&shared, false);
            }
        }
    });

    // Language popover.
    let weak = Rc::downgrade(shared);
    ui.language_popover.connect_show(move |_| {
        if let Some(shared) = weak.upgrade() {
            shared.ui.language_search.set_text("");
            rebuild_language_list(&shared);
            shared.ui.language_search.grab_focus();
        }
    });
    let search = ui.language_search.clone();
    ui.language_list
        .set_filter_func(move |row| language_matches(&row.widget_name(), search.text().trim()));
    let list = ui.language_list.clone();
    ui.language_search
        .connect_search_changed(move |_| list.invalidate_filter());
    let weak = Rc::downgrade(shared);
    ui.language_search.connect_activate(move |_| {
        let Some(shared) = weak.upgrade() else {
            return;
        };
        let mut child = shared.ui.language_list.first_child();
        while let Some(widget) = child {
            if let Ok(row) = widget.clone().downcast::<gtk::ListBoxRow>() {
                if row.is_child_visible() && row.is_activatable() {
                    activate_language_row(&shared, &row);
                    return;
                }
            }
            child = widget.next_sibling();
        }
    });
    let weak = Rc::downgrade(shared);
    ui.language_list.connect_row_activated(move |_, row| {
        if let Some(shared) = weak.upgrade() {
            activate_language_row(&shared, row);
        }
    });

    // Mode controls.
    let weak = Rc::downgrade(shared);
    ui.exact_button.connect_toggled(move |button| {
        if let Some(shared) = weak.upgrade() {
            if !shared.updating_controls.get() && button.is_active() {
                set_exact(&shared, true);
            }
        }
    });
    let weak = Rc::downgrade(shared);
    ui.ai_button.connect_toggled(move |button| {
        if let Some(shared) = weak.upgrade() {
            if !shared.updating_controls.get() && button.is_active() {
                set_exact(&shared, false);
            }
        }
    });
    let weak = Rc::downgrade(shared);
    ui.chinese_variant.connect_selected_notify(move |dropdown| {
        let Some(shared) = weak.upgrade() else {
            return;
        };
        if shared.updating_controls.get() {
            return;
        }
        {
            let mut app = shared.app.borrow_mut();
            app.settings.chinese_variant = if dropdown.selected() == 1 {
                "traditional"
            } else {
                "simplified"
            }
            .into();
            app.save_settings();
        }
        let id = shared.app.borrow().language.id;
        set_language(&shared, id, false);
    });
    let weak = Rc::downgrade(shared);
    ui.japanese_mode.connect_selected_notify(move |dropdown| {
        let Some(shared) = weak.upgrade() else {
            return;
        };
        if shared.updating_controls.get() {
            return;
        }
        {
            let mut app = shared.app.borrow_mut();
            app.settings.japanese_mode = if dropdown.selected() == 1 {
                "katakana"
            } else {
                "hiragana"
            }
            .into();
            app.save_settings();
        }
        let id = shared.app.borrow().language.id;
        set_language(&shared, id, false);
    });

    let weak = Rc::downgrade(shared);
    ui.keyboard_button.connect_toggled(move |button| {
        if let Some(shared) = weak.upgrade() {
            shared
                .ui
                .keyboard_revealer
                .set_reveal_child(button.is_active());
            let mut app = shared.app.borrow_mut();
            app.settings.show_keyboard = button.is_active();
            app.save_settings();
        }
    });

    // Find bar.
    let weak = Rc::downgrade(shared);
    ui.find_entry.connect_search_changed(move |_| {
        if let Some(shared) = weak.upgrade() {
            highlight_matches(&shared);
        }
    });
    let weak = Rc::downgrade(shared);
    ui.find_entry.connect_activate(move |_| {
        if let Some(shared) = weak.upgrade() {
            find(&shared, true);
        }
    });
    let weak = Rc::downgrade(shared);
    ui.find_entry.connect_stop_search(move |_| {
        if let Some(shared) = weak.upgrade() {
            close_find(&shared);
        }
    });
    let weak = Rc::downgrade(shared);
    ui.replace_entry.connect_activate(move |_| {
        if let Some(shared) = weak.upgrade() {
            replace_one(&shared);
        }
    });

    // Drag and drop of text files.
    let drop = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
    let weak = Rc::downgrade(shared);
    drop.connect_drop(move |_, value, _, _| {
        let Some(shared) = weak.upgrade() else {
            return false;
        };
        let Ok(files) = value.get::<gdk::FileList>() else {
            return false;
        };
        if let Some(path) = files.files().first().and_then(gio::File::path) {
            open_with_confirmation(&shared, path);
            return true;
        }
        false
    });
    ui.window.add_controller(drop);

    let weak = Rc::downgrade(shared);
    ui.window.connect_close_request(move |window| {
        let Some(shared) = weak.upgrade() else {
            return glib::Propagation::Proceed;
        };
        let autosave = shared.app.borrow().settings.autosave;
        if !autosave && is_dirty(&shared) {
            let shared_for_close = shared.clone();
            let window = window.clone();
            dialogs::confirm_unsaved(&shared, move |proceed| {
                if proceed {
                    finish_close(&shared_for_close);
                    window.destroy();
                }
            });
            return glib::Propagation::Stop;
        }
        finish_close(&shared);
        glib::Propagation::Proceed
    });
}

fn finish_close(shared: &SharedRef) {
    let (width, height) = shared.ui.window.default_size();
    let maximized = shared.ui.window.is_maximized();
    let position = shared.ui.paned.position();
    {
        let mut app = shared.app.borrow_mut();
        if !maximized && width > 0 && height > 0 {
            app.settings.window_width = width;
            app.settings.window_height = height;
        }
        app.settings.window_maximized = maximized;
        app.settings.pane_position = position;
        app.save_settings();
        app.save_learned();
    }
    save_session_now(shared);
}

// ---------------------------------------------------------------------------
// Actions and shortcuts
// ---------------------------------------------------------------------------

fn add_action<F: Fn(&SharedRef) + 'static>(
    application: &gtk::Application,
    shared: &SharedRef,
    name: &str,
    accels: &[&str],
    handler: F,
) {
    let action = gio::SimpleAction::new(name, None);
    let weak = Rc::downgrade(shared);
    action.connect_activate(move |_, _| {
        if let Some(shared) = weak.upgrade() {
            handler(&shared);
        }
    });
    application.add_action(&action);
    if !accels.is_empty() {
        application.set_accels_for_action(&format!("app.{name}"), accels);
    }
}

fn install_actions(application: &gtk::Application, shared: &SharedRef) {
    add_action(application, shared, "new", &["<Control>n"], |shared| {
        let target = shared.clone();
        dialogs::confirm_unsaved(shared, move |proceed| {
            if proceed {
                new_document(&target);
            }
        });
    });
    add_action(
        application,
        shared,
        "clear",
        &["<Control><Shift>n"],
        |shared| {
            let target = shared.clone();
            dialogs::confirm_unsaved(shared, move |proceed| {
                if proceed {
                    new_document(&target);
                }
            });
        },
    );
    add_action(application, shared, "open", &["<Control>o"], |shared| {
        let target = shared.clone();
        dialogs::confirm_unsaved(shared, move |proceed| {
            if proceed {
                dialogs::open_file(&target);
            }
        });
    });
    add_action(application, shared, "save", &["<Control>s"], |shared| {
        let path = shared.app.borrow().current_path.clone();
        match path {
            Some(path) => {
                save_to(shared, &path);
            }
            None => dialogs::save_as(shared, |_| {}),
        }
    });
    add_action(
        application,
        shared,
        "save-as",
        &["<Control><Shift>s"],
        |shared| {
            dialogs::save_as(shared, |_| {});
        },
    );
    add_action(
        application,
        shared,
        "export-docx",
        &["<Control>e"],
        dialogs::export_docx,
    );
    add_action(
        application,
        shared,
        "export-roman",
        &[],
        dialogs::export_roman,
    );
    add_action(
        application,
        shared,
        "copy-output",
        &["<Control><Shift>c"],
        copy_output,
    );
    add_action(application, shared, "find", &["<Control>f"], open_find);
    add_action(application, shared, "replace", &["<Control>h"], |shared| {
        open_find(shared);
        shared.ui.replace_entry.grab_focus();
    });
    add_action(application, shared, "find-next", &["F3"], |shared| {
        find(shared, true)
    });
    add_action(
        application,
        shared,
        "find-previous",
        &["<Shift>F3"],
        |shared| find(shared, false),
    );
    add_action(application, shared, "find-close", &[], close_find);
    add_action(application, shared, "replace-one", &[], replace_one);
    add_action(application, shared, "replace-all", &[], replace_all);
    add_action(application, shared, "language", &["<Control>l"], |shared| {
        shared.ui.language_button.popup();
    });
    add_action(
        application,
        shared,
        "previous-language",
        &["<Control><Shift>l"],
        previous_language,
    );
    add_action(
        application,
        shared,
        "toggle-exact",
        &["<Control>m"],
        |shared| {
            let (supported, exact) = {
                let app = shared.app.borrow();
                (app.language.supports_exact(), app.options().exact)
            };
            if supported {
                set_exact(shared, !exact);
                toast(
                    shared,
                    if exact {
                        "AI mode"
                    } else {
                        "Exact ITRANS / IAST mode"
                    },
                );
            }
        },
    );
    add_action(
        application,
        shared,
        "toggle-keyboard",
        &["<Control>k"],
        |shared| {
            let button = &shared.ui.keyboard_button;
            button.set_active(!button.is_active());
        },
    );
    add_action(
        application,
        shared,
        "toggle-theme",
        &["<Control><Shift>t"],
        |shared| {
            let next = match shared.app.borrow().theme {
                Theme::Dark => Theme::Light,
                Theme::Light => Theme::Dark,
            };
            {
                let mut app = shared.app.borrow_mut();
                app.settings.theme = if next == Theme::Dark { "dark" } else { "light" }.into();
                app.save_settings();
            }
            apply_theme(shared, next);
        },
    );
    add_action(
        application,
        shared,
        "toggle-layout",
        &["<Control><Shift>v"],
        |shared| {
            let vertical = {
                let mut app = shared.app.borrow_mut();
                app.settings.vertical_layout = !app.settings.vertical_layout;
                app.save_settings();
                app.settings.vertical_layout
            };
            apply_layout(shared, vertical);
        },
    );
    add_action(
        application,
        shared,
        "zoom-in",
        &["<Control>plus", "<Control>equal", "<Control>KP_Add"],
        |shared| zoom(shared, 1),
    );
    add_action(
        application,
        shared,
        "zoom-out",
        &["<Control>minus", "<Control>KP_Subtract"],
        |shared| zoom(shared, -1),
    );
    add_action(
        application,
        shared,
        "zoom-reset",
        &["<Control>0", "<Control>KP_0"],
        |shared| zoom(shared, 0),
    );
    add_action(
        application,
        shared,
        "convert-script",
        &["<Control><Shift>k"],
        dialogs::script_converter,
    );
    add_action(
        application,
        shared,
        "preferences",
        &["<Control>comma"],
        dialogs::preferences,
    );
    add_action(
        application,
        shared,
        "shortcuts",
        &["F1", "<Control>question"],
        dialogs::shortcuts,
    );
    add_action(application, shared, "about", &[], dialogs::about);
    add_action(application, shared, "quit", &["<Control>q"], |shared| {
        shared.ui.window.close();
    });

    let pick = gio::SimpleAction::new("pick", Some(glib::VariantTy::INT32));
    let weak = Rc::downgrade(shared);
    pick.connect_activate(move |_, parameter| {
        if let (Some(shared), Some(index)) = (
            weak.upgrade(),
            parameter.and_then(|value| value.get::<i32>()),
        ) {
            if index >= 1 {
                choose_candidate(&shared, index as usize - 1);
            }
        }
    });
    application.add_action(&pick);
    for index in 1..=9 {
        application.set_accels_for_action(
            &format!("app.pick({index})"),
            &[&format!("<Alt>{index}"), &format!("<Alt>KP_{index}")],
        );
    }

    let weak = Rc::downgrade(shared);
    let escape = gtk::EventControllerKey::new();
    escape.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape {
            if let Some(shared) = weak.upgrade() {
                if shared.ui.find_revealer.reveals_child() {
                    close_find(&shared);
                    return glib::Propagation::Stop;
                }
            }
        }
        glib::Propagation::Proceed
    });
    shared.ui.window.add_controller(escape);
}

pub fn apply_theme(shared: &SharedRef, theme: Theme) {
    shared.app.borrow_mut().theme = theme;
    shared.providers.apply_theme(theme);
    shared.ui.theme_button.set_icon_name(match theme {
        Theme::Dark => "weather-clear-night-symbolic",
        Theme::Light => "weather-clear-symbolic",
    });
    for view in [&shared.ui.roman, &shared.ui.output] {
        let table = view.buffer().tag_table();
        if let Some(tag) = table.lookup("current-word") {
            tag.set_background_rgba(Some(&theme.highlight()));
        }
        if let Some(tag) = table.lookup("search-match") {
            tag.set_background_rgba(Some(&theme.search_highlight()));
        }
    }
}

pub fn apply_layout(shared: &SharedRef, vertical: bool) {
    let paned = &shared.ui.paned;
    paned.set_orientation(if vertical {
        gtk::Orientation::Vertical
    } else {
        gtk::Orientation::Horizontal
    });
    let size = if vertical {
        paned.height()
    } else {
        paned.width()
    };
    if size > 0 {
        paned.set_position(size / 2);
    }
    shared.ui.layout_button.set_icon_name(if vertical {
        "view-paged-symbolic"
    } else {
        "view-dual-symbolic"
    });
}

pub fn zoom(shared: &SharedRef, step: i32) {
    let (language, traditional, size) = {
        let mut app = shared.app.borrow_mut();
        let size = if step == 0 {
            16
        } else {
            (i32::from(app.settings.font_size) + step).clamp(9, 48) as u16
        };
        app.settings.font_size = size;
        app.save_settings();
        (app.language, app.options().traditional, size)
    };
    shared
        .providers
        .apply_editor_fonts(language, traditional, size);
    update_status(shared);
}

fn copy_output(shared: &SharedRef) {
    let buffer = shared.ui.output.buffer();
    let text = buffer
        .selection_bounds()
        .map(|(start, end)| buffer.text(&start, &end, true).to_string())
        .unwrap_or_else(|| text_of(&shared.ui.output));
    if text.is_empty() {
        toast(shared, "Nothing to copy yet");
        return;
    }
    shared.ui.window.clipboard().set_text(&text);
    let count = text.chars().count();
    toast(
        shared,
        &format!(
            "Copied {count} character{}",
            if count == 1 { "" } else { "s" }
        ),
    );
}

// ---------------------------------------------------------------------------
// Find and replace (in the output pane)
// ---------------------------------------------------------------------------

fn open_find(shared: &SharedRef) {
    shared.ui.find_revealer.set_reveal_child(true);
    let buffer = shared.ui.output.buffer();
    if let Some((start, end)) = buffer.selection_bounds() {
        let selected = buffer.text(&start, &end, true);
        if !selected.contains('\n') && !selected.is_empty() {
            shared.ui.find_entry.set_text(&selected);
        }
    }
    shared.ui.find_entry.grab_focus();
    highlight_matches(shared);
}

fn close_find(shared: &SharedRef) {
    shared.ui.find_revealer.set_reveal_child(false);
    let buffer = shared.ui.output.buffer();
    buffer.remove_tag_by_name("search-match", &buffer.start_iter(), &buffer.end_iter());
    shared.ui.output.grab_focus();
}

const SEARCH_FLAGS: gtk::TextSearchFlags = gtk::TextSearchFlags::CASE_INSENSITIVE;

fn highlight_matches(shared: &SharedRef) {
    let buffer = shared.ui.output.buffer();
    buffer.remove_tag_by_name("search-match", &buffer.start_iter(), &buffer.end_iter());
    let query = shared.ui.find_entry.text().to_string();
    if query.is_empty() {
        shared.ui.find_count.set_text("");
        return;
    }
    let mut count = 0;
    let mut iter = buffer.start_iter();
    while let Some((start, end)) =
        iter.forward_search(&query, SEARCH_FLAGS | gtk::TextSearchFlags::TEXT_ONLY, None)
    {
        buffer.apply_tag_by_name("search-match", &start, &end);
        count += 1;
        iter = end;
        if count >= 10_000 {
            break;
        }
    }
    shared.ui.find_count.set_text(&match count {
        0 => "no matches".to_owned(),
        1 => "1 match".to_owned(),
        n => format!("{n} matches"),
    });
}

fn find(shared: &SharedRef, forward: bool) {
    let query = shared.ui.find_entry.text().to_string();
    if query.is_empty() {
        return;
    }
    let buffer = shared.ui.output.buffer();
    let flags = SEARCH_FLAGS | gtk::TextSearchFlags::TEXT_ONLY;
    let (selection_start, selection_end) = buffer.selection_bounds().unwrap_or_else(|| {
        let cursor = buffer.iter_at_offset(buffer.cursor_position());
        (cursor, cursor)
    });
    let found = if forward {
        selection_end
            .forward_search(&query, flags, None)
            .or_else(|| buffer.start_iter().forward_search(&query, flags, None))
    } else {
        selection_start
            .backward_search(&query, flags, None)
            .or_else(|| buffer.end_iter().backward_search(&query, flags, None))
    };
    if let Some((mut start, end)) = found {
        buffer.select_range(&start, &end);
        shared
            .ui
            .output
            .scroll_to_iter(&mut start, 0.15, false, 0.0, 0.0);
    } else {
        toast(shared, "No match");
    }
}

fn replace_one(shared: &SharedRef) {
    let query = shared.ui.find_entry.text().to_string();
    let replacement = shared.ui.replace_entry.text().to_string();
    let buffer = shared.ui.output.buffer();
    if let Some((mut start, mut end)) = buffer.selection_bounds() {
        if buffer.text(&start, &end, true).to_lowercase() == query.to_lowercase()
            && !query.is_empty()
        {
            buffer.begin_user_action();
            buffer.delete(&mut start, &mut end);
            buffer.insert(&mut start, &replacement);
            buffer.end_user_action();
        }
    }
    find(shared, true);
    highlight_matches(shared);
}

fn replace_all(shared: &SharedRef) {
    let query = shared.ui.find_entry.text().to_string();
    if query.is_empty() {
        return;
    }
    let replacement = shared.ui.replace_entry.text().to_string();
    let buffer = shared.ui.output.buffer();
    let flags = SEARCH_FLAGS | gtk::TextSearchFlags::TEXT_ONLY;
    let mut count = 0;
    buffer.begin_user_action();
    let mut iter = buffer.start_iter();
    while let Some((mut start, mut end)) = iter.forward_search(&query, flags, None) {
        let offset = start.offset();
        buffer.delete(&mut start, &mut end);
        buffer.insert(&mut start, &replacement);
        count += 1;
        iter = buffer.iter_at_offset(offset + replacement.chars().count() as i32);
        if count >= 100_000 {
            break;
        }
    }
    buffer.end_user_action();
    highlight_matches(shared);
    toast(
        shared,
        &format!(
            "Replaced {count} occurrence{}",
            if count == 1 { "" } else { "s" }
        ),
    );
}

// ---------------------------------------------------------------------------
// Documents and session
// ---------------------------------------------------------------------------

pub fn new_document(shared: &SharedRef) {
    {
        let mut app = shared.app.borrow_mut();
        app.current_path = None;
        app.saved_output.clear();
        app.auto_output.clear();
        app.restoring = false;
    }
    set_text_quietly(shared, &shared.ui.output, "");
    shared.ui.roman.buffer().set_text("");
    render_now(shared);
    shared.ui.roman.grab_focus();
    toast(shared, "New document");
}

pub fn open_with_confirmation(shared: &SharedRef, path: PathBuf) {
    let target = shared.clone();
    dialogs::confirm_unsaved(shared, move |proceed| {
        if proceed {
            open_document(&target, &path);
        }
    });
}

/// Opens a UTF-8 file. Plain Roman (ASCII) text goes to the Roman pane to be
/// transliterated; anything else is opened as the output document.
pub fn open_document(shared: &SharedRef, path: &Path) {
    let text = match ut_storage::read_utf8(path) {
        Ok(text) => text,
        Err(error) => {
            toast(shared, &public_io_error("open the file", &error));
            return;
        }
    };
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let roman_text = text.is_ascii() && text.chars().any(|ch| ch.is_ascii_alphabetic());
    if roman_text {
        {
            let mut app = shared.app.borrow_mut();
            app.current_path = None;
            app.saved_output.clear();
            app.auto_output.clear();
        }
        set_text_quietly(shared, &shared.ui.output, "");
        shared.ui.roman.buffer().set_text(&text);
        render_now(shared);
        toast(
            shared,
            &format!("Imported Roman text from {name} — save the output under a new name"),
        );
    } else {
        {
            let mut app = shared.app.borrow_mut();
            app.current_path = Some(path.to_path_buf());
            app.saved_output = text.clone();
            app.auto_output.clear();
            app.restoring = false;
        }
        set_text_quietly(shared, &shared.ui.roman, "");
        set_text_quietly(shared, &shared.ui.output, &text);
        render_now(shared);
        toast(
            shared,
            &format!("Opened {name} — Roman typing is added at the end"),
        );
    }
    update_title(shared);
    update_counts(shared);
}

pub fn save_to(shared: &SharedRef, path: &Path) -> bool {
    let text = text_of(&shared.ui.output);
    match ut_storage::atomic_write_private(path, text.as_bytes(), true) {
        Ok(()) => {
            {
                let mut app = shared.app.borrow_mut();
                app.current_path = Some(path.to_path_buf());
                app.saved_output = text;
                app.session_dirty = true;
            }
            update_title(shared);
            let name = shared.app.borrow().document_name();
            toast(shared, &format!("Saved {name}"));
            true
        }
        Err(error) => {
            toast(shared, &public_io_error("save the document", &error));
            false
        }
    }
}

pub fn public_io_error(action: &str, error: &std::io::Error) -> String {
    let reason = match error.kind() {
        std::io::ErrorKind::AlreadyExists => "the destination already exists",
        std::io::ErrorKind::PermissionDenied => "permission was denied",
        std::io::ErrorKind::InvalidData => "the file is not UTF-8 text",
        std::io::ErrorKind::InvalidInput => "the file is not a regular text file under 16 MB",
        std::io::ErrorKind::NotFound => "the file was not found",
        std::io::ErrorKind::WriteZero => "the disk is full",
        _ => "a file system error occurred",
    };
    format!("Could not {action}: {reason}")
}

fn session_snapshot(shared: &SharedRef) -> Option<Session> {
    let roman = text_of(&shared.ui.roman);
    let output = text_of(&shared.ui.output);
    let dirty = is_dirty(shared);
    let app = shared.app.borrow();
    if !app.settings.autosave {
        return None;
    }
    Some(Session {
        version: 1,
        language: app.language.id.to_owned(),
        exact: app.options().exact,
        roman,
        output,
        auto_output: app.auto_output.clone(),
        path: app.current_path.clone(),
        modified: dirty,
    })
}

pub fn save_session_now(shared: &SharedRef) {
    let path = shared.app.borrow().paths.session_file();
    match session_snapshot(shared) {
        Some(session) => {
            let _ = ut_storage::save_session(&path, &session);
        }
        None => {
            let _ = std::fs::remove_file(&path);
        }
    }
    shared.app.borrow_mut().session_dirty = false;
}

fn schedule_session_snapshots(shared: &SharedRef) {
    let weak = Rc::downgrade(shared);
    glib::timeout_add_seconds_local(15, move || {
        let Some(shared) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        let dirty = shared.app.borrow().session_dirty;
        if dirty {
            if let Some(session) = session_snapshot(&shared) {
                let path = shared.app.borrow().paths.session_file();
                shared.app.borrow_mut().session_dirty = false;
                std::thread::spawn(move || {
                    let _ = ut_storage::save_session(&path, &session);
                });
            }
        }
        glib::ControlFlow::Continue
    });
}

fn restore_session(shared: &SharedRef) {
    let (autosave, session_path, recovery_path) = {
        let app = shared.app.borrow();
        (
            app.settings.autosave,
            app.paths.session_file(),
            app.paths.recovery_file(),
        )
    };
    // Version 1.1 kept a plain recovery file; adopt it once.
    if let Ok(text) = ut_storage::read_utf8(&recovery_path) {
        let _ = ut_storage::clear_recovery(&recovery_path);
        if !text.is_empty() {
            set_text_quietly(shared, &shared.ui.output, &text);
            update_counts(shared);
            toast(
                shared,
                "Recovered the document from the previous version — save it to keep it",
            );
            return;
        }
    }
    if !autosave {
        return;
    }
    let Ok(session) = ut_storage::load_session(&session_path) else {
        return;
    };
    if session.roman.is_empty() && session.output.is_empty() {
        return;
    }
    {
        let mut app = shared.app.borrow_mut();
        if let Some(language) = language(&session.language) {
            app.language = language;
            app.settings.language = language.id.to_owned();
            let id = language.id.to_owned();
            app.settings.exact_languages.retain(|value| value != &id);
            if session.exact && language.supports_exact() {
                app.settings.exact_languages.push(id);
            }
        }
        app.current_path = session.path.clone().filter(|path| path.exists());
        app.saved_output = if session.modified {
            String::new()
        } else {
            session.output.clone()
        };
        app.auto_output = session.auto_output.clone();
        app.restoring = true;
    }
    set_text_quietly(shared, &shared.ui.output, &session.output);
    set_text_quietly(shared, &shared.ui.roman, &session.roman);
    shared
        .ui
        .roman_placeholder
        .set_visible(session.roman.is_empty());
    let end = shared.ui.roman.buffer().end_iter();
    shared.ui.roman.buffer().place_cursor(&end);
    update_title(shared);
    update_counts(shared);
    toast(shared, "Restored your previous session");
}
