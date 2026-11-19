#![forbid(unsafe_code)]

use std::path::Path;
use ut_core::{bounded_candidates, CompositionEngine, EngineError, EngineKey, EngineSnapshot};
use ut_kkc_sys::{RawSnapshot, Session};

pub use ut_kkc_sys::InputMode as JapaneseInputMode;

pub struct JapaneseEngine {
    session: Session,
}

impl JapaneseEngine {
    pub fn new(dictionary: &Path) -> Result<Self, EngineError> {
        Session::new(dictionary)
            .map(|session| Self { session })
            .map_err(EngineError::new)
    }

    pub fn new_with_data(
        dictionary: &Path,
        model_metadata: Option<&Path>,
        rule_metadata: Option<&Path>,
        mode: JapaneseInputMode,
    ) -> Result<Self, EngineError> {
        Session::new_with_data(dictionary, model_metadata, rule_metadata, mode)
            .map(|session| Self { session })
            .map_err(EngineError::new)
    }

    fn convert(raw: RawSnapshot) -> EngineSnapshot {
        EngineSnapshot {
            raw_input: raw.input,
            preedit: raw.preedit.clone(),
            cursor: raw.cursor,
            segments: Vec::new(),
            candidates: bounded_candidates(raw.candidates),
            highlighted: raw.highlighted,
            page: raw.page_start.checked_div(raw.page_size).unwrap_or(0),
            page_size: raw.page_size,
            committed: raw.committed.filter(|value| !value.is_empty()),
            composing: !raw.preedit.is_empty(),
        }
    }

    fn keyval(key: EngineKey) -> Option<u32> {
        match key {
            EngineKey::Char(ch) if ch.is_ascii() => Some(ch as u32),
            EngineKey::Backspace => Some(ut_kkc_sys::KEY_BACKSPACE),
            EngineKey::Left => Some(ut_kkc_sys::KEY_LEFT),
            EngineKey::Right => Some(ut_kkc_sys::KEY_RIGHT),
            EngineKey::Up => Some(ut_kkc_sys::KEY_UP),
            EngineKey::Down => Some(ut_kkc_sys::KEY_DOWN),
            EngineKey::PageUp => Some(ut_kkc_sys::KEY_PAGE_UP),
            EngineKey::PageDown => Some(ut_kkc_sys::KEY_PAGE_DOWN),
            EngineKey::Space => Some(b' ' as u32),
            EngineKey::Enter => Some(ut_kkc_sys::KEY_ENTER),
            EngineKey::Escape => Some(ut_kkc_sys::KEY_ESCAPE),
            EngineKey::Delete | EngineKey::Candidate(_) | EngineKey::Char(_) => None,
        }
    }
}

impl JapaneseEngine {
    /// Shows a candidate of the focused segment in the preedit without
    /// committing the sentence.
    pub fn preview(&mut self, index: usize) -> Result<EngineSnapshot, EngineError> {
        self.session
            .preview_candidate(index)
            .map(Self::convert)
            .map_err(EngineError::new)
    }

    /// Number of segments (bunsetsu) while converting.
    pub fn segment_count(&mut self) -> usize {
        self.session.segment_count()
    }
}

impl CompositionEngine for JapaneseEngine {
    fn name(&self) -> &'static str {
        "libkkc Japanese"
    }

    fn reset(&mut self) -> Result<EngineSnapshot, EngineError> {
        self.session
            .reset()
            .map(Self::convert)
            .map_err(EngineError::new)
    }

    fn process(&mut self, key: EngineKey) -> Result<EngineSnapshot, EngineError> {
        if let EngineKey::Candidate(index) = key {
            return self
                .session
                .select_candidate(index)
                .map(Self::convert)
                .map_err(EngineError::new);
        }
        let Some(keyval) = Self::keyval(key) else {
            return self.snapshot();
        };
        self.session
            .process_key(keyval)
            .map(Self::convert)
            .map_err(EngineError::new)
    }

    fn snapshot(&mut self) -> Result<EngineSnapshot, EngineError> {
        self.session
            .snapshot()
            .map(Self::convert)
            .map_err(EngineError::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn romaji_becomes_kana_and_kanji_candidates() {
        let dictionary = Path::new("/usr/share/skk/SKK-JISYO.L");
        if !dictionary.exists() {
            return;
        }
        let mut engine = JapaneseEngine::new(dictionary).unwrap();
        let mut snapshot = EngineSnapshot::default();
        for ch in "watashi".chars() {
            snapshot = engine.process(EngineKey::Char(ch)).unwrap();
        }
        assert_eq!(snapshot.preedit, "わたし");
        snapshot = engine.process(EngineKey::Space).unwrap();
        assert_eq!(snapshot.preedit, "私");
        snapshot = engine.process(EngineKey::Space).unwrap();
        assert!(snapshot.candidates.iter().any(|item| item == "私"));
    }

    #[test]
    fn relocatable_data_paths_work() {
        let dictionary = Path::new("/usr/share/skk/SKK-JISYO.L");
        let model = Path::new("/usr/lib/x86_64-linux-gnu/libkkc/models/sorted3/metadata.json");
        let rule = Path::new("/usr/share/libkkc/rules/default/metadata.json");
        if !dictionary.exists() || !model.exists() || !rule.exists() {
            return;
        }
        let mut engine = JapaneseEngine::new_with_data(
            dictionary,
            Some(model),
            Some(rule),
            JapaneseInputMode::Katakana,
        )
        .unwrap();
        let mut snapshot = EngineSnapshot::default();
        for ch in "toukyou".chars() {
            snapshot = engine.process(EngineKey::Char(ch)).unwrap();
        }
        assert_eq!(snapshot.preedit, "トウキョウ");
    }
}
