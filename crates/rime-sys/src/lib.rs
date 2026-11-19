#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::undocumented_unsafe_blocks)]

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;
use std::sync::{Mutex, OnceLock};

type RimeSessionId = usize;
type Bool = c_int;

#[repr(C)]
struct RimeTraits {
    data_size: c_int,
    shared_data_dir: *const c_char,
    user_data_dir: *const c_char,
    distribution_name: *const c_char,
    distribution_code_name: *const c_char,
    distribution_version: *const c_char,
    app_name: *const c_char,
    modules: *const *const c_char,
    min_log_level: c_int,
    log_dir: *const c_char,
    prebuilt_data_dir: *const c_char,
    staging_dir: *const c_char,
}

#[repr(C)]
struct RimeComposition {
    length: c_int,
    cursor_pos: c_int,
    sel_start: c_int,
    sel_end: c_int,
    preedit: *mut c_char,
}

#[repr(C)]
struct RimeCandidate {
    text: *mut c_char,
    comment: *mut c_char,
    reserved: *mut c_void,
}

#[repr(C)]
struct RimeMenu {
    page_size: c_int,
    page_no: c_int,
    is_last_page: Bool,
    highlighted_candidate_index: c_int,
    num_candidates: c_int,
    candidates: *mut RimeCandidate,
    select_keys: *mut c_char,
}

#[repr(C)]
struct RimeCommit {
    data_size: c_int,
    text: *mut c_char,
}

#[repr(C)]
struct RimeContext {
    data_size: c_int,
    composition: RimeComposition,
    menu: RimeMenu,
    commit_text_preview: *mut c_char,
    select_labels: *mut *mut c_char,
}

#[link(name = "rime")]
extern "C" {
    fn RimeSetup(traits: *mut RimeTraits);
    fn RimeInitialize(traits: *mut RimeTraits);
    fn RimeStartMaintenance(full_check: Bool) -> Bool;
    fn RimeJoinMaintenanceThread();
    fn RimeCreateSession() -> RimeSessionId;
    fn RimeDestroySession(session_id: RimeSessionId) -> Bool;
    fn RimeProcessKey(session_id: RimeSessionId, keycode: c_int, mask: c_int) -> Bool;
    fn RimeClearComposition(session_id: RimeSessionId);
    fn RimeGetCommit(session_id: RimeSessionId, commit: *mut RimeCommit) -> Bool;
    fn RimeFreeCommit(commit: *mut RimeCommit) -> Bool;
    fn RimeGetContext(session_id: RimeSessionId, context: *mut RimeContext) -> Bool;
    fn RimeFreeContext(context: *mut RimeContext) -> Bool;
    fn RimeSelectCandidateOnCurrentPage(session_id: RimeSessionId, index: usize) -> Bool;
    fn RimeSelectSchema(session_id: RimeSessionId, schema_id: *const c_char) -> Bool;
}

static INITIALIZED: OnceLock<Result<(), String>> = OnceLock::new();
static RIME_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RawSnapshot {
    pub preedit: String,
    pub cursor: usize,
    pub page: usize,
    pub page_size: usize,
    pub highlighted: usize,
    pub candidates: Vec<String>,
    pub committed: Option<String>,
}

pub struct Session {
    id: RimeSessionId,
}

impl Session {
    pub fn new(shared_data: &Path, user_data: &Path, schema: &str) -> Result<Self, String> {
        let shared = path_to_cstring(shared_data)?;
        let user = path_to_cstring(user_data)?;
        let prebuilt = path_to_cstring(&shared_data.join("build"))?;

        let initialized = INITIALIZED.get_or_init(|| {
            let shared = Box::leak(Box::new(shared));
            let user = Box::leak(Box::new(user));
            let prebuilt = Box::leak(Box::new(prebuilt));
            let distribution =
                Box::leak(Box::new(CString::new("Universal Transliterator").unwrap()));
            let code_name = Box::leak(Box::new(CString::new("universal-transliterator").unwrap()));
            let version = Box::leak(Box::new(CString::new(env!("CARGO_PKG_VERSION")).unwrap()));
            let app_name = Box::leak(Box::new(
                CString::new("rime.universal-transliterator").unwrap(),
            ));
            let log_dir = Box::leak(Box::new(CString::new("").unwrap()));
            let mut traits = RimeTraits {
                data_size: (std::mem::size_of::<RimeTraits>() - std::mem::size_of::<c_int>())
                    as c_int,
                shared_data_dir: shared.as_ptr(),
                user_data_dir: user.as_ptr(),
                distribution_name: distribution.as_ptr(),
                distribution_code_name: code_name.as_ptr(),
                distribution_version: version.as_ptr(),
                app_name: app_name.as_ptr(),
                modules: ptr::null(),
                min_log_level: 2,
                log_dir: log_dir.as_ptr(),
                prebuilt_data_dir: prebuilt.as_ptr(),
                staging_dir: ptr::null(),
            };
            let _guard = RIME_LOCK
                .lock()
                .map_err(|_| "Rime lock was poisoned".to_string())?;
            // SAFETY: every pointer in traits is process-lifetime storage and the struct
            // exactly mirrors librime 1.x's public C ABI.
            unsafe {
                RimeSetup(&mut traits);
                RimeInitialize(&mut traits);
                if RimeStartMaintenance(0) != 0 {
                    RimeJoinMaintenanceThread();
                }
            }
            Ok(())
        });
        initialized.clone()?;

        let schema =
            CString::new(schema).map_err(|_| "schema id contains a NUL byte".to_string())?;
        let _guard = RIME_LOCK
            .lock()
            .map_err(|_| "Rime lock was poisoned".to_string())?;
        // SAFETY: librime was initialized above; the returned identifier is checked.
        let id = unsafe { RimeCreateSession() };
        if id == 0 {
            return Err("Rime could not create an input session".into());
        }
        // SAFETY: id is a live session and schema is a valid NUL-terminated string.
        if unsafe { RimeSelectSchema(id, schema.as_ptr()) } == 0 {
            // SAFETY: id was just created and has not been destroyed.
            unsafe { RimeDestroySession(id) };
            return Err(format!("Rime schema '{schema:?}' is unavailable"));
        }
        Ok(Self { id })
    }

    pub fn process_key(&mut self, keycode: i32) -> Result<RawSnapshot, String> {
        let _guard = RIME_LOCK
            .lock()
            .map_err(|_| "Rime lock was poisoned".to_string())?;
        // SAFETY: self owns a live session and keycode/mask are scalar values.
        unsafe { RimeProcessKey(self.id, keycode, 0) };
        self.snapshot_locked()
    }

    pub fn select_candidate(&mut self, index_on_page: usize) -> Result<RawSnapshot, String> {
        let _guard = RIME_LOCK
            .lock()
            .map_err(|_| "Rime lock was poisoned".to_string())?;
        // SAFETY: self owns a live session. librime bounds-checks the candidate index.
        if unsafe { RimeSelectCandidateOnCurrentPage(self.id, index_on_page) } == 0 {
            return Err("Rime rejected the candidate selection".into());
        }
        self.snapshot_locked()
    }

    pub fn clear(&mut self) -> Result<RawSnapshot, String> {
        let _guard = RIME_LOCK
            .lock()
            .map_err(|_| "Rime lock was poisoned".to_string())?;
        // SAFETY: self owns a live session.
        unsafe { RimeClearComposition(self.id) };
        self.snapshot_locked()
    }

    pub fn snapshot(&mut self) -> Result<RawSnapshot, String> {
        let _guard = RIME_LOCK
            .lock()
            .map_err(|_| "Rime lock was poisoned".to_string())?;
        self.snapshot_locked()
    }

    fn snapshot_locked(&self) -> Result<RawSnapshot, String> {
        let mut result = RawSnapshot::default();
        let mut commit = RimeCommit {
            data_size: (std::mem::size_of::<RimeCommit>() - std::mem::size_of::<c_int>()) as c_int,
            text: ptr::null_mut(),
        };
        // SAFETY: commit is initialized according to librime's documented ABI.
        if unsafe { RimeGetCommit(self.id, &mut commit) } != 0 {
            // SAFETY: a successful RimeGetCommit returns a valid optional C string.
            result.committed = unsafe { string_from_ptr(commit.text) };
            // SAFETY: commit was filled by RimeGetCommit and is released exactly once.
            unsafe { RimeFreeCommit(&mut commit) };
        }

        let mut context = RimeContext {
            data_size: (std::mem::size_of::<RimeContext>() - std::mem::size_of::<c_int>()) as c_int,
            composition: RimeComposition {
                length: 0,
                cursor_pos: 0,
                sel_start: 0,
                sel_end: 0,
                preedit: ptr::null_mut(),
            },
            menu: RimeMenu {
                page_size: 0,
                page_no: 0,
                is_last_page: 0,
                highlighted_candidate_index: 0,
                num_candidates: 0,
                candidates: ptr::null_mut(),
                select_keys: ptr::null_mut(),
            },
            commit_text_preview: ptr::null_mut(),
            select_labels: ptr::null_mut(),
        };
        // SAFETY: context is initialized according to librime's documented ABI.
        if unsafe { RimeGetContext(self.id, &mut context) } != 0 {
            // SAFETY: a successful RimeGetContext owns a valid optional preedit string.
            let preedit = unsafe { string_from_ptr(context.composition.preedit) };
            result.preedit = preedit.unwrap_or_default();
            result.cursor = usize::try_from(context.composition.cursor_pos).unwrap_or(0);
            result.page = usize::try_from(context.menu.page_no).unwrap_or(0);
            result.page_size = usize::try_from(context.menu.page_size).unwrap_or(0);
            result.highlighted =
                usize::try_from(context.menu.highlighted_candidate_index).unwrap_or(0);
            let count = usize::try_from(context.menu.num_candidates)
                .unwrap_or(0)
                .min(64);
            if !context.menu.candidates.is_null() {
                // SAFETY: librime guarantees num_candidates contiguous entries while
                // the context is live; count is additionally capped.
                let candidates =
                    unsafe { std::slice::from_raw_parts(context.menu.candidates, count) };
                result.candidates = candidates
                    .iter()
                    .filter_map(|candidate| {
                        // SAFETY: each candidate in the live context has an optional C string.
                        unsafe { string_from_ptr(candidate.text) }
                    })
                    .collect();
            }
            // SAFETY: context was filled by RimeGetContext and is released exactly once.
            unsafe { RimeFreeContext(&mut context) };
        }
        Ok(result)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Ok(_guard) = RIME_LOCK.lock() {
            // SAFETY: self uniquely owns the live session id, and Drop runs once.
            unsafe { RimeDestroySession(self.id) };
        }
    }
}

fn path_to_cstring(path: &Path) -> Result<CString, String> {
    CString::new(path.as_os_str().as_bytes()).map_err(|_| "path contains a NUL byte".to_string())
}

unsafe fn string_from_ptr(value: *const c_char) -> Option<String> {
    if value.is_null() {
        None
    } else {
        // SAFETY: callers only pass librime-owned NUL-terminated UTF-8 string pointers.
        let value = unsafe { CStr::from_ptr(value) };
        Some(value.to_string_lossy().into_owned())
    }
}

// X11 keysyms used by both GTK and librime.
pub const KEY_BACKSPACE: i32 = 0xff08;
pub const KEY_ENTER: i32 = 0xff0d;
pub const KEY_ESCAPE: i32 = 0xff1b;
pub const KEY_PAGE_UP: i32 = 0xff55;
pub const KEY_PAGE_DOWN: i32 = 0xff56;
pub const KEY_LEFT: i32 = 0xff51;
pub const KEY_UP: i32 = 0xff52;
pub const KEY_RIGHT: i32 = 0xff53;
pub const KEY_DOWN: i32 = 0xff54;
