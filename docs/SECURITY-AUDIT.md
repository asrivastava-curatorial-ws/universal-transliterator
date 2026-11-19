# Security audit

Date: 2026-10-17

Scope: the two identical submitted archives, the extracted Node/Express project,
and the replacement Rust workspace. The old application was inspected as data;
no `npm install`, lifecycle script, model initialization, or old application
code was executed.

## Submitted archive identity

| Item | Value |
|---|---|
| `Universal-Transliteration.tar.gz` | 34,202 bytes |
| Duplicate `Universal-Transliteration.tar(1).gz` | byte-identical |
| SHA-256 | `fb3fef7ef285fc86acc3493cf332390afae5ed62d00a99f340b41b4130674ae1` |
| Expanded size reported by gzip | 92,160 bytes |

## Complete original file inventory

| File | Bytes | Purpose |
|---|---:|---|
| `.gitignore` | 280 | ignores Node, editor, and hypothetical Tauri outputs |
| `LICENSE` | 1,084 | MIT template with an unfilled copyright placeholder |
| `README.md` | 2,772 | features, server instructions, and claimed Tauri wrapper |
| `package.json` | 435 | Node metadata and three direct dependencies |
| `server.js` | 3,646 | Express server, IndicXlit/ONNX calls, DOCX endpoint |
| `public/index.html` | 11,561 | complete browser UI and inline JavaScript controller |
| `public/keyboards.js` | 16,249 | on-screen layouts and Chinese hints |
| `public/style.css` | 3,938 | browser styling |
| `public/translit.js` | 43,315 | deterministic tables, Hangul/Kana/Telex logic, Pinyin table |

There is no `src-tauri/`, Rust manifest, Tauri configuration, sidecar, desktop
entry, lockfile, test tree, model file, font, or vendored dependency. The README
says a Tauri 2 wrapper was added, but then describes it as a future step. The
archive is only a Node/Express localhost web application.

## Original dependency and data inventory

`package.json` declares Node `>=18` and:

| Dependency | Declaration | Finding |
|---|---|---|
| `@cloudrumbles/indic-transliterate` | `1.2.2` | direct version is exact; transitive graph and model files are absent without a lockfile |
| `docx` | `^9.5.1` | mutable semver range |
| `express` | `^5.1.0` | mutable semver range |

There is no `package-lock.json`, npm shrinkwrap, integrity hash, SPDX inventory,
or captured license set. The README attributes the hand-curated Pinyin table to
CC-CEDICT under CC-BY-SA-4.0 but provides neither a source revision, extraction
procedure, data hash, nor a copy of that license. The IndicXlit model is obtained
indirectly from an npm package and its redistribution terms are deferred to the
reader. The MIT file itself retains `<YOUR NAME OR ORGANIZATION>` and therefore
does not establish usable provenance for the submitted code.

## Findings in the original application

### High: configurable network service without an application security boundary

`server.js` listens on `HOST` and `PORT`. The default is loopback, but setting
`HOST=0.0.0.0` or another interface exposes the service. There is no
authentication, authorization, CSRF defense, Origin validation, Host allowlist,
or request-rate limit. A malicious web page can cause browser requests to a
loopback service; JSON POSTs require a CORS preflight in common cases, but the
design should not rely on browser CORS as an authentication mechanism. Static
and API responses omit CSP, frame, referrer, MIME-sniffing, and related defense
headers.

### High: resource-amplification paths

The `/api/transliterate` request has a 10,000-character check, but every unique
ASCII word is sent concurrently with one unbounded `Promise.all`. An adversarial
document can create a large burst of ONNX work. `/api/health` performs model
initialization, so a nominal readiness request triggers expensive state. The
global 3,000-entry cache uses simple FIFO eviction, has no per-client fairness,
and retains generated candidate strings. DOCX generation builds the entire
archive in memory with `Packer.toBuffer`. Express's 100 KiB JSON limit is the
only bound on the export path.

### Medium: internal error disclosure

API exception messages are returned verbatim as JSON and logged. Native/model
errors may reveal paths, package internals, or configuration. The UI displays
the returned string directly. The replacement maps filesystem failures to
bounded user-facing categories and keeps document text out of logs.

### Medium: whole-document recomputation and destructive UI coupling

After a 220 ms debounce, the browser retransliterates the entire input and
assigns the whole output textarea. Deterministic modes run synchronously on the
browser UI thread. Indic words are gathered across the complete document on
each request. Direct edits in the output can be silently replaced by the next
input update; composition and committed text are not separate models.

### Medium: data-loss conditions

There is no native Open/Save, atomic write, dirty indicator, recovery journal,
autosave, recent-document metadata, or shutdown handling. Browser downloads do
not establish a document path. “New” checks only one input field; output-only
edits can be discarded. In-flight local deterministic conversion cannot be
cancelled, while server cancellation only stops the browser request and does not
guarantee cancellation of already-started model work.

### Medium: misleading/incomplete language claims

Mandarin is a small source-embedded table rather than a complete IME: it lacks
Rime segmentation, sentence-scale ranking, learned frequency management, full
schema behavior, and robust candidate paging. Japanese is deterministic kana
conversion without kanji candidates. Thai is picker-only. The README correctly
labels several distinctions, but the overall “Tauri wrapper” and “real
dictionary-backed” presentation overstates what is shipped. No automated
language acceptance tests exist.

### Low/medium: filesystem and deployment assumptions

`express.static("public")` depends on the process working directory. There is no
symlink policy, private XDG storage, path-independent bundled data lookup, or
AppImage. The proposed Tauri design would still require a separately running
Node/ONNX service.

### Positive observations

The server defaults to `127.0.0.1`, Express limits JSON to 100 KiB, language IDs
use an allowlist, the transliteration endpoint rejects more than 10,000
characters, the browser uses `textContent` for candidate/key labels, and the
print view escapes `&`, `<`, and `>` before `document.write`. These reduce risk
but do not remedy the architectural findings.

## Replacement security posture

| Control | Implementation |
|---|---|
| Network isolation | no HTTP listener, telemetry, updater, or runtime downloader; Indic inference is a private stdin/stdout child; a self-validating seccomp harness closes inherited descriptors, proves its IPv4/IPv6 socket trap, and runs the complete headless self-test beneath that filter |
| Unsafe isolation | all safe crates use `forbid(unsafe_code)`; only `rime-sys` and `kkc-sys` contain documented C-ABI blocks |
| Input bound | Roman input and worker IPC are bounded; stale neural results use generation IDs and are discarded |
| Document bound | regular, non-symlink UTF-8 files only; 16 MiB maximum |
| Storage privacy | XDG directories mode `0700`; settings, saves, and recovery are written through mode-`0600` temporary files and atomic rename |
| Export safety | XML characters are escaped; invalid XML scalar values become U+FFFD; export refuses to clobber a path |
| Configuration | unknown JSON fields and malformed files are rejected; enumerated settings are clamped/allowlisted |
| Bundled data | fixed AppDir paths; no first-run fetch; environment overrides are explicit administrative/test inputs |
| Privilege | the GUI refuses effective UID 0 and never requests elevation |
| Packaging | graphics driver stacks, Vulkan/GL/EGL/GLX loaders, glibc, and network GIO modules are intentionally not bundled |
| Logs | no document content is logged; normal dialogs use categorized errors rather than raw paths |

## Residual risks and incomplete controls

- Rime and libkkc are native C/C++ libraries. Their FFI boundaries are small and
  ownership is documented, but this build did not complete ASan or Valgrind.
- Rime/libkkc whole-text conversion currently runs synchronously on GTK's main
  thread. Neural Indic conversion runs in a background worker, but very large
  Rime/libkkc input could still briefly pause the UI.
- Open and first save use native dialogs, but New/Open do not yet prompt for all
  dirty-document transitions. Recovery limits accidental loss but is not a
  substitute for a confirmation workflow.
- User-learning controls, learned-data clearing, PDF/print, recent files, and
  cross-distribution qualification are not implemented. See
  `KNOWN-LIMITATIONS.md`.
- `cargo audit` and `cargo deny` were unavailable in the controlled package
  snapshot. Clippy, rustfmt, lock enforcement, auditable embedding, SPDX
  generation, native notice capture, and socket tracing ran; absence of the two
  requested scanners remains a release exception, not a passing result.

No claim in this document upgrades an unexecuted test into a pass. Exact results
are in `TEST-REPORT.md`.

## Addendum for 1.2.0 (2026-11-19)

1.2.0 adds persistent state, file import and a richer interface. These are the changes
that matter for security and privacy.

| Area | 1.2.0 behaviour |
|---|---|
| Network | Unchanged: no listener, no telemetry, no updater, no downloads. `scripts/check_no_network.sh` runs the full self-test under the seccomp socket trap and passes on 1.2.0. |
| New stored data | `session.json` (both panes, language, file path) and `learned.json` (word → chosen spelling). Both are written atomically, mode `0600`, in `0700` XDG directories. Learning can be switched off, and *Forget learned words* clears it. The learned store is capped at 50,000 entries. |
| Restored state | A damaged settings, session or learned file falls back to defaults; values are clamped on load; a session pointing at a missing file restores the text without the path. |
| File input | Open and drag-and-drop accept regular, non-symlink UTF-8 files up to 16 MiB. The Roman pane is capped at 1 MB. Files passed on the command line follow the same rules. |
| Exports | Word export escapes XML, writes through a temporary file and refuses to write through a symlink. Replacing an existing file needs the save dialog's confirmation. |
| AI worker | Requests are batches of unique words: at most 512 per request from the app and 4,000 accepted by the worker, 48 characters per word, a 20,000-entry cache, and decoding bounded by word length. The worker's standard error is discarded, so document text never reaches a log. |
| Desktop integration | Single instance through GApplication on the session D-Bus (`org.universaltransliterator.UniversalTransliterator`). The theme follows the desktop through a read-only call to the XDG settings portal. `--new-window` opts out of single-instance mode. |
| Privilege | The editor refuses to run as root. The command-line mode reads only its arguments or standard input and writes to standard output. |
| Packaging | The AppImage no longer carries a second copy of ONNX Runtime or the Adwaita cursor set. It bundles Ubuntu's Node.js 18 rather than any newer `node` found on `PATH`, so every bundled binary has package provenance and a licence file. |

Residual risks for 1.2.0:

- Rime and libkkc conversions still run on the GTK main thread. They are cached per
  segment, and a 400-syllable Pinyin document converts in about 0.5 s on the test
  machine, but very large Chinese or Japanese pastes can pause typing briefly.
- No ASan or Valgrind run was done on the Rime/libkkc FFI paths, and `cargo audit` /
  `cargo deny` were not available offline. The crate sources are Ubuntu's packaged
  versions, pinned by `Cargo.lock`.
- `learned.json` and `session.json` are plain text readable by your user account, so
  anyone with access to your account can read your recent document from the session
  file. If that matters, turn off *Restore my work on start* in Preferences: the session
  file is then deleted and no longer written.
