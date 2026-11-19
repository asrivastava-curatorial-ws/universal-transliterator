#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)
VERSION=${VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -n 1)}
ARCH=${ARCH:-x86_64}
SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-1795046400}
BUILD_ROOT=${BUILD_ROOT:-$ROOT/build}
DIST_ROOT=${DIST_ROOT:-$ROOT/dist}
APPDIR=$BUILD_ROOT/AppDir
APPIMAGE=$DIST_ROOT/Universal-Transliterator-$VERSION-$ARCH.AppImage
LIBDIR=$APPDIR/usr/lib/x86_64-linux-gnu
NATIVE_PACKAGES=$BUILD_ROOT/native-packages.tsv
PROVENANCE=$BUILD_ROOT/bundled-files.tsv

case "$APPDIR" in
    "$ROOT"/build/AppDir | "$BUILD_ROOT"/AppDir) ;;
    *) echo "refusing unsafe AppDir path: $APPDIR" >&2; exit 2 ;;
esac

for command in cargo ldd mksquashfs patchelf dpkg-query sha256sum node; do
    command -v "$command" >/dev/null 2>&1 || {
        echo "missing packaging command: $command" >&2
        exit 2
    }
done

binary=$ROOT/target/release/universal-transliterator
if [ ! -x "$binary" ]; then
    echo "release binary is missing; run cargo auditable build --release --locked" >&2
    exit 2
fi

rm -rf -- "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$LIBDIR" "$APPDIR/usr/share/applications" \
    "$APPDIR/usr/share/metainfo" "$APPDIR/usr/share/icons/hicolor/scalable/apps" \
    "$APPDIR/usr/share/doc/universal-transliterator" "$APPDIR/etc/fonts" \
    "$APPDIR/usr/libexec/universal-transliterator" \
    "$DIST_ROOT"
: >"$PROVENANCE"

install -m 0755 "$binary" "$APPDIR/usr/bin/universal-transliterator"
# Ubuntu's Node.js package, so the bundled runtime has package provenance and
# its licence is captured (a newer node earlier on PATH is not used).
if [ -n "${NODE_BINARY:-}" ]; then
    node_binary=$NODE_BINARY
elif [ -x /usr/bin/node ]; then
    node_binary=/usr/bin/node
else
    node_binary=$(command -v node)
fi
install -m 0755 "$node_binary" "$APPDIR/usr/libexec/universal-transliterator/node"
install -m 0755 "$ROOT/packaging/appimage/AppRun" "$APPDIR/AppRun"
install -m 0644 "$ROOT/packaging/appimage/org.universaltransliterator.UniversalTransliterator.desktop" \
    "$APPDIR/org.universaltransliterator.UniversalTransliterator.desktop"
install -m 0644 "$ROOT/packaging/appimage/org.universaltransliterator.UniversalTransliterator.desktop" \
    "$APPDIR/usr/share/applications/org.universaltransliterator.UniversalTransliterator.desktop"
install -m 0644 "$ROOT/packaging/appimage/org.universaltransliterator.UniversalTransliterator.metainfo.xml" \
    "$APPDIR/usr/share/metainfo/org.universaltransliterator.UniversalTransliterator.metainfo.xml"
install -m 0644 "$ROOT/packaging/appimage/org.universaltransliterator.UniversalTransliterator.svg" \
    "$APPDIR/org.universaltransliterator.UniversalTransliterator.svg"
install -m 0644 "$ROOT/packaging/appimage/org.universaltransliterator.UniversalTransliterator.svg" \
    "$APPDIR/usr/share/icons/hicolor/scalable/apps/org.universaltransliterator.UniversalTransliterator.svg"
ln -s org.universaltransliterator.UniversalTransliterator.svg "$APPDIR/.DirIcon"
install -m 0644 "$ROOT/packaging/appimage/fonts.conf" "$APPDIR/etc/fonts/fonts.conf"
install -m 0644 "$ROOT/LICENSE" "$APPDIR/usr/share/doc/universal-transliterator/COPYING"
install -m 0644 "$ROOT/packaging/appimage/APPIMAGE-RUNTIME-NOTICE.txt" \
    "$APPDIR/usr/share/doc/universal-transliterator/APPIMAGE-RUNTIME-NOTICE.txt"
install -m 0644 "$ROOT/docs/THIRD-PARTY-LICENSES.md" \
    "$APPDIR/usr/share/doc/universal-transliterator/THIRD-PARTY-LICENSES.md"

copy_tree() {
    local source=$1 destination=$2
    if [ ! -d "$source" ]; then
        echo "required data directory is missing: $source" >&2
        exit 2
    fi
    mkdir -p "$destination"
    cp -a "$source"/. "$destination"/
}

copy_tree /usr/share/rime-data "$APPDIR/usr/share/rime-data"
copy_tree /usr/share/libkkc/rules/default "$APPDIR/usr/share/libkkc/rules/default"
copy_tree /usr/lib/x86_64-linux-gnu/libkkc/models/sorted3 "$LIBDIR/libkkc/models/sorted3"
mkdir -p "$APPDIR/usr/share/skk"
install -m 0644 /usr/share/skk/SKK-JISYO.L "$APPDIR/usr/share/skk/SKK-JISYO.L"
copy_tree "$ROOT/resources/indic" "$APPDIR/usr/share/universal-transliterator/indic"

copy_tree /usr/share/glib-2.0/schemas "$APPDIR/usr/share/glib-2.0/schemas"
copy_tree /usr/share/icons/Adwaita "$APPDIR/usr/share/icons/Adwaita"
# Cursors come from the desktop's own theme (XCURSOR_PATH never points into
# the AppImage), so the 11 MB Adwaita cursor set would be dead weight.
rm -rf -- "$APPDIR/usr/share/icons/Adwaita/cursors"
mkdir -p "$APPDIR/usr/share/icons/hicolor"
for icon_file in /usr/share/icons/hicolor/index.theme /usr/share/icons/hicolor/icon-theme.cache; do
    [ ! -f "$icon_file" ] || install -m 0644 "$icon_file" "$APPDIR/usr/share/icons/hicolor/$(basename "$icon_file")"
done

font_destination=$APPDIR/usr/share/fonts
mkdir -p "$font_destination/truetype/noto" "$font_destination/opentype/noto"
fonts=(
    NotoSans-Regular.ttf
    NotoSans-Bold.ttf
    NotoSansMono-Regular.ttf
    NotoSansDevanagari-Regular.ttf
    NotoSansBengali-Regular.ttf
    NotoSansGujarati-Regular.ttf
    NotoSansGurmukhi-Regular.ttf
    NotoSansOriya-Regular.ttf
    NotoSansTamil-Regular.ttf
    NotoSansTelugu-Regular.ttf
    NotoSansKannada-Regular.ttf
    NotoSansMalayalam-Regular.ttf
    NotoSansSinhala-Regular.ttf
    NotoSansArabic-Regular.ttf
    NotoNaskhArabic-Regular.ttf
    NotoNastaliqUrdu-Regular.ttf
    NotoSansHebrew-Regular.ttf
    NotoSansArmenian-Regular.ttf
    NotoSansGeorgian-Regular.ttf
    NotoSansThai-Regular.ttf
    NotoSansEthiopic-Regular.ttf
    NotoSansOlChiki-Regular.ttf
    NotoSansMeeteiMayek-Regular.ttf
)
for font in "${fonts[@]}"; do
    install -m 0644 "/usr/share/fonts/truetype/noto/$font" "$font_destination/truetype/noto/$font"
done
# JetBrains Mono (OFL-1.1) is the interface and Roman-pane font.
mkdir -p "$font_destination/truetype/jetbrains-mono"
for font in JetBrainsMono-Regular.ttf JetBrainsMono-Bold.ttf JetBrainsMono-Italic.ttf; do
    install -m 0644 "/usr/share/fonts/truetype/jetbrains-mono/$font" \
        "$font_destination/truetype/jetbrains-mono/$font"
done
install -m 0644 /usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc \
    "$font_destination/opentype/noto/NotoSansCJK-Regular.ttc"

loader_source=/usr/lib/x86_64-linux-gnu/gdk-pixbuf-2.0/2.10.0/loaders
loader_destination=$LIBDIR/gdk-pixbuf-2.0/2.10.0/loaders
mkdir -p "$loader_destination" "$LIBDIR/gdk-pixbuf-2.0" "$LIBDIR/gio/modules"
for plugin in "$loader_source"/*.so; do
    install -m 0755 "$plugin" "$loader_destination/$(basename "$plugin")"
done
install -m 0755 /usr/lib/x86_64-linux-gnu/gdk-pixbuf-2.0/gdk-pixbuf-query-loaders \
    "$LIBDIR/gdk-pixbuf-2.0/gdk-pixbuf-query-loaders"
install -m 0755 /usr/lib/x86_64-linux-gnu/gio/modules/libdconfsettings.so \
    "$LIBDIR/gio/modules/libdconfsettings.so"
printf '%s\n' 'libdconfsettings.so: gsettings-backend' >"$LIBDIR/gio/modules/giomodule.cache"

queue=$BUILD_ROOT/elf-queue.txt
seen=$BUILD_ROOT/elf-seen.txt
: >"$queue"
: >"$seen"
{
    printf '%s\n' "$APPDIR/usr/bin/universal-transliterator" \
        "$APPDIR/usr/libexec/universal-transliterator/node" \
        "$LIBDIR/gdk-pixbuf-2.0/gdk-pixbuf-query-loaders" \
        "$LIBDIR/gio/modules/libdconfsettings.so"
    find "$loader_destination" -type f -name '*.so' -print
    find "$APPDIR/usr/share/universal-transliterator/indic" -type f \
        \( -name '*.so' -o -name '*.so.*' -o -name '*.node' \) -print
} >>"$queue"

excluded_library() {
    case "$1" in
        ld-linux-*.so.*|libc.so.*|libm.so.*|libdl.so.*|libpthread.so.*|librt.so.*|libresolv.so.*|libutil.so.*|libnss_*.so.*) return 0 ;;
        libGL.so.*|libEGL.so.*|libGLX.so.*|libGLdispatch.so.*|libOpenGL.so.*|libvulkan.so.*|libdrm.so.*|libgbm.so.*) return 0 ;;
        *) return 1 ;;
    esac
}

index=1
while :; do
    elf=$(sed -n "${index}p" "$queue")
    [ -n "$elf" ] || break
    index=$((index + 1))
    if grep -Fqx "$elf" "$seen"; then
        continue
    fi
    printf '%s\n' "$elf" >>"$seen"
    while IFS= read -r dependency; do
        [ -n "$dependency" ] || continue
        base=$(basename "$dependency")
        if excluded_library "$base"; then
            continue
        fi
        # Already bundled where its user expects it (ONNX Runtime is found
        # next to its Node binding through RUNPATH $ORIGIN); a second copy
        # in the library directory would only add 19 MB.
        case "$dependency" in
            "$APPDIR"/*) continue ;;
        esac
        # Prefer the distribution's copy over anything under /usr/local so
        # every bundled library has package provenance and a licence file.
        case "$dependency" in
            /usr/local/*)
                for system_copy in "/lib/x86_64-linux-gnu/$base" "/usr/lib/x86_64-linux-gnu/$base"; do
                    if [ -e "$system_copy" ]; then
                        dependency=$system_copy
                        break
                    fi
                done
                ;;
        esac
        destination=$LIBDIR/$base
        if [ ! -e "$destination" ]; then
            cp -L --preserve=mode "$dependency" "$destination"
            printf '%s\t%s\n' "$dependency" "${destination#"$APPDIR"/}" >>"$PROVENANCE"
            printf '%s\n' "$destination" >>"$queue"
        fi
    done < <(ldd "$elf" 2>/dev/null | awk '/=> \// {print $3} /^\// {print $1}')
done

# The executable is relocatable even when invoked outside AppRun (useful for
# extraction diagnostics). Plugin dependencies are resolved through AppRun's
# LD_LIBRARY_PATH because their nesting depths differ.
patchelf --set-rpath "\$ORIGIN/../lib/x86_64-linux-gnu" \
    "$APPDIR/usr/bin/universal-transliterator"

while IFS= read -r -d '' elf; do
    if file "$elf" | grep -q 'ELF '; then
        if ldd "$elf" 2>&1 | grep -q 'not found'; then
            echo "unresolved bundled ELF dependency: $elf" >&2
            ldd "$elf" >&2
            exit 1
        fi
    fi
done < <(find "$APPDIR/usr/bin" "$LIBDIR" -type f -print0)

packages=$BUILD_ROOT/native-package-names.txt
{
    printf '%s\n' \
        adwaita-icon-theme fonts-jetbrains-mono fonts-noto-cjk fonts-noto-core fonts-noto-mono \
        librsvg2-common \
        libglib2.0-0t64 libglib2.0-data libgtk-4-1 libgtk-4-common \
        libkkc2 libkkc-common libkkc-data librime1t64 librime-data \
        nodejs \
        rime-data-luna-pinyin rime-data-pinyin-simp rime-data-stroke \
        rime-essay rime-prelude skkdic
    while IFS=$'\t' read -r source _destination; do
        dpkg-query -S "$source" 2>/dev/null | sed -n '1s/: \/.*//p' || true
    done <"$PROVENANCE"
} | sed 's/:amd64$//' | LC_ALL=C sort -u >"$packages"

: >"$NATIVE_PACKAGES"
license_dir=$APPDIR/usr/share/doc/universal-transliterator/licenses/debian
mkdir -p "$license_dir"
while IFS= read -r package; do
    [ -n "$package" ] || continue
    if dpkg-query -W -f='${binary:Package}\t${Version}\t${Homepage}\n' "$package" >>"$NATIVE_PACKAGES" 2>/dev/null; then
        copyright=/usr/share/doc/$package/copyright
        if [ -f "$copyright" ]; then
            cp -L "$copyright" "$license_dir/$package.copyright"
        fi
    fi
done <"$packages"
LC_ALL=C sort -u -o "$NATIVE_PACKAGES" "$NATIVE_PACKAGES"

python3 "$ROOT/scripts/generate_sbom.py" --root "$ROOT" --native "$NATIVE_PACKAGES" \
    --license-dir "$APPDIR/usr/share/doc/universal-transliterator/licenses/rust" \
    --output "$APPDIR/usr/share/doc/universal-transliterator/Universal-Transliterator.spdx.json"

desktop-file-validate "$APPDIR/usr/share/applications/org.universaltransliterator.UniversalTransliterator.desktop"
appstreamcli validate --no-net --override=url-homepage-missing=info \
    "$APPDIR/usr/share/metainfo/org.universaltransliterator.UniversalTransliterator.metainfo.xml"

find "$APPDIR" -exec touch -h -d "@$SOURCE_DATE_EPOCH" {} +
sort_file=$BUILD_ROOT/squashfs.sort
find "$APPDIR" -mindepth 1 -printf '%P 0\n' | LC_ALL=C sort >"$sort_file"
squashfs=$BUILD_ROOT/Universal-Transliterator.squashfs
rm -f -- "$squashfs" "$APPIMAGE"
env -u SOURCE_DATE_EPOCH mksquashfs "$APPDIR" "$squashfs" -noappend -comp xz -b 1M -all-root \
    -no-xattrs -mkfs-time "$SOURCE_DATE_EPOCH" -all-time "$SOURCE_DATE_EPOCH" \
    -sort "$sort_file" -processors "${MKSQUASHFS_PROCESSORS:-1}" >/dev/null
cp "$ROOT/packaging/appimage/runtime-x86_64" "$APPIMAGE"
dd if="$squashfs" of="$APPIMAGE" oflag=append conv=notrunc status=none
chmod 0755 "$APPIMAGE"
touch -d "@$SOURCE_DATE_EPOCH" "$APPIMAGE"

sha256sum "$APPIMAGE"
printf 'AppImage size: %s bytes\n' "$(stat -c %s "$APPIMAGE")"
