# Third-party components and licenses

This is the human-readable inventory. The release's SPDX 2.3 JSON is the exact
machine-readable package/version inventory. The AppImage also contains, under
`usr/share/doc/universal-transliterator/licenses/`, the distribution's complete
`debian/copyright` file for every bundled native package and the licence/notice files
of each Rust crate where the packaged source provides them.

New Universal Transliterator code is GPL-3.0-or-later. It does not relicense the
items below.

## Major native/runtime components

| Component in recorded build | Version | Upstream/source | Declared package terms |
|---|---:|---|---|
| AI4Bharat IndicXlit package/model wrapper | `@cloudrumbles/indic-transliterate` 1.2.2 | cloudrumbles/indic-transliterate; AI4Bharat IndicXlit models | MIT package licence; model provenance described upstream |
| ONNX Runtime Node binding | 1.20.1 | Microsoft ONNX Runtime | MIT |
| Node.js runtime (`nodejs`, with `libnode109`) | 18.19.1 (Ubuntu 24.04) | Node.js | MIT, plus the bundled third-party notices in the package copyright file |
| GTK | 4.14.5 | `https://www.gtk.org/` | LGPL-2.1+ |
| GLib/GIO/GObject | 2.80.0 | GNOME GLib | LGPL-2.1+ (package file controls) |
| librsvg (gdk-pixbuf SVG loader, `librsvg2-common`) | 2.58.0 | GNOME librsvg | LGPL-2+; bundled Rust crates under their own permissive terms (see package file) |
| librime | 1.10.0 | `https://github.com/rime/librime` | Ubuntu package: GPL-3 plus ISC-variant files |
| libkkc | 0.3.5 | `https://github.com/ueno/libkkc` | GPL-3.0+; Debian files Expat |
| libkkc sorted3 data | 0.2.7 | libkkc data repository | GPL-3+; Debian files GPL-2+ |
| SKK-JISYO.L (`skkdic`) | 20230109 | `https://skk-dev.github.io/dict/` | GPL-2+ and file-specific notices |
| Rime Luna Pinyin | git snapshot `79aeae2` package | rime-luna-pinyin | LGPL-3 |
| Rime Pinyin Simplified | git snapshot `52b9c75` package | rime-pinyin-simp | Apache-2.0 |
| Rime Prelude | git snapshot `dd84abe` package | rime-prelude | LGPL-3 |
| Rime Essay | git snapshot `e0519d0` package | rime-essay | LGPL-3 |
| Rime Stroke | git snapshot `65fdbbf` package | rime-stroke | LGPL-3 |
| Noto core fonts (Latin, Indic, Arabic, Nastaliq, Hebrew, Armenian, Georgian, Thai, Ethiopic, Ol Chiki, Meetei Mayek) | 20201225 | Google Noto | OFL-1.1 (plus package-specific exceptions/notices) |
| JetBrains Mono (Regular, Bold, Italic) | 2.304 | JetBrains/JetBrainsMono | OFL-1.1, as embedded in the font files; Debian's copyright file still records the earlier Apache-2.0 terms |
| Noto CJK | 20230817 | notofonts/noto-cjk | OFL-1.1; see packaged notice |
| Adwaita icons (without the cursor set) | Ubuntu 46.0 package | GNOME | CC-BY-SA-3.0 or LGPL-3, and CC-BY-SA-4.0 |
| AppImage type-2 runtime | pinned binary SHA in component manifest | AppImage/type2-runtime | component-specific upstream licenses; embedded notice and separate release notice |
| squashfuse within runtime | runtime-reported 0.5 lineage | vasi/squashfuse | BSD-2-Clause |

The AppImage dynamically bundles further GTK/Rime/KKC dependencies discovered
from ELF metadata. Their exact Debian binary-package names, versions, homepages,
and copyright texts are generated during packaging; they include the applicable
Cairo, Pango, HarfBuzz, Fontconfig, FreeType, image codec, X11/Wayland,
LevelDB, OpenCC, Marisa, yaml-cpp, and compression-library packages. glibc and
host Vulkan/GL/EGL/GLX/DRM/GBM stacks are deliberately excluded.

## Rust package license inventory

Exact versions are in `Cargo.lock` and the SPDX file. The following groups list
every non-workspace Cargo package by its declared expression:

| Declared expression | Packages |
|---|---|
| MIT | cairo-rs, cairo-sys-rs, field-offset, gdk-pixbuf, gdk-pixbuf-sys, gdk4, gdk4-sys, gio, gio-sys, glib, glib-macros, glib-sys, gobject-sys, graphene-rs, graphene-sys, gsk4, gsk4-sys, gtk4, gtk4-macros, gtk4-sys, memoffset, pango, pango-sys, slab, version-compare, winnow, zip |
| MIT OR Apache-2.0 (including reversed/spelling-equivalent forms) | anyhow, autocfg, bitflags, cfg-expr, cfg-if, crc32fast, crossbeam-utils, directories, dirs-sys, errno, fastrand, flate2, all futures packages, hashbrown, heck, indexmap, itoa, libc, pin-project-lite, pin-utils, pkg-config, proc-macro-crate, proc-macro-error, proc-macro-error-attr, proc-macro2, quote, rustc_version, semver, serde, serde_derive, serde_json, serde_spanned, smallvec, syn 1 and 2, system-deps, tempfile, thiserror, thiserror-impl, toml, toml_datetime, toml_edit, version_check |
| 0BSD OR MIT OR Apache-2.0 | adler |
| Unlicense OR MIT | byteorder, memchr |
| MIT OR Zlib OR Apache-2.0 | miniz_oxide |
| Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | linux-raw-sys, rustix |
| Apache-2.0 WITH LLVM-exception | target-lexicon |
| Apache-2.0 OR BSL-1.0 | ryu |
| MPL-2.0 | option-ext |
| (MIT OR Apache-2.0) AND Unicode-DFS-2016 | unicode-ident |
| MIT/Apache-2.0 | unicode-normalization |

The generated license tree is authoritative if a summary spelling differs from
a package file. GPL/LGPL corresponding-source offers and redistribution duties
must be satisfied by anyone redistributing a modified bundle; the upstream URLs
and Debian source package metadata in the notices identify corresponding source.

## Indic worker material

The release bundles the IndicXlit encoder, decoder, vocabulary, ONNX Runtime
binding, and a small private stdin/stdout worker. The worker's beam-search logic
is adapted from the MIT-licensed `@cloudrumbles/indic-transliterate` 1.2.2
package. Its MIT notice and the ONNX Runtime MIT notice are stored under
`resources/indic/licenses` and copied into the AppImage.
