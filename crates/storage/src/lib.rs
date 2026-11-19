#![forbid(unsafe_code)]

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

pub const MAX_DOCUMENT_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub version: u32,
    pub language: String,
    /// Editor font size in points.
    pub font_size: u16,
    pub show_keyboard: bool,
    /// "dark", "light" or "system".
    pub theme: String,
    /// Restore the previous session (both panes) on start.
    pub autosave: bool,
    /// Remember chosen alternatives across sessions.
    pub learning_enabled: bool,
    pub chinese_variant: String,
    pub japanese_mode: String,
    /// Languages switched to exact (ITRANS / IAST) mode.
    pub exact_languages: Vec<String>,
    /// Most recently used languages, newest first.
    pub recent_languages: Vec<String>,
    /// Write 0–9 with the script's own digits.
    pub native_digits: bool,
    pub window_width: i32,
    pub window_height: i32,
    pub window_maximized: bool,
    /// Divider position between the panes; 0 means "half".
    pub pane_position: i32,
    /// Stack the panes vertically instead of side by side.
    pub vertical_layout: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 2,
            language: "hi".into(),
            font_size: 16,
            show_keyboard: false,
            theme: "dark".into(),
            autosave: true,
            learning_enabled: true,
            chinese_variant: "simplified".into(),
            japanese_mode: "hiragana".into(),
            exact_languages: Vec::new(),
            recent_languages: Vec::new(),
            native_digits: false,
            window_width: 1280,
            window_height: 820,
            window_maximized: false,
            pane_position: 0,
            vertical_layout: false,
        }
    }
}

/// The working state of both panes, restored on the next start.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Session {
    pub version: u32,
    pub language: String,
    pub exact: bool,
    pub roman: String,
    pub output: String,
    /// The last automatically generated output (base for merging edits).
    pub auto_output: String,
    pub path: Option<PathBuf>,
    pub modified: bool,
}

/// Alternatives the person picked, per language namespace and Roman key.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Learned {
    pub version: u32,
    pub choices: BTreeMap<String, BTreeMap<String, String>>,
}

pub const MAX_LEARNED_ENTRIES: usize = 50_000;

impl Learned {
    pub fn get(&self, namespace: &str, key: &str) -> Option<&String> {
        self.choices.get(namespace).and_then(|map| map.get(key))
    }

    pub fn set(&mut self, namespace: &str, key: &str, value: &str) {
        if key.chars().count() > 128
            || value.chars().count() > 256
            || self.len() >= MAX_LEARNED_ENTRIES
        {
            return;
        }
        self.choices
            .entry(namespace.to_owned())
            .or_default()
            .insert(key.to_owned(), value.to_owned());
    }

    pub fn remove(&mut self, namespace: &str, key: &str) -> bool {
        let removed = self
            .choices
            .get_mut(namespace)
            .is_some_and(|map| map.remove(key).is_some());
        self.choices.retain(|_, map| !map.is_empty());
        removed
    }

    pub fn len(&self) -> usize {
        self.choices.values().map(BTreeMap::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Clone, Debug)]
pub struct Paths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub state_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl Paths {
    pub fn discover() -> Result<Self, String> {
        let dirs = ProjectDirs::from("org", "UniversalTransliterator", "UniversalTransliterator")
            .ok_or_else(|| "XDG user directories are unavailable".to_string())?;
        let state_dir = std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs.data_local_dir().to_path_buf())
            .join("universal-transliterator");
        let paths = Self {
            config_dir: dirs.config_dir().to_path_buf(),
            data_dir: dirs.data_dir().to_path_buf(),
            state_dir,
            cache_dir: dirs.cache_dir().to_path_buf(),
        };
        for path in [
            &paths.config_dir,
            &paths.data_dir,
            &paths.state_dir,
            &paths.cache_dir,
        ] {
            fs::create_dir_all(path)
                .map_err(|error| format!("cannot create application data directory: {error}"))?;
            let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o700));
        }
        Ok(paths)
    }

    pub fn settings_file(&self) -> PathBuf {
        self.config_dir.join("settings.json")
    }
    /// Plain-text recovery file written by version 1.1.
    pub fn recovery_file(&self) -> PathBuf {
        self.state_dir.join("recovery.txt")
    }
    pub fn session_file(&self) -> PathBuf {
        self.state_dir.join("session.json")
    }
    pub fn learned_file(&self) -> PathBuf {
        self.data_dir.join("learned.json")
    }
    pub fn rime_user_dir(&self) -> PathBuf {
        self.data_dir.join("rime")
    }
}

pub fn read_utf8(path: &Path) -> io::Result<String> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "symbolic links are not opened",
        ));
    }
    if !metadata.is_file() || metadata.len() > MAX_DOCUMENT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "file is not a regular UTF-8 document within the size limit",
        ));
    }
    let file = File::open(path)?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_DOCUMENT_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "document exceeds the size limit",
        ));
    }
    String::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "document is not valid UTF-8"))
}

pub fn atomic_write_private(path: &Path, contents: &[u8], overwrite: bool) -> io::Result<()> {
    if contents.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "document exceeds the size limit",
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "destination has no parent"))?;
    fs::create_dir_all(parent)?;
    if path.exists() && !overwrite {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "destination already exists",
        ));
    }
    if path
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "refusing to replace a symbolic link",
        ));
    }
    let mut temp = NamedTempFile::new_in(parent)?;
    temp.as_file()
        .set_permissions(fs::Permissions::from_mode(0o600))?;
    temp.write_all(contents)?;
    temp.as_file().sync_all()?;
    if overwrite && path.exists() {
        temp.persist(path).map_err(|error| error.error)?;
    } else {
        temp.persist_noclobber(path).map_err(|error| error.error)?;
    }
    File::open(parent)?.sync_all()?;
    Ok(())
}

pub fn load_settings(path: &Path) -> io::Result<Settings> {
    if !path.exists() {
        return Ok(Settings::default());
    }
    let text = read_utf8(path)?;
    let mut settings: Settings = serde_json::from_str(&text)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "settings file is malformed"))?;
    settings.font_size = settings.font_size.clamp(9, 72);
    if !matches!(settings.theme.as_str(), "system" | "light" | "dark") {
        settings.theme = "system".into();
    }
    if !matches!(
        settings.chinese_variant.as_str(),
        "simplified" | "traditional"
    ) {
        settings.chinese_variant = "simplified".into();
    }
    if !matches!(settings.japanese_mode.as_str(), "hiragana" | "katakana") {
        settings.japanese_mode = "hiragana".into();
    }
    let valid_id = |id: &String| {
        !id.is_empty()
            && id.len() <= 16
            && id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
    };
    settings.exact_languages.retain(valid_id);
    settings.exact_languages.truncate(64);
    settings.recent_languages.retain(valid_id);
    settings.recent_languages.truncate(8);
    settings.window_width = settings.window_width.clamp(480, 10_000);
    settings.window_height = settings.window_height.clamp(360, 10_000);
    settings.pane_position = settings.pane_position.clamp(0, 10_000);
    settings.version = 2;
    Ok(settings)
}

pub fn load_session(path: &Path) -> io::Result<Session> {
    let text = read_utf8(path)?;
    serde_json::from_str(&text)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "session file is malformed"))
}

pub fn save_session(path: &Path, session: &Session) -> io::Result<()> {
    let bytes = serde_json::to_vec(session)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "session could not be encoded"))?;
    atomic_write_private(path, &bytes, true)
}

pub fn load_learned(path: &Path) -> io::Result<Learned> {
    if !path.exists() {
        return Ok(Learned::default());
    }
    let text = read_utf8(path)?;
    let mut learned: Learned = serde_json::from_str(&text).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "learned words file is malformed",
        )
    })?;
    learned.choices.retain(|namespace, map| {
        map.retain(|key, value| key.chars().count() <= 128 && value.chars().count() <= 256);
        namespace.len() <= 32 && !map.is_empty()
    });
    learned.version = 1;
    Ok(learned)
}

pub fn save_learned(path: &Path, learned: &Learned) -> io::Result<()> {
    let mut learned = learned.clone();
    learned.version = 1;
    let bytes = serde_json::to_vec_pretty(&learned).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "learned words could not be encoded",
        )
    })?;
    atomic_write_private(path, &bytes, true)
}

pub fn save_settings(path: &Path, settings: &Settings) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(settings)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "settings could not be encoded"))?;
    atomic_write_private(path, &bytes, true)
}

pub fn write_recovery(path: &Path, text: &str) -> io::Result<()> {
    atomic_write_private(path, text.as_bytes(), true)
}

pub fn clear_recovery(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

pub fn private_create(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_is_atomic_private_and_refuses_clobber() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("doc.txt");
        atomic_write_private(&path, b"hello", false).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            atomic_write_private(&path, b"bad", false)
                .unwrap_err()
                .kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read_to_string(path).unwrap(), "hello");
    }

    #[test]
    fn old_settings_files_still_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(
            &path,
            r#"{"version":1,"language":"kn","font_size":18,"show_keyboard":true,"theme":"system","autosave":true,"learning_enabled":true,"chinese_variant":"simplified","japanese_mode":"hiragana"}"#,
        )
        .unwrap();
        let settings = load_settings(&path).unwrap();
        assert_eq!(settings.language, "kn");
        assert_eq!(settings.version, 2);
        assert!(settings.recent_languages.is_empty());
    }

    #[test]
    fn session_and_learned_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let session_path = dir.path().join("session.json");
        let session = Session {
            version: 1,
            language: "hi".into(),
            roman: "namaste".into(),
            output: "नमस्ते".into(),
            auto_output: "नमस्ते".into(),
            ..Session::default()
        };
        save_session(&session_path, &session).unwrap();
        assert_eq!(load_session(&session_path).unwrap(), session);

        let learned_path = dir.path().join("learned.json");
        let mut learned = Learned::default();
        learned.set("hi", "abhisht", "अभीष्ट");
        save_learned(&learned_path, &learned).unwrap();
        let loaded = load_learned(&learned_path).unwrap();
        assert_eq!(
            loaded.get("hi", "abhisht").map(String::as_str),
            Some("अभीष्ट")
        );
        let mut loaded = loaded;
        assert!(loaded.remove("hi", "abhisht"));
        assert!(loaded.is_empty());
    }

    #[test]
    fn malformed_settings_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, "{not-json}").unwrap();
        assert_eq!(
            load_settings(&path).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
}
