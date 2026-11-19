#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::undocumented_unsafe_blocks)]

use std::ffi::{c_char, c_int, c_uint, c_void, CStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;
use std::sync::Once;

enum KkcLanguageModel {}
enum KkcLanguageModelMetadata {}
enum KkcContext {}
enum KkcSystemSegmentDictionary {}
enum KkcDictionary {}
enum KkcDictionaryList {}
enum KkcKeyEvent {}
enum KkcCandidateList {}
enum KkcCandidate {}
enum KkcSegmentList {}
enum KkcRuleMetadata {}
enum KkcRule {}

#[repr(C)]
struct GError {
    domain: c_uint,
    code: c_int,
    message: *mut c_char,
}

#[link(name = "kkc")]
extern "C" {
    fn kkc_init();
    fn kkc_language_model_load(
        name: *const c_char,
        error: *mut *mut GError,
    ) -> *mut KkcLanguageModel;
    fn kkc_language_model_metadata_new(
        name: *const c_char,
        filename: *const c_char,
        error: *mut *mut GError,
    ) -> *mut KkcLanguageModelMetadata;
    fn kkc_language_model_metadata_create_language_model(
        metadata: *mut KkcLanguageModelMetadata,
        error: *mut *mut GError,
    ) -> *mut KkcLanguageModel;
    fn kkc_context_new(model: *mut KkcLanguageModel) -> *mut KkcContext;
    fn kkc_rule_metadata_new(
        name: *const c_char,
        filename: *const c_char,
        error: *mut *mut GError,
    ) -> *mut KkcRuleMetadata;
    fn kkc_rule_new(metadata: *mut KkcRuleMetadata, error: *mut *mut GError) -> *mut KkcRule;
    fn kkc_context_set_typing_rule(context: *mut KkcContext, rule: *mut KkcRule);
    fn kkc_context_set_input_mode(context: *mut KkcContext, mode: c_int);
    fn kkc_system_segment_dictionary_new(
        path: *const c_char,
        encoding: *const c_char,
        error: *mut *mut GError,
    ) -> *mut KkcSystemSegmentDictionary;
    fn kkc_dictionary_list_new() -> *mut KkcDictionaryList;
    fn kkc_dictionary_list_add(list: *mut KkcDictionaryList, dictionary: *mut KkcDictionary);
    fn kkc_context_set_dictionaries(context: *mut KkcContext, list: *mut KkcDictionaryList);
    fn kkc_key_event_new(keyval: c_uint, keycode: c_uint, modifiers: c_int) -> *mut KkcKeyEvent;
    fn kkc_context_process_key_event(context: *mut KkcContext, event: *mut KkcKeyEvent) -> c_int;
    fn kkc_context_reset(context: *mut KkcContext);
    fn kkc_context_has_output(context: *mut KkcContext) -> c_int;
    fn kkc_context_poll_output(context: *mut KkcContext) -> *mut c_char;
    fn kkc_context_get_candidates(context: *mut KkcContext) -> *mut KkcCandidateList;
    fn kkc_context_get_segments(context: *mut KkcContext) -> *mut KkcSegmentList;
    fn kkc_context_get_input(context: *mut KkcContext) -> *mut c_char;
    fn kkc_context_get_input_cursor_pos(context: *mut KkcContext) -> c_int;
    fn kkc_segment_list_get_output(segments: *mut KkcSegmentList) -> *mut c_char;
    fn kkc_segment_list_get_size(segments: *mut KkcSegmentList) -> c_int;
    fn kkc_candidate_list_get(list: *mut KkcCandidateList, index: c_int) -> *mut KkcCandidate;
    fn kkc_candidate_list_select_at(list: *mut KkcCandidateList, index_in_page: c_uint) -> c_int;
    fn kkc_candidate_list_get_cursor_pos(list: *mut KkcCandidateList) -> c_int;
    fn kkc_candidate_list_get_size(list: *mut KkcCandidateList) -> c_int;
    fn kkc_candidate_list_get_page_start(list: *mut KkcCandidateList) -> c_uint;
    fn kkc_candidate_list_get_page_size(list: *mut KkcCandidateList) -> c_uint;
    fn kkc_candidate_get_output(candidate: *mut KkcCandidate) -> *const c_char;
}

#[link(name = "gobject-2.0")]
extern "C" {
    fn g_object_unref(object: *mut c_void);
}

#[link(name = "glib-2.0")]
extern "C" {
    fn g_free(memory: *mut c_void);
    fn g_error_free(error: *mut GError);
}

static INITIALIZE: Once = Once::new();

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RawSnapshot {
    pub input: String,
    pub preedit: String,
    pub cursor: usize,
    pub page_start: usize,
    pub page_size: usize,
    pub highlighted: usize,
    pub candidates: Vec<String>,
    pub committed: Option<String>,
}

pub struct Session {
    context: *mut KkcContext,
    model: *mut KkcLanguageModel,
    model_metadata: *mut KkcLanguageModelMetadata,
    rule: *mut KkcRule,
    rule_metadata: *mut KkcRuleMetadata,
    dictionaries: *mut KkcDictionaryList,
    system_dictionary: *mut KkcSystemSegmentDictionary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum InputMode {
    Hiragana = 0,
    Katakana = 1,
}

impl Session {
    pub fn new(dictionary_path: &Path) -> Result<Self, String> {
        Self::new_with_data(dictionary_path, None, None, InputMode::Hiragana)
    }

    pub fn new_with_data(
        dictionary_path: &Path,
        model_metadata_path: Option<&Path>,
        rule_metadata_path: Option<&Path>,
        input_mode: InputMode,
    ) -> Result<Self, String> {
        INITIALIZE.call_once(|| {
            // SAFETY: libkkc documents kkc_init as the process-wide initializer.
            unsafe { kkc_init() };
        });
        // Convert every caller-controlled path before allocating native objects so
        // an embedded NUL cannot cause an early-return leak later in construction.
        let dictionary_path = path_to_cstring(dictionary_path, "Japanese dictionary")?;
        let model_metadata_filename = model_metadata_path
            .map(|path| path_to_cstring(path, "Japanese model metadata"))
            .transpose()?;
        let rule_metadata_filename = rule_metadata_path
            .map(|path| path_to_cstring(path, "Japanese typing-rule metadata"))
            .transpose()?;
        let mut error = ptr::null_mut();
        let model_name = CString::new("sorted3").unwrap();
        let mut model_metadata = ptr::null_mut();
        let model = if let Some(path) = model_metadata_filename.as_ref() {
            // SAFETY: both strings are valid for the call and error is an out-pointer.
            model_metadata = unsafe {
                kkc_language_model_metadata_new(model_name.as_ptr(), path.as_ptr(), &mut error)
            };
            if model_metadata.is_null() {
                return Err(take_error(error, "could not load libkkc model metadata"));
            }
            error = ptr::null_mut();
            // SAFETY: model_metadata is a live object returned immediately above.
            unsafe { kkc_language_model_metadata_create_language_model(model_metadata, &mut error) }
        } else {
            // SAFETY: model_name is a valid NUL-terminated string; error is an out-pointer.
            unsafe { kkc_language_model_load(model_name.as_ptr(), &mut error) }
        };
        if model.is_null() {
            if !model_metadata.is_null() {
                // SAFETY: metadata is a live GObject owned by this constructor.
                unsafe { g_object_unref(model_metadata.cast()) };
            }
            return Err(take_error(
                error,
                "could not load libkkc's sorted3 language model",
            ));
        }

        // SAFETY: model is a valid live GObject returned above.
        let context = unsafe { kkc_context_new(model) };
        if context.is_null() {
            // SAFETY: objects are live GObjects and have not yet been released.
            unsafe {
                g_object_unref(model.cast());
                if !model_metadata.is_null() {
                    g_object_unref(model_metadata.cast());
                }
            }
            return Err("libkkc could not create a conversion context".into());
        }

        let mut rule_metadata = ptr::null_mut();
        let mut rule = ptr::null_mut();
        if let Some(path) = rule_metadata_filename.as_ref() {
            let name = CString::new("default").unwrap();
            error = ptr::null_mut();
            // SAFETY: strings are valid for the call and error is an out-pointer.
            let loaded_rule_metadata =
                unsafe { kkc_rule_metadata_new(name.as_ptr(), path.as_ptr(), &mut error) };
            rule_metadata = loaded_rule_metadata;
            if rule_metadata.is_null() {
                let message = take_error(error, "could not load libkkc typing-rule metadata");
                // SAFETY: objects are live and owned by this constructor.
                unsafe {
                    g_object_unref(context.cast());
                    g_object_unref(model.cast());
                    if !model_metadata.is_null() {
                        g_object_unref(model_metadata.cast());
                    }
                }
                return Err(message);
            }
            error = ptr::null_mut();
            // SAFETY: rule_metadata is live and error is an out-pointer.
            rule = unsafe { kkc_rule_new(rule_metadata, &mut error) };
            if rule.is_null() {
                let message = take_error(error, "could not load libkkc typing rule");
                // SAFETY: objects are live and owned by this constructor.
                unsafe {
                    g_object_unref(rule_metadata.cast());
                    g_object_unref(context.cast());
                    g_object_unref(model.cast());
                    if !model_metadata.is_null() {
                        g_object_unref(model_metadata.cast());
                    }
                }
                return Err(message);
            }
            // SAFETY: context and rule are live; the property setter retains the rule.
            unsafe { kkc_context_set_typing_rule(context, rule) };
        }
        // SAFETY: context is live and input_mode is a valid KkcInputMode discriminant.
        unsafe { kkc_context_set_input_mode(context, input_mode as c_int) };

        let encoding = CString::new("EUC-JP").unwrap();
        error = ptr::null_mut();
        // SAFETY: both C strings remain live for the duration of the call.
        let system_dictionary = unsafe {
            kkc_system_segment_dictionary_new(
                dictionary_path.as_ptr(),
                encoding.as_ptr(),
                &mut error,
            )
        };
        if system_dictionary.is_null() {
            let message = take_error(error, "could not load the Japanese SKK dictionary");
            // SAFETY: these are live GObjects owned by this constructor.
            unsafe {
                g_object_unref(context.cast());
                g_object_unref(model.cast());
                if !rule.is_null() {
                    g_object_unref(rule.cast());
                }
                if !rule_metadata.is_null() {
                    g_object_unref(rule_metadata.cast());
                }
                if !model_metadata.is_null() {
                    g_object_unref(model_metadata.cast());
                }
            }
            return Err(message);
        }

        // SAFETY: constructor has no arguments and returns a new GObject.
        let dictionaries = unsafe { kkc_dictionary_list_new() };
        if dictionaries.is_null() {
            // SAFETY: all objects are live and owned by this constructor.
            unsafe {
                g_object_unref(system_dictionary.cast());
                g_object_unref(context.cast());
                g_object_unref(model.cast());
                if !rule.is_null() {
                    g_object_unref(rule.cast());
                }
                if !rule_metadata.is_null() {
                    g_object_unref(rule_metadata.cast());
                }
                if !model_metadata.is_null() {
                    g_object_unref(model_metadata.cast());
                }
            }
            return Err("libkkc could not create a dictionary list".into());
        }
        // SAFETY: the system dictionary implements KkcDictionary; libkkc retains
        // references when adding the dictionary and assigning the list.
        unsafe {
            kkc_dictionary_list_add(dictionaries, system_dictionary.cast());
            kkc_context_set_dictionaries(context, dictionaries);
        }
        Ok(Self {
            context,
            model,
            model_metadata,
            rule,
            rule_metadata,
            dictionaries,
            system_dictionary,
        })
    }

    pub fn process_key(&mut self, keyval: u32) -> Result<RawSnapshot, String> {
        self.send_key(keyval)?;
        self.snapshot()
    }

    pub fn select_candidate(&mut self, index_on_page: usize) -> Result<RawSnapshot, String> {
        // SAFETY: context is live and returns a borrowed candidate-list pointer.
        let list = unsafe { kkc_context_get_candidates(self.context) };
        if list.is_null() {
            return Err("no Japanese candidate list is active".into());
        }
        let index =
            u32::try_from(index_on_page).map_err(|_| "candidate index is too large".to_string())?;
        // SAFETY: list is live; libkkc bounds-checks index_on_page.
        if unsafe { kkc_candidate_list_select_at(list, index) } == 0 {
            return Err("libkkc rejected the candidate selection".into());
        }
        self.send_key(KEY_ENTER)?;
        self.snapshot()
    }

    /// Highlights a candidate of the focused segment without committing, so
    /// the preedit shows the whole sentence with that choice.
    pub fn preview_candidate(&mut self, index_on_page: usize) -> Result<RawSnapshot, String> {
        // SAFETY: context is live and returns a borrowed candidate-list pointer.
        let list = unsafe { kkc_context_get_candidates(self.context) };
        if list.is_null() {
            return Err("no Japanese candidate list is active".into());
        }
        let index =
            u32::try_from(index_on_page).map_err(|_| "candidate index is too large".to_string())?;
        // SAFETY: list is live; libkkc bounds-checks index_on_page.
        if unsafe { kkc_candidate_list_select_at(list, index) } == 0 {
            return Err("libkkc rejected the candidate selection".into());
        }
        self.snapshot()
    }

    /// Number of conversion segments (bunsetsu) in the current sentence.
    pub fn segment_count(&mut self) -> usize {
        // SAFETY: context is live and returns a borrowed segment-list pointer.
        let segments = unsafe { kkc_context_get_segments(self.context) };
        if segments.is_null() {
            return 0;
        }
        // SAFETY: segments is a live list owned by the context.
        let size = unsafe { kkc_segment_list_get_size(segments) };
        usize::try_from(size).unwrap_or(0)
    }

    pub fn reset(&mut self) -> Result<RawSnapshot, String> {
        // SAFETY: context is live and uniquely accessed through &mut self.
        unsafe { kkc_context_reset(self.context) };
        self.snapshot()
    }

    pub fn snapshot(&mut self) -> Result<RawSnapshot, String> {
        let mut result = RawSnapshot::default();
        // SAFETY: context is live; returned string is newly allocated with g_malloc.
        let input = unsafe { kkc_context_get_input(self.context) };
        // SAFETY: ownership of input was transferred by kkc_context_get_input.
        result.input = unsafe { take_owned_string(input) };
        // SAFETY: context is live and returns a borrowed segment-list pointer.
        let segments = unsafe { kkc_context_get_segments(self.context) };
        if !segments.is_null() {
            // SAFETY: segments is live; the returned string is newly allocated.
            result.preedit = unsafe { take_owned_string(kkc_segment_list_get_output(segments)) };
        }
        if result.preedit.is_empty() {
            result.preedit = result.input.clone();
        }
        // SAFETY: context is live and the accessor returns a scalar cursor position.
        let cursor = unsafe { kkc_context_get_input_cursor_pos(self.context) };
        result.cursor = usize::try_from(cursor).unwrap_or(0);

        // SAFETY: context is live and returns a borrowed candidate-list pointer.
        let list = unsafe { kkc_context_get_candidates(self.context) };
        if !list.is_null() {
            // SAFETY: all accessors read the live list.
            let size = usize::try_from(unsafe { kkc_candidate_list_get_size(list) })
                .unwrap_or(0)
                .min(64);
            // SAFETY: list is live and each accessor returns scalar page metadata.
            let page_start = unsafe { kkc_candidate_list_get_page_start(list) };
            // SAFETY: list is live and each accessor returns scalar page metadata.
            let page_size = unsafe { kkc_candidate_list_get_page_size(list) };
            // SAFETY: list is live and each accessor returns scalar page metadata.
            let cursor = unsafe { kkc_candidate_list_get_cursor_pos(list) };
            result.page_start = page_start as usize;
            result.page_size = page_size as usize;
            result.highlighted = usize::try_from(cursor)
                .unwrap_or(result.page_start)
                .saturating_sub(result.page_start);
            // libkkc's get(index) is relative to the currently exposed candidate
            // collection even though page_start describes the logical global offset.
            // Reading 0..size matches the public Vala API and avoids mixing the two.
            for index in 0..size {
                // SAFETY: index is strictly below the reported/capped list size.
                let candidate = unsafe { kkc_candidate_list_get(list, index as c_int) };
                if !candidate.is_null() {
                    // SAFETY: candidate is a live owned reference and text is NUL-terminated.
                    let value = unsafe { borrowed_string(kkc_candidate_get_output(candidate)) };
                    if let Some(value) = value {
                        result.candidates.push(value);
                    }
                    // SAFETY: candidate_list_get transfers a new GObject reference.
                    unsafe { g_object_unref(candidate.cast()) };
                }
            }
        }

        // SAFETY: context is live; poll_output returns a newly allocated string when present.
        if unsafe { kkc_context_has_output(self.context) } != 0 {
            // SAFETY: has_output is true, so poll_output returns an owned optional string.
            let committed = unsafe { take_owned_string(kkc_context_poll_output(self.context)) };
            result.committed = Some(committed);
        }
        Ok(result)
    }

    fn send_key(&mut self, keyval: u32) -> Result<(), String> {
        // SAFETY: scalar arguments are valid; returned event is owned by this call.
        let event = unsafe { kkc_key_event_new(keyval, 0, 0) };
        if event.is_null() {
            return Err("libkkc could not create a key event".into());
        }
        // SAFETY: context and event are live for the duration of the call.
        let handled = unsafe { kkc_context_process_key_event(self.context, event) };
        // SAFETY: event is a live GObject released exactly once.
        unsafe { g_object_unref(event.cast()) };
        if handled == 0 {
            return Err("libkkc did not handle that key".into());
        }
        Ok(())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // SAFETY: the four GObjects are non-null, owned by this Session, and each is
        // released exactly once. Context is released first to drop its retained refs.
        unsafe {
            g_object_unref(self.context.cast());
            g_object_unref(self.dictionaries.cast());
            g_object_unref(self.system_dictionary.cast());
            if !self.rule.is_null() {
                g_object_unref(self.rule.cast());
            }
            if !self.rule_metadata.is_null() {
                g_object_unref(self.rule_metadata.cast());
            }
            g_object_unref(self.model.cast());
            if !self.model_metadata.is_null() {
                g_object_unref(self.model_metadata.cast());
            }
        }
    }
}

fn path_to_cstring(path: &Path, label: &str) -> Result<CString, String> {
    CString::new(path.as_os_str().as_bytes())
        .map_err(|_| format!("{label} path contains a NUL byte"))
}

fn take_error(error: *mut GError, fallback: &str) -> String {
    if error.is_null() {
        return fallback.to_owned();
    }
    // SAFETY: error is a live GError from a failed GLib API call.
    let native_message = unsafe { borrowed_string((*error).message) };
    let message = native_message.unwrap_or_else(|| fallback.to_owned());
    // SAFETY: the GError is owned by the caller and released exactly once.
    unsafe { g_error_free(error) };
    message
}

unsafe fn borrowed_string(value: *const c_char) -> Option<String> {
    if value.is_null() {
        None
    } else {
        // SAFETY: callers pass libkkc/GLib-owned NUL-terminated strings.
        let value = unsafe { CStr::from_ptr(value) };
        Some(value.to_string_lossy().into_owned())
    }
}

unsafe fn take_owned_string(value: *mut c_char) -> String {
    if value.is_null() {
        return String::new();
    }
    // SAFETY: value is a GLib-allocated NUL-terminated string.
    let result = unsafe { CStr::from_ptr(value) }
        .to_string_lossy()
        .into_owned();
    // SAFETY: ownership of value was transferred to the caller by libkkc.
    unsafe { g_free(value.cast()) };
    result
}

pub const KEY_BACKSPACE: u32 = 0xff08;
pub const KEY_ENTER: u32 = 0xff0d;
pub const KEY_ESCAPE: u32 = 0xff1b;
pub const KEY_LEFT: u32 = 0xff51;
pub const KEY_UP: u32 = 0xff52;
pub const KEY_RIGHT: u32 = 0xff53;
pub const KEY_DOWN: u32 = 0xff54;
pub const KEY_PAGE_UP: u32 = 0xff55;
pub const KEY_PAGE_DOWN: u32 = 0xff56;
