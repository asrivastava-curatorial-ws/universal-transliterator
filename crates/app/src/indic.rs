#![forbid(unsafe_code)]
//! Client for the private, offline IndicXlit worker (Node + ONNX Runtime).
//!
//! The worker is a child process spoken to over stdin/stdout, one JSON line
//! per request. There is no socket or localhost server. Requests are batches
//! of unique words; answers are cached by the caller.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};

/// Number of alternatives requested per word.
pub const CANDIDATE_COUNT: u8 = 5;
pub const MAX_WORDS_PER_REQUEST: usize = 512;

#[derive(Debug, Serialize)]
struct Request {
    id: u64,
    lang: String,
    words: Vec<String>,
    count: u8,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Response {
    pub id: u64,
    pub ok: bool,
    pub lang: String,
    pub words: HashMap<String, Vec<String>>,
    pub error: String,
}

pub struct Client {
    requests: Sender<Request>,
    responses: Receiver<Response>,
    next_id: u64,
}

impl Client {
    /// Starts the worker thread. The Node process and the model start with
    /// the first request, in the background; the first answer arrives once
    /// the model is ready.
    pub fn start() -> Self {
        let (request_tx, request_rx) = mpsc::channel();
        let (response_tx, response_rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("indicxlit-worker".into())
            .spawn(move || worker_loop(request_rx, response_tx));
        if let Err(error) = spawned {
            eprintln!("Universal Transliterator: could not start the IndicXlit thread: {error}");
        }
        Self {
            requests: request_tx,
            responses: response_rx,
            next_id: 1,
        }
    }

    /// Queues one batch of words (at most [`MAX_WORDS_PER_REQUEST`]) and
    /// returns its request id.
    pub fn request(&mut self, model: &str, words: Vec<String>) -> Option<u64> {
        let id = self.next_id;
        self.next_id += 1;
        let request = Request {
            id,
            lang: model.to_owned(),
            words,
            count: CANDIDATE_COUNT,
        };
        self.requests.send(request).ok().map(|()| id)
    }

    /// Returns a finished answer, if any (never blocks).
    pub fn try_response(&self) -> Option<Response> {
        self.responses.try_recv().ok()
    }
}

fn spawn_worker() -> std::io::Result<Child> {
    let root = resource_root();
    Command::new(node_binary(&root))
        .arg(root.join("worker.js"))
        .env("UT_INDIC_MODEL_DIR", root.join("models"))
        .env("UT_ONNX_BINDING", binding_path(&root))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
}

fn worker_loop(requests: Receiver<Request>, responses: Sender<Response>) {
    // Node and the model start with the first request, so a session that
    // never uses an AI language does not pay ~180 MB of memory for them.
    let Ok(first) = requests.recv() else {
        return;
    };
    let mut child = match spawn_worker() {
        Ok(child) => child,
        Err(error) => {
            for request in std::iter::once(first).chain(requests) {
                let _ = responses.send(error_response(
                    request.id,
                    &request.lang,
                    format!("could not start the bundled AI engine ({error})"),
                ));
            }
            return;
        }
    };
    let (Some(mut input), Some(output)) = (child.stdin.take(), child.stdout.take()) else {
        let _ = child.kill();
        return;
    };
    let mut output = BufReader::new(output);
    let mut line = String::new();
    let mut next = Some(first);
    while let Some(request) = next.take().or_else(|| requests.recv().ok()) {
        let encoded = match serde_json::to_string(&request) {
            Ok(encoded) => encoded,
            Err(error) => {
                let _ =
                    responses.send(error_response(request.id, &request.lang, error.to_string()));
                continue;
            }
        };
        if writeln!(input, "{encoded}")
            .and_then(|()| input.flush())
            .is_err()
        {
            let _ = responses.send(error_response(
                request.id,
                &request.lang,
                "the AI engine stopped".into(),
            ));
            break;
        }
        line.clear();
        if output
            .read_line(&mut line)
            .ok()
            .filter(|size| *size > 0)
            .is_none()
        {
            let _ = responses.send(error_response(
                request.id,
                &request.lang,
                "the AI engine stopped".into(),
            ));
            break;
        }
        let response = serde_json::from_str::<Response>(&line).unwrap_or_else(|error| {
            error_response(
                request.id,
                &request.lang,
                format!("invalid AI engine response: {error}"),
            )
        });
        if responses.send(response).is_err() {
            break;
        }
    }
    // Answer anything still queued so the UI never waits forever.
    while let Ok(request) = requests.try_recv() {
        let _ = responses.send(error_response(
            request.id,
            &request.lang,
            "the AI engine stopped".into(),
        ));
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn error_response(id: u64, lang: &str, error: String) -> Response {
    Response {
        id,
        ok: false,
        lang: lang.to_owned(),
        error,
        ..Response::default()
    }
}

/// Synchronously transliterates words (command line and self-test).
pub fn run_once(model: &str, words: &[String]) -> Result<HashMap<String, Vec<String>>, String> {
    let mut child =
        spawn_worker().map_err(|error| format!("could not start the AI engine: {error}"))?;
    let result = (|| {
        let mut input = child.stdin.take().ok_or("worker input is unavailable")?;
        let output = child.stdout.take().ok_or("worker output is unavailable")?;
        let mut output = BufReader::new(output);
        let mut results = HashMap::new();
        for (index, chunk) in words.chunks(MAX_WORDS_PER_REQUEST).enumerate() {
            let request = Request {
                id: index as u64 + 1,
                lang: model.to_owned(),
                words: chunk.to_vec(),
                count: CANDIDATE_COUNT,
            };
            let encoded = serde_json::to_string(&request).map_err(|error| error.to_string())?;
            writeln!(input, "{encoded}").map_err(|error| error.to_string())?;
            input.flush().map_err(|error| error.to_string())?;
            let mut line = String::new();
            output
                .read_line(&mut line)
                .map_err(|error| error.to_string())?;
            let response: Response = serde_json::from_str(&line)
                .map_err(|error| format!("invalid AI engine response: {error}"))?;
            if !response.ok {
                return Err(response.error);
            }
            results.extend(response.words);
        }
        Ok(results)
    })();
    let _ = child.kill();
    let _ = child.wait();
    result
}

fn executable_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
}

/// Location of the worker, models and ONNX Runtime.
pub fn resource_root() -> PathBuf {
    if let Some(value) = std::env::var_os("UT_INDIC_ROOT") {
        return PathBuf::from(value);
    }
    if let Some(directory) = executable_dir() {
        let installed = directory.join("../share/universal-transliterator/indic");
        if installed.join("worker.js").is_file() {
            return installed;
        }
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../resources/indic")
}

fn binding_path(root: &Path) -> PathBuf {
    root.join("onnxruntime/bin/napi-v3/linux/x64/onnxruntime_binding.node")
}

fn node_binary(root: &Path) -> PathBuf {
    if let Some(value) = std::env::var_os("UT_NODE_BINARY") {
        return PathBuf::from(value);
    }
    let bundled = root.join("node/bin/node");
    if bundled.is_file() {
        return bundled;
    }
    if let Some(directory) = executable_dir() {
        let installed = directory.join("../libexec/universal-transliterator/node");
        if installed.is_file() {
            return installed;
        }
    }
    std::env::var_os("PATH")
        .and_then(|path| {
            std::env::split_paths(&path)
                .map(|directory| directory.join("node"))
                .find(|candidate| candidate.is_file())
        })
        .unwrap_or_else(|| PathBuf::from("node"))
}

/// Whether the bundled model files are present.
pub fn model_available() -> bool {
    let root = resource_root();
    root.join("models/indicxlit_encoder.onnx").is_file() && binding_path(&root).is_file()
}
