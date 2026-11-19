#![forbid(unsafe_code)]
//! On-screen script keyboards. Keys insert into whichever pane was focused
//! last, so a missing letter can be added to the output directly.

use crate::app::SharedRef;
use gtk::prelude::*;
use std::rc::Rc;
use std::sync::OnceLock;
use ut_keyboards::{builtins, find, Layout};

fn layouts() -> &'static [Layout] {
    static LAYOUTS: OnceLock<Vec<Layout>> = OnceLock::new();
    LAYOUTS.get_or_init(|| builtins().unwrap_or_default())
}

// Long lines of Indic text are measured differently by rustfmt versions.
#[rustfmt::skip]
fn matra_keys(layout_id: &str) -> &'static [&'static str] {
    match layout_id {
        "devanagari" => &[
            "ा", "ि", "ी", "ु", "ू", "ृ", "े", "ै", "ो", "ौ", "ं", "ः", "ँ", "़", "्", "।", "॥", "ॐ",
        ],
        "bengali" | "assamese" => &[
            "া", "ি", "ী", "ু", "ূ", "ৃ", "ে", "ৈ", "ো", "ৌ", "ং", "ঃ", "ঁ", "়", "্", "ৎ", "।",
        ],
        "gujarati" => &[
            "ા", "િ", "ી", "ુ", "ૂ", "ૃ", "ે", "ૈ", "ો", "ૌ", "ં", "ઃ", "ઁ", "઼", "્", "ૐ",
        ],
        "gurmukhi" => &[
            "ਾ", "ਿ", "ੀ", "ੁ", "ੂ", "ੇ", "ੈ", "ੋ", "ੌ", "ਂ", "ੰ", "ੱ", "਼", "੍", "ੴ",
        ],
        "odia" => &[
            "ା", "ି", "ୀ", "ୁ", "ୂ", "ୃ", "େ", "ୈ", "ୋ", "ୌ", "ଂ", "ଃ", "ଁ", "଼", "୍",
        ],
        "tamil" => &["ா", "ி", "ீ", "ு", "ூ", "ெ", "ே", "ை", "ொ", "ோ", "ௌ", "ஃ", "்"],
        "telugu" => &[
            "ా", "ి", "ీ", "ు", "ూ", "ృ", "ె", "ే", "ై", "ొ", "ో", "ౌ", "ం", "ః", "ఁ", "్",
        ],
        "kannada" => &[
            "ಾ", "ಿ", "ೀ", "ು", "ೂ", "ೃ", "ೆ", "ೇ", "ೈ", "ೊ", "ೋ", "ೌ", "ಂ", "ಃ", "಼", "್",
        ],
        "malayalam" => &[
            "ാ", "ി", "ീ", "ു", "ൂ", "ൃ", "െ", "േ", "ൈ", "ൊ", "ോ", "ൌ", "ം", "ഃ", "്", "ൺ", "ൻ", "ർ",
            "ൽ", "ൾ",
        ],
        "sinhala" => &[
            "ා", "ැ", "ෑ", "ි", "ී", "ු", "ූ", "ෘ", "ෙ", "ේ", "ෛ", "ො", "ෝ", "ෞ", "ං", "ඃ", "්",
        ],
        "arabic" | "perso_arabic" | "persian" => &["َ", "ِ", "ُ", "ّ", "ْ", "ً", "ٍ", "ٌ", "،", "؟", "؛"],
        "hebrew" => &["ְ", "ַ", "ָ", "ֶ", "ֵ", "ִ", "ֹ", "ֻ", "ּ", "״", "׳"],
        _ => &[],
    }
}

fn key_button(shared: &SharedRef, label: &str, value: &'static str, wide: bool) -> gtk::Button {
    let button = gtk::Button::with_label(label);
    button.add_css_class("keyboard-key");
    if wide {
        button.add_css_class("wide");
    }
    button.set_focus_on_click(false);
    let weak = Rc::downgrade(shared);
    let owned = value.to_owned();
    button.connect_clicked(move |_| {
        if let Some(shared) = weak.upgrade() {
            press(&shared, &owned);
        }
    });
    button
}

/// Combining marks are shown on a dotted circle so they are visible.
fn is_mark(ch: char) -> bool {
    let code = ch as u32;
    if (0x0900..=0x0DFF).contains(&code) {
        let relative = code & 0x7F;
        return matches!(relative, 0x00..=0x03 | 0x3A..=0x3C | 0x3E..=0x4D | 0x51..=0x57 | 0x62 | 0x63)
            || matches!(code, 0x0A70 | 0x0A71);
    }
    matches!(code, 0x0300..=0x036F | 0x0591..=0x05BD | 0x05BF | 0x05C1 | 0x05C2 | 0x05C4 | 0x05C5 | 0x05C7 | 0x064B..=0x0652 | 0x1C78..=0x1C7D)
}

fn key_button_owned(shared: &SharedRef, value: String) -> gtk::Button {
    let label = if value.chars().all(is_mark) {
        format!("◌{value}")
    } else {
        value.clone()
    };
    let button = gtk::Button::with_label(&label);
    button.add_css_class("keyboard-key");
    button.set_focus_on_click(false);
    let weak = Rc::downgrade(shared);
    button.connect_clicked(move |_| {
        if let Some(shared) = weak.upgrade() {
            press(&shared, &value);
        }
    });
    button
}

pub fn render(shared: &SharedRef, layout_id: &str) {
    let container = &shared.ui.keyboard_box;
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
    let Some(layout) = find(layouts(), layout_id) else {
        return;
    };
    let caption = gtk::Label::new(Some(&format!(
        "{} keyboard — keys go into the pane you used last",
        layout.name
    )));
    caption.add_css_class("keyboard-caption");
    container.append(&caption);
    for row in &layout.rows {
        let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        row_box.set_halign(gtk::Align::Center);
        for key in row {
            match key.as_str() {
                "Space" | "Enter" | "Backspace" => continue,
                _ => row_box.append(&key_button_owned(shared, key.clone())),
            }
        }
        if row_box.first_child().is_some() {
            container.append(&row_box);
        }
    }
    let matras = matra_keys(&layout.id);
    if !matras.is_empty() {
        let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        row_box.set_halign(gtk::Align::Center);
        for value in matras {
            row_box.append(&key_button_owned(shared, (*value).to_owned()));
        }
        container.append(&row_box);
    }
    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    controls.set_halign(gtk::Align::Center);
    controls.append(&key_button(shared, "⌫ Backspace", "\u{8}", true));
    controls.append(&key_button(shared, "Space", " ", true));
    controls.append(&key_button(shared, "↵ Enter", "\n", true));
    container.append(&controls);
}

fn press(shared: &SharedRef, value: &str) {
    let editor = shared.ui.active_editor.borrow().clone();
    let buffer = editor.buffer();
    if value == "\u{8}" {
        if let Some((mut start, mut end)) = buffer.selection_bounds() {
            buffer.delete(&mut start, &mut end);
        } else {
            let mut end = buffer.iter_at_offset(buffer.cursor_position());
            let mut start = end;
            if start.backward_char() {
                buffer.delete(&mut start, &mut end);
            }
        }
    } else {
        buffer.delete_selection(true, true);
        buffer.insert_at_cursor(value);
    }
    editor.grab_focus();
}
