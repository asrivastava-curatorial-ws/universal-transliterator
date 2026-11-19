#![forbid(unsafe_code)]

use std::path::Path;
use ut_core::{bounded_candidates, CompositionEngine, EngineError, EngineKey, EngineSnapshot};
use ut_rime_sys::{RawSnapshot, Session};

pub struct RimeEngine {
    session: Session,
}

impl RimeEngine {
    pub fn new(shared_data: &Path, user_data: &Path) -> Result<Self, EngineError> {
        Self::new_with_schema(shared_data, user_data, "pinyin_simp")
    }

    pub fn new_with_schema(
        shared_data: &Path,
        user_data: &Path,
        schema: &str,
    ) -> Result<Self, EngineError> {
        if !matches!(schema, "pinyin_simp" | "luna_pinyin") {
            return Err(EngineError::new("unsupported bundled Rime schema"));
        }
        std::fs::create_dir_all(user_data).map_err(|error| {
            EngineError::new(format!("cannot create Rime data directory: {error}"))
        })?;
        Session::new(shared_data, user_data, schema)
            .map(|session| Self { session })
            .map_err(EngineError::new)
    }

    fn convert(raw: RawSnapshot) -> EngineSnapshot {
        EngineSnapshot {
            raw_input: raw.preedit.clone(),
            preedit: raw.preedit.clone(),
            cursor: raw.cursor,
            segments: Vec::new(),
            candidates: bounded_candidates(raw.candidates),
            highlighted: raw.highlighted,
            page: raw.page,
            page_size: raw.page_size,
            committed: raw.committed,
            composing: !raw.preedit.is_empty(),
        }
    }

    fn keycode(key: EngineKey) -> Option<i32> {
        match key {
            EngineKey::Char(ch) if ch.is_ascii() => Some(ch as i32),
            EngineKey::Backspace => Some(ut_rime_sys::KEY_BACKSPACE),
            EngineKey::Left => Some(ut_rime_sys::KEY_LEFT),
            EngineKey::Right => Some(ut_rime_sys::KEY_RIGHT),
            EngineKey::Up => Some(ut_rime_sys::KEY_UP),
            EngineKey::Down => Some(ut_rime_sys::KEY_DOWN),
            EngineKey::PageUp => Some(ut_rime_sys::KEY_PAGE_UP),
            EngineKey::PageDown => Some(ut_rime_sys::KEY_PAGE_DOWN),
            EngineKey::Space => Some(b' ' as i32),
            EngineKey::Enter => Some(ut_rime_sys::KEY_ENTER),
            EngineKey::Escape => Some(ut_rime_sys::KEY_ESCAPE),
            EngineKey::Delete | EngineKey::Candidate(_) | EngineKey::Char(_) => None,
        }
    }
}

impl CompositionEngine for RimeEngine {
    fn name(&self) -> &'static str {
        "Rime Pinyin"
    }

    fn reset(&mut self) -> Result<EngineSnapshot, EngineError> {
        self.session
            .clear()
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
        let Some(keycode) = Self::keycode(key) else {
            return self.snapshot();
        };
        self.session
            .process_key(keycode)
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
    fn pinyin_produces_real_hanzi_candidates() {
        if !Path::new("/usr/share/rime-data").exists() {
            return;
        }
        let user = tempfile::tempdir().unwrap();
        let mut engine = RimeEngine::new(Path::new("/usr/share/rime-data"), user.path()).unwrap();
        let mut snapshot = EngineSnapshot::default();
        for ch in "nihao".chars() {
            snapshot = engine.process(EngineKey::Char(ch)).unwrap();
        }
        assert!(snapshot
            .candidates
            .iter()
            .any(|item| item.contains('你') || item.contains("你好")));
        assert!(snapshot.composing);
    }

    #[test]
    fn traditional_schema_produces_candidates() {
        if !Path::new("/usr/share/rime-data").exists() {
            return;
        }
        let user = tempfile::tempdir().unwrap();
        let mut engine = RimeEngine::new_with_schema(
            Path::new("/usr/share/rime-data"),
            user.path(),
            "luna_pinyin",
        )
        .unwrap();
        let mut snapshot = EngineSnapshot::default();
        for ch in "hanti".chars() {
            snapshot = engine.process(EngineKey::Char(ch)).unwrap();
        }
        assert!(!snapshot.candidates.is_empty());
    }
}
