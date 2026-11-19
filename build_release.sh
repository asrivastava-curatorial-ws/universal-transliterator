#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
VERSION=${VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -n 1)}
ARCH=x86_64
# Reproducible timestamps: the last commit's time, or the release date.
if [ -z "${SOURCE_DATE_EPOCH:-}" ]; then
    SOURCE_DATE_EPOCH=$(git -C "$ROOT" log -1 --format=%ct 2>/dev/null || echo 1795046400)
fi
export SOURCE_DATE_EPOCH
DIST=$ROOT/dist
BUILD=$ROOT/build
APPIMAGE=$DIST/Universal-Transliterator-$VERSION-$ARCH.AppImage
SOURCE_ARCHIVE=$DIST/Universal-Transliterator-$VERSION-source.tar.zst
SBOM=$DIST/Universal-Transliterator-$VERSION.spdx.json

mkdir -p "$DIST" "$BUILD"
chmod 0755 "$ROOT/build_release.sh" "$ROOT/packaging/appimage/AppRun" \
    "$ROOT/packaging/appimage/build_appimage.sh" \
    "$ROOT/scripts/check_no_network.sh" "$ROOT/scripts/generate_sbom.py"

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked

if cargo --list | grep -q '^    auditable'; then
    cargo auditable build --release --locked -p universal-transliterator
else
    cargo build --release --locked -p universal-transliterator
fi

target/release/universal-transliterator --self-test
bash scripts/check_no_network.sh target/release/universal-transliterator "$BUILD/network.trace"

VERSION=$VERSION ARCH=$ARCH SOURCE_DATE_EPOCH=$SOURCE_DATE_EPOCH \
    bash packaging/appimage/build_appimage.sh

rm -rf -- "$BUILD/appimage smoke" "$BUILD/appimage path with spaces"
mkdir -p "$BUILD/appimage smoke" "$BUILD/appimage path with spaces"
cp "$APPIMAGE" "$BUILD/appimage path with spaces/$(basename "$APPIMAGE")"
(
    cd "$BUILD/appimage smoke"
    "$BUILD/appimage path with spaces/$(basename "$APPIMAGE")" --appimage-extract >/dev/null
    ./squashfs-root/AppRun --version
    ./squashfs-root/AppRun --diagnostics
    ./squashfs-root/AppRun --self-test
)

python3 scripts/generate_sbom.py --root "$ROOT" --native "$BUILD/native-packages.tsv" \
    --output "$SBOM"

rm -f -- "$SOURCE_ARCHIVE" "$DIST/SHA256SUMS"
NAME=$(basename "$ROOT")
tar --sort=name --mtime="@$SOURCE_DATE_EPOCH" --owner=0 --group=0 --numeric-owner \
    --exclude="$NAME/target" \
    --exclude="$NAME/build" \
    --exclude="$NAME/dist" \
    --exclude="$NAME/.git" \
    --exclude='__pycache__' \
    --exclude='*.pyc' \
    --transform="s,^$NAME,universal-transliterator-$VERSION," \
    -C "$(dirname "$ROOT")" -cf - "$NAME" \
    | zstd -q -19 -T1 -o "$SOURCE_ARCHIVE"
touch -d "@$SOURCE_DATE_EPOCH" "$SOURCE_ARCHIVE" "$SBOM"

(
    cd "$DIST"
    sha256sum "$(basename "$APPIMAGE")" "$(basename "$SOURCE_ARCHIVE")" \
        "$(basename "$SBOM")" >SHA256SUMS
)
echo "Release artifacts written to $DIST:"
ls -l "$DIST"
