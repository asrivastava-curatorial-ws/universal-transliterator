# Building Universal Transliterator

The tested build machine is **Ubuntu or Kubuntu 24.04, x86-64**. The AppImage it
produces needs glibc 2.39 or newer.

The build is offline and locked. Rust crates come from Ubuntu's packaged crate sources
in `/usr/share/cargo/registry` (see `.cargo/config.toml`), and `Cargo.lock` pins every
version. The IndicXlit models and the ONNX Runtime binding are already in
`resources/indic`, so there is no `npm install` and nothing is downloaded during the
build.

## 1. Install the build packages

```bash
sudo apt update
sudo xargs -a packaging/ubuntu-build-deps.txt apt install -y --no-install-recommends
```

## 2. Use Ubuntu's Rust toolchain

The project is built with Ubuntu's `rustc`/`cargo` 1.75 and the crates packaged for
it. If you also have **rustup**, Cargo would pick rustup's `cargo-fmt` and
`cargo-clippy` from `~/.cargo/bin` first. Point Cargo at an empty home and put
`/usr/bin` first for the build shell:

```bash
export CARGO_HOME="$HOME/.cache/ut-cargo-home" PATH="/usr/bin:$PATH"
cargo --version        # cargo 1.75.x (Ubuntu)
```

Without rustup you can skip this step.

## 3. Build, test and package

```bash
./build_release.sh
```

The script runs, in order:

1. `cargo fmt --check` and `cargo clippy -D warnings`;
2. all workspace tests;
3. an optimised build;
4. `--self-test`, which exercises every engine offline;
5. a seccomp check that no network socket is opened;
6. AppImage assembly, with the libraries, data, fonts and licence files it needs;
7. extraction of the AppImage into a path with spaces, and the self-test run again
   from there;
8. the SPDX SBOM, the source archive and `SHA256SUMS`.

Output in `dist/`:

```text
Universal-Transliterator-1.2.0-x86_64.AppImage
Universal-Transliterator-1.2.0-source.tar.zst
Universal-Transliterator-1.2.0.spdx.json
SHA256SUMS
```

`mksquashfs` uses one thread by default so that output is repeatable. For a faster
personal build:

```bash
MKSQUASHFS_PROCESSORS="$(nproc)" ./build_release.sh
```

Timestamps come from `SOURCE_DATE_EPOCH`, which is the last commit time when building
from a Git checkout.

## Development

```bash
cargo run --release -p universal-transliterator              # the editor
cargo run --release -p universal-transliterator -- --self-test
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

A development build finds its data on its own: the AI model in `resources/indic`, and
Rime, libkkc and the system `node` from the packages above. Set these variables to point
it at other locations:

| Variable | Default |
|---|---|
| `UT_INDIC_ROOT` | `resources/indic` (or `../share/universal-transliterator/indic` next to the binary) |
| `UT_NODE_BINARY` | `node` on `PATH` |
| `UT_RIME_DATA_DIR` | `/usr/share/rime-data` |
| `UT_KKC_DICTIONARY` | `/usr/share/skk/SKK-JISYO.L` |
| `UT_KKC_MODEL_METADATA`, `UT_KKC_RULE_METADATA` | the libkkc package paths |

Run `universal-transliterator --diagnostics` to print the paths that are actually in use.

### Layout

| Path | What it is |
|---|---|
| `crates/app` | GTK 4 application: window, pipeline, dialogs, keyboard, self-test, CLI |
| `crates/transliteration` | language registry and rule engines (ITRANS/IAST, Korean, Telex, Ge'ez, Ol Chiki, tables), tokenizer |
| `crates/text` | diff, three-way merge and minimal buffer edits |
| `crates/rime-sys`, `crates/kkc-sys` | small FFI bindings to librime and libkkc |
| `crates/rime-engine`, `crates/japanese-engine`, `crates/core` | safe composition engines over those bindings, and their shared types |
| `crates/keyboards` | on-screen keyboard layouts (`data/layouts.json`) |
| `crates/storage`, `crates/export` | settings, session, learned choices, atomic saves; DOCX writer |
| `resources/indic` | IndicXlit models, ONNX Runtime binding and the `worker.js` process |
| `packaging/appimage` | AppRun, desktop file, AppStream metadata, icon, fontconfig, AppImage builder |
| `scripts` | network check, SBOM generator |

## Other distributions

To build against crates.io instead of Ubuntu's crate packages, move
`.cargo/config.toml` out of the way; `Cargo.lock` still selects the same versions. This
has not been tested. GTK 4.14+, librime, libkkc with its data, an SKK dictionary and
Node.js 18+ are needed at build time and for a development run.
