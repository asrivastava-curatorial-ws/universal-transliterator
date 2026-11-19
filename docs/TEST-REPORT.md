# Test report — 1.2.0

Date: 2026-11-19. Build machine: Ubuntu 24.04 x86-64, Ubuntu's Rust 1.75 toolchain and
crate sources, GTK 4.14.5, librime 1.10.0, libkkc 0.3.5, Node.js 18.19.1.

## Automated (run by `build_release.sh` and by CI)

| Check | Result |
|---|---|
| `cargo fmt --check` (rustfmt 1.75; also checked with 1.97) | pass |
| `cargo clippy --workspace --all-targets -D warnings` (1.75; also checked with 1.97) | pass |
| Unit tests: 67 across the workspace (39 language engines and tokenizer, 7 merge/diff, 7 conversion pipeline, others in storage, export, keyboards and engines) | pass |
| `--self-test`: 31 keyboard layouts, ITRANS Hindi, Korean, Telex, Amharic, Ol Chiki, Russian, Arabic, script conversion, romanization, edit-preserving merge, IndicXlit pipeline and alternatives, Rime and its alternatives, libkkc words, sentence and per-segment alternatives, atomic storage, DOCX export | pass, about 1.1 s |
| Self-test under a seccomp filter that traps IPv4/IPv6 socket creation (`scripts/check_no_network.sh`) | pass: no socket opened |
| AppImage: desktop-file and AppStream validation | pass |
| AppImage extracted into a path with spaces: `--version`, `--diagnostics`, `--self-test` | pass |
| SPDX SBOM, source archive, `SHA256SUMS` | generated |

## Interface (Xvfb, AppImage and release build, run as an unprivileged user)

- **Start-up:** the window shows in about 0.3 s. A session that starts in Japanese
  does not start the AI worker; switching to Hindi with
  <kbd>Ctrl</kbd>+<kbd>L</kbd> starts it.
- **Typing and merging:** typed `main kahan hoon? mera naam abhisht` gives
  `मैं कहाँ हूँ? मेरा नाम अभिष्ट`. A `!` added by hand in the output survived further
  typing (` hai`).
- **Alternatives:** <kbd>Alt</kbd>+<kbd>2</kbd> picked the second alternative. The
  choice was written to `learned.json` and used again after a restart.
- **Session:** after quitting and starting again, both panes, the language and the
  picked alternative came back, and the alternatives bar worked at once.
- **Screens checked:** dark and light themes, the language picker, the on-screen
  keyboard, Japanese phrase conversion with per-segment alternatives, right-to-left
  Arabic output. These are the screenshots in `docs/screenshots/`.
- **Fonts:** Devanagari in the output-pane title renders correctly through the
  bundled Noto fonts. Before this fix, the system fallback (FreeSans) mis-shaped it.

## Languages

Every example in `docs/LANGUAGE-SUPPORT.md` and `README.md` was produced by this build's
command-line mode. For Hindi, the model's first choice was compared with the standard
spelling of 261 common chat words. It differed for 84 of them. In 71 cases it was
plainly wrong rather than an accepted variant (for example `aap` → एएपी, `main` → मैन),
and those 71 words now get their standard spelling by default.

## Not covered

- Real GPU renderers (only Cairo on Xvfb was used), a Wayland session, and HiDPI
  scaling.
- Desktop integration on KDE Plasma (theme portal, file manager "Open with").
- Distributions other than Ubuntu 24.04.
- ASan/Valgrind runs on the Rime and libkkc bindings; `cargo audit` / `cargo deny`
  (not available offline).
