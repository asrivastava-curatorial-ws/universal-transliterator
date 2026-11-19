# Component manifest

| Component | Version / use | Licence |
|---|---|---|
| Universal Transliterator | 1.2.0, Rust/GTK 4 application | GPL-3.0-or-later |
| AI4Bharat IndicXlit model via `@cloudrumbles/indic-transliterate` | package 1.2.2; encoder, decoder and vocabulary | MIT package licence; upstream model attribution retained |
| `worker.js` | batched beam-search worker adapted from the package above | GPL-3.0-or-later (new code), MIT notice kept |
| ONNX Runtime Node binding | 1.20.1, Linux x86-64 CPU | MIT |
| Node.js | 18.19.1 from Ubuntu 24.04 (`nodejs`, `libnode109`), runs only the worker | MIT and bundled notices |
| GTK 4 / GLib | 4.14.5 / 2.80.0 from Ubuntu 24.04 | LGPL-2.1+ |
| librime + Rime data | 1.10.0; Luna Pinyin, Pinyin Simplified, Stroke, Prelude, Essay | GPL-3 as packaged by Debian (with ISC/BSD/Expat files); data LGPL-3 / Apache-2.0 |
| libkkc + data, SKK-JISYO.L | 0.3.5, 0.2.7, 20230109 | GPL-3+ / GPL-2+ |
| librsvg SVG loader | 2.58.0, for symbolic icons | LGPL-2+ |
| Fonts | JetBrains Mono 2.304; Noto Sans for every bundled script; Noto Sans CJK | SIL OFL 1.1 |
| AppImage type-2 runtime | `packaging/appimage/runtime-x86_64` | see `APPIMAGE-RUNTIME-NOTICE.txt` |

Exact Rust crate versions are locked in `Cargo.lock`. Exact native package versions,
homepages and copyright files are captured by the AppImage build into the SPDX SBOM
(`Universal-Transliterator-1.2.0.spdx.json`) and under
`usr/share/doc/universal-transliterator/licenses/` inside the AppImage.
