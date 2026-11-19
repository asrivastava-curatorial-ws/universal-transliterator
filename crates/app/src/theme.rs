#![forbid(unsafe_code)]
//! GitHub-style dark and light themes with JetBrains Mono.
//!
//! The stylesheet is split into a palette (swapped when the theme changes)
//! and structural rules that only refer to palette colours. Editor fonts are
//! a third provider because they depend on the language and zoom level.

use gtk::gdk;
use gtk::prelude::*;
use ut_transliteration::{LanguageInfo, Script};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub fn from_setting(value: &str) -> Theme {
        match value {
            "light" => Theme::Light,
            "dark" => Theme::Dark,
            _ => system_theme(),
        }
    }

    /// Background colour used for the highlighted word in both panes.
    pub fn highlight(self) -> gdk::RGBA {
        match self {
            Theme::Dark => gdk::RGBA::new(0.22, 0.545, 0.992, 0.24),
            Theme::Light => gdk::RGBA::new(0.035, 0.412, 0.855, 0.16),
        }
    }

    pub fn search_highlight(self) -> gdk::RGBA {
        match self {
            Theme::Dark => gdk::RGBA::new(0.824, 0.6, 0.133, 0.38),
            Theme::Light => gdk::RGBA::new(0.98, 0.78, 0.2, 0.55),
        }
    }
}

/// Follows the desktop's light/dark preference through the XDG settings
/// portal (a local D-Bus call). Falls back to dark.
fn system_theme() -> Theme {
    let Ok(connection) = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE) else {
        return Theme::Dark;
    };
    let reply = connection.call_sync(
        Some("org.freedesktop.portal.Desktop"),
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Settings",
        "Read",
        Some(&("org.freedesktop.appearance", "color-scheme").to_variant()),
        None,
        gio::DBusCallFlags::NONE,
        500,
        gio::Cancellable::NONE,
    );
    // The reply is (v) holding a variant that holds a u32: 1 dark, 2 light.
    let value = reply.ok().and_then(|variant| {
        let outer = variant.child_value(0);
        let mut inner = outer.as_variant()?;
        while let Some(next) = inner.as_variant() {
            inner = next;
        }
        inner.get::<u32>()
    });
    match value {
        Some(2) => Theme::Light,
        _ => Theme::Dark,
    }
}

const DARK: &str = r#"
@define-color bg #0d1117;
@define-color bg_subtle #161b22;
@define-color bg_inset #010409;
@define-color bg_overlay #161b22;
@define-color border #30363d;
@define-color border_muted #21262d;
@define-color fg #e6edf3;
@define-color fg_muted #8d96a0;
@define-color fg_subtle #6e7681;
@define-color accent #2f81f7;
@define-color accent_fg #4493f8;
@define-color accent_subtle rgba(56, 139, 253, 0.15);
@define-color success #238636;
@define-color success_hover #2ea043;
@define-color success_fg #3fb950;
@define-color attention_fg #d29922;
@define-color danger_fg #f85149;
@define-color button_bg #21262d;
@define-color button_hover #30363d;
@define-color button_border rgba(240, 246, 252, 0.10);
@define-color selection rgba(56, 139, 253, 0.40);
@define-color shadow rgba(1, 4, 9, 0.85);
"#;

const LIGHT: &str = r#"
@define-color bg #ffffff;
@define-color bg_subtle #f6f8fa;
@define-color bg_inset #f6f8fa;
@define-color bg_overlay #ffffff;
@define-color border #d0d7de;
@define-color border_muted #d8dee4;
@define-color fg #1f2328;
@define-color fg_muted #656d76;
@define-color fg_subtle #6e7781;
@define-color accent #0969da;
@define-color accent_fg #0969da;
@define-color accent_subtle #ddf4ff;
@define-color success #1f883d;
@define-color success_hover #1a7f37;
@define-color success_fg #1a7f37;
@define-color attention_fg #9a6700;
@define-color danger_fg #d1242f;
@define-color button_bg #f6f8fa;
@define-color button_hover #eef1f4;
@define-color button_border rgba(31, 35, 40, 0.15);
@define-color selection rgba(9, 105, 218, 0.25);
@define-color shadow rgba(140, 149, 159, 0.35);
"#;

const STRUCTURE: &str = r#"
* {
  /* Script fonts are listed explicitly so labels and entries with native text
     (the output title, find/replace) never fall back to a font that cannot
     shape the script. */
  font-family: "JetBrains Mono", "Noto Sans Devanagari", "Noto Sans Bengali",
    "Noto Sans Gurmukhi", "Noto Sans Gujarati", "Noto Sans Oriya", "Noto Sans Tamil",
    "Noto Sans Telugu", "Noto Sans Kannada", "Noto Sans Malayalam", "Noto Sans Sinhala",
    "Noto Naskh Arabic", "Noto Sans Hebrew", "Noto Sans Armenian", "Noto Sans Georgian",
    "Noto Sans Thai", "Noto Sans Ethiopic", "Noto Sans Ol Chiki", "Noto Sans Meetei Mayek",
    "Noto Sans CJK SC", "Noto Sans Mono", monospace;
}
window, dialog, .background {
  background-color: @bg;
  color: @fg;
}
window { font-size: 10pt; }
label { color: inherit; }
.dim, .dim-label { color: @fg_muted; }
.subtle { color: @fg_subtle; }

/* Header bar */
headerbar {
  background: @bg_subtle;
  color: @fg;
  border-bottom: 1px solid @border;
  box-shadow: none;
  min-height: 46px;
  padding: 0 6px;
}
headerbar:backdrop { background: @bg_subtle; }
headerbar .title { font-weight: 700; }
windowcontrols button {
  min-width: 22px; min-height: 22px; padding: 2px; margin: 0 2px;
  background: transparent; border: none; box-shadow: none;
}
windowcontrols button:hover { background: @button_hover; }

/* Buttons (GitHub style) */
button {
  background: @button_bg;
  color: @fg;
  border: 1px solid @button_border;
  border-radius: 6px;
  box-shadow: none;
  text-shadow: none;
  -gtk-icon-shadow: none;
  padding: 4px 10px;
  min-height: 22px;
  outline-color: @accent;
}
button:hover { background: @button_hover; }
button:active, button:checked { background: @border; }
button:disabled { color: @fg_subtle; opacity: 0.6; }
button.flat { background: transparent; border-color: transparent; }
button.flat:hover { background: @button_hover; }
button.flat:checked { background: @accent_subtle; color: @accent_fg; }
button.image-button { padding: 4px 7px; }
button.suggested-action, button.primary {
  background: @success;
  color: #ffffff;
  border-color: rgba(240, 246, 252, 0.10);
  font-weight: 700;
}
button.suggested-action:hover, button.primary:hover { background: @success_hover; }
button.suggested-action label, button.primary label { color: #ffffff; }
button.destructive-action { color: @danger_fg; }
.linked > button { border-radius: 0; }
.linked > button:first-child { border-top-left-radius: 6px; border-bottom-left-radius: 6px; }
.linked > button:last-child { border-top-right-radius: 6px; border-bottom-right-radius: 6px; }
.segmented > togglebutton, .segmented > button {
  padding: 3px 10px;
  font-size: 9pt;
}
.segmented > button:checked {
  background: @accent_subtle;
  color: @accent_fg;
  border-color: @accent;
  font-weight: 700;
}

/* Language button */
.language-button {
  padding: 4px 12px;
  border-radius: 6px;
  font-weight: 700;
}
.language-button .native { font-size: 11pt; }
.language-button .engine { color: @fg_muted; font-weight: 400; font-size: 8.5pt; }

/* Entries, dropdowns, spin buttons */
entry, spinbutton, searchbar entry {
  background: @bg;
  color: @fg;
  border: 1px solid @border;
  border-radius: 6px;
  box-shadow: none;
  min-height: 28px;
  caret-color: @fg;
}
entry:focus-within, spinbutton:focus-within {
  border-color: @accent;
  outline: 1px solid @accent;
  outline-offset: -1px;
}
entry placeholder, entry > text > placeholder { color: @fg_subtle; }
dropdown > button, combobox > button { padding: 3px 8px; }
popover > contents, popover.menu > contents {
  background: @bg_overlay;
  color: @fg;
  border: 1px solid @border;
  border-radius: 8px;
  box-shadow: 0 8px 24px @shadow;
  padding: 6px;
}
popover > arrow { background: @bg_overlay; border: 1px solid @border; }
popover.menu modelbutton { border-radius: 6px; padding: 6px 10px; min-height: 22px; }
popover.menu modelbutton:hover { background: @accent_subtle; }
popover.menu accelerator { color: @fg_subtle; margin-left: 24px; }
popover.menu separator { background: @border_muted; margin: 4px 0; }

/* Lists */
list, listview { background: transparent; color: @fg; }
row { border-radius: 6px; padding: 0; }
row:hover { background: @accent_subtle; }
row:selected, row:selected:focus { background: @accent_subtle; color: @fg; }
row.group-header { background: transparent; }
row.group-header label {
  color: @fg_muted;
  font-size: 8.5pt;
  font-weight: 700;
  padding: 10px 8px 4px 8px;
}
.native, .script-text {
  font-family: "Noto Sans", "Noto Sans Devanagari", "Noto Sans Bengali", "Noto Sans Gurmukhi",
    "Noto Sans Gujarati", "Noto Sans Oriya", "Noto Sans Tamil", "Noto Sans Telugu",
    "Noto Sans Kannada", "Noto Sans Malayalam", "Noto Sans Sinhala", "Noto Naskh Arabic",
    "Noto Nastaliq Urdu", "Noto Sans Hebrew", "Noto Sans Armenian", "Noto Sans Georgian",
    "Noto Sans Thai", "Noto Sans Ethiopic", "Noto Sans Ol Chiki", "Noto Sans Meetei Mayek",
    "Noto Sans CJK SC", "JetBrains Mono", sans-serif;
}
.language-row { padding: 6px 8px; }
.language-row .native { font-size: 12pt; font-weight: 700; }
.language-row .name { color: @fg_muted; }
.language-row.current { background: @accent_subtle; }

.badge {
  font-size: 7.5pt;
  font-weight: 700;
  padding: 1px 7px;
  border-radius: 999px;
  border: 1px solid @border;
  color: @fg_muted;
}
.badge.ai { color: @accent_fg; border-color: @accent; }
.badge.dictionary { color: @success_fg; border-color: @success; }
.badge.rules { color: @attention_fg; border-color: @attention_fg; }

/* Document panels */
.workspace { margin: 10px 12px 0 12px; }
.panel {
  background: @bg;
  border: 1px solid @border;
  border-radius: 8px;
}
.panel-header {
  background: @bg_subtle;
  border-bottom: 1px solid @border;
  border-radius: 8px 8px 0 0;
  padding: 5px 6px 5px 12px;
  min-height: 30px;
}
.panel-title { font-weight: 700; font-size: 9.5pt; }
.panel-note { color: @fg_muted; font-size: 8.5pt; }
.panel-count { color: @fg_subtle; font-size: 8.5pt; }
paned > separator {
  background: transparent;
  min-width: 10px;
  min-height: 10px;
  background-image: none;
}
paned.horizontal > separator { margin: 0 1px; }
paned > separator:hover { background: @accent_subtle; border-radius: 4px; }

textview, textview text {
  background-color: @bg;
  color: @fg;
}
textview text selection, textview selection, text selection {
  background-color: @selection;
  color: @fg;
}
textview { caret-color: @accent_fg; }
.placeholder {
  color: @fg_subtle;
  font-size: 11pt;
  margin: 18px 20px;
}

/* Alternatives bar */
.candidate-bar {
  margin: 8px 12px 0 12px;
  padding: 5px 8px;
  border: 1px solid @border;
  border-radius: 8px;
  background: @bg_subtle;
  min-height: 34px;
}
.candidate-word { color: @fg_muted; font-size: 9pt; margin-right: 4px; }
.candidate-word .roman { color: @fg; font-weight: 700; }
.candidate {
  padding: 3px 10px;
  margin: 0 2px;
  border-radius: 6px;
  background: @bg;
}
.candidate .number { color: @fg_subtle; font-size: 8pt; margin-right: 6px; }
.candidate .hint { color: @fg_subtle; font-size: 7.5pt; margin-left: 6px; }
.candidate.current { border-color: @accent; background: @accent_subtle; }
.candidate.current .number { color: @accent_fg; }

/* Find bar and notices */
.findbar {
  margin: 8px 12px 0 12px;
  padding: 6px 8px;
  background: @bg_subtle;
  border: 1px solid @border;
  border-radius: 8px;
}
.toast {
  background: @bg_overlay;
  color: @fg;
  border: 1px solid @border;
  border-radius: 8px;
  padding: 8px 14px;
  margin-bottom: 44px;
  box-shadow: 0 8px 24px @shadow;
}

/* On-screen keyboard */
.keyboard {
  margin: 8px 12px 0 12px;
  padding: 8px;
  background: @bg_subtle;
  border: 1px solid @border;
  border-radius: 8px;
}
.keyboard-key {
  min-width: 38px;
  min-height: 34px;
  padding: 2px 6px;
  font-size: 13pt;
  font-family: "Noto Sans", "Noto Sans Devanagari", "Noto Sans Bengali", "Noto Sans Gurmukhi",
    "Noto Sans Gujarati", "Noto Sans Oriya", "Noto Sans Tamil", "Noto Sans Telugu",
    "Noto Sans Kannada", "Noto Sans Malayalam", "Noto Sans Sinhala", "Noto Naskh Arabic",
    "Noto Sans Hebrew", "Noto Sans Armenian", "Noto Sans Georgian", "Noto Sans Thai",
    "Noto Sans Ethiopic", "Noto Sans Ol Chiki", "Noto Sans Meetei Mayek", "Noto Sans CJK SC", sans-serif;
}
.keyboard-key.wide { min-width: 96px; font-size: 9.5pt; font-family: "JetBrains Mono", monospace; }
.keyboard-caption { color: @fg_muted; font-size: 8pt; margin: 2px 0; }

/* Status bar */
.statusbar {
  border-top: 1px solid @border;
  background: @bg_subtle;
  padding: 3px 12px;
  margin-top: 8px;
  min-height: 24px;
}
.statusbar label { font-size: 8.5pt; color: @fg_muted; }
.status-dot { font-size: 9pt; }
.status-dot.ready { color: @success_fg; }
.status-dot.busy { color: @attention_fg; }
.status-dot.warning { color: @danger_fg; }

/* Dialogs */
.dialog-content { padding: 18px 20px; }
.section-title {
  font-weight: 700;
  color: @fg;
  margin-top: 10px;
  margin-bottom: 4px;
  border-bottom: 1px solid @border_muted;
  padding-bottom: 4px;
}
.kbd {
  font-size: 8.5pt;
  padding: 1px 6px;
  border: 1px solid @border;
  border-bottom-width: 2px;
  border-radius: 5px;
  background: @bg_subtle;
  color: @fg;
}
switch { background: @border; border: none; border-radius: 999px; }
switch:checked { background: @success; }
switch slider { background: #ffffff; border-radius: 999px; min-width: 18px; min-height: 18px; }
scrollbar { background: transparent; }
scrollbar slider { background: @border; border-radius: 999px; min-width: 6px; min-height: 6px; }
scrollbar slider:hover { background: @fg_subtle; }
tooltip, tooltip.background {
  background: @bg_overlay;
  color: @fg;
  border: 1px solid @border;
  border-radius: 6px;
}
tooltip label { color: @fg; }
separator { background: @border_muted; }
checkbutton check { border: 1px solid @border; background: @bg; }
checkbutton check:checked { background: @accent; color: #ffffff; }
aboutdialog .large-title, .large-title { font-size: 16pt; font-weight: 700; }
"#;

pub struct Providers {
    palette: gtk::CssProvider,
    editors: gtk::CssProvider,
}

impl Providers {
    pub fn install() -> Self {
        let structure = gtk::CssProvider::new();
        structure.load_from_string(STRUCTURE);
        let palette = gtk::CssProvider::new();
        let editors = gtk::CssProvider::new();
        if let Some(display) = gdk::Display::default() {
            // Palette first so the structure rules can resolve @colours.
            gtk::style_context_add_provider_for_display(
                &display,
                &palette,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
            gtk::style_context_add_provider_for_display(
                &display,
                &structure,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
            );
            gtk::style_context_add_provider_for_display(
                &display,
                &editors,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 2,
            );
        }
        Self { palette, editors }
    }

    pub fn apply_theme(&self, theme: Theme) {
        self.palette.load_from_string(match theme {
            Theme::Dark => DARK,
            Theme::Light => LIGHT,
        });
        if let Some(settings) = gtk::Settings::default() {
            settings.set_gtk_application_prefer_dark_theme(theme == Theme::Dark);
        }
    }

    pub fn apply_editor_fonts(&self, language: &LanguageInfo, traditional: bool, size: u16) {
        let script_font = match language.script {
            Script::Han if traditional => "Noto Sans CJK TC",
            other => other.font_family(),
        };
        let css = format!(
            r#"
textview.roman-editor, textview.roman-editor text {{
  font-family: "JetBrains Mono", "Noto Sans Mono", monospace;
  font-size: {size}pt;
}}
textview.output-editor, textview.output-editor text {{
  font-family: "{script_font}", "Noto Sans", "JetBrains Mono", "Noto Sans Devanagari",
    "Noto Sans CJK SC", "Noto Naskh Arabic", sans-serif;
  font-size: {output}pt;
}}
.candidate .text {{
  font-family: "{script_font}", "Noto Sans", "JetBrains Mono", sans-serif;
  font-size: 12pt;
}}
.preview-editor, .preview-editor text {{
  font-family: "JetBrains Mono", "Noto Sans", "Noto Sans Devanagari", "Noto Sans Bengali",
    "Noto Sans Gurmukhi", "Noto Sans Gujarati", "Noto Sans Oriya", "Noto Sans Tamil",
    "Noto Sans Telugu", "Noto Sans Kannada", "Noto Sans Malayalam", "Noto Sans Sinhala", sans-serif;
  font-size: {size}pt;
}}
"#,
            output = size + 1,
        );
        self.editors.load_from_string(&css);
    }
}
