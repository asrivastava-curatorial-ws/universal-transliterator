# Publishing on GitHub

These steps create `asrivastava-curatorial-ws/universal-transliterator`, push the
source, and publish release v1.2.0 with the AppImage. They use the GitHub CLI.

## 0. One-time setup

```bash
sudo apt install -y git gh
gh auth login                      # GitHub.com → HTTPS → log in with a browser
git config --global user.name  "Abhisht Srivastava"
git config --global user.email "you@example.com"
```

## 1. Create the repository and push

From the extracted source folder (`universal-transliterator-1.2.0/`):

```bash
git init -b main
git add -A
git commit -m "Universal Transliterator 1.2.0"
gh repo create asrivastava-curatorial-ws/universal-transliterator --public \
  --description "Offline Roman-to-Unicode transliteration editor for 42 languages (GTK 4, Rust)" \
  --source . --remote origin --push
gh repo edit asrivastava-curatorial-ws/universal-transliterator \
  --homepage "https://github.com/asrivastava-curatorial-ws/universal-transliterator" \
  --add-topic transliteration,indic,input-method,gtk4,rust,offline,appimage,linux
```

The push starts the **CI** workflow (format, Clippy, tests, self-test, network
check). Follow it with `gh run watch`.

## 2. Publish the release

Choose one of the two options.

### A. Let GitHub build it (recommended)

```bash
git tag -a v1.2.0 -m "Universal Transliterator 1.2.0"
git push origin v1.2.0
gh run watch                       # about 20–30 minutes
```

The **Release AppImage** workflow builds on `ubuntu-24.04`, runs every test, and creates
the release. The release notes come from the 1.2.0 section of `CHANGELOG.md`, and the
assets are the AppImage, source archive, SBOM and `SHA256SUMS`.

### B. Upload the AppImage you already have

Put the downloaded files in the folder first, then:

```bash
awk '/^## 1.2.0/{p=1;next} /^## /&&p{exit} p' CHANGELOG.md > /tmp/notes.md
gh release create v1.2.0 --target main \
  --title "Universal Transliterator 1.2.0" --notes-file /tmp/notes.md \
  Universal-Transliterator-1.2.0-x86_64.AppImage SHA256SUMS
```

If the tag also starts the Release workflow, the workflow replaces these files with
its own build of the same source and adds the source archive and SBOM.

## 3. Check

```bash
gh release view v1.2.0 --web
```

Anyone can then install with:

```bash
wget https://github.com/asrivastava-curatorial-ws/universal-transliterator/releases/download/v1.2.0/Universal-Transliterator-1.2.0-x86_64.AppImage
chmod +x Universal-Transliterator-1.2.0-x86_64.AppImage
./Universal-Transliterator-1.2.0-x86_64.AppImage
```

## Next releases

1. Set the new version in `Cargo.toml` (`[workspace.package] version`); the build
   scripts read it from there. Run `cargo update -w --offline` to refresh `Cargo.lock`.
2. Add a `<release>` entry to
   `packaging/appimage/org.universaltransliterator.UniversalTransliterator.metainfo.xml`,
   a `## x.y.z — date` section at the top of `CHANGELOG.md`, and update the download
   links in `README.md`.
3. Commit, then `git tag -a vX.Y.Z -m "…" && git push origin main vX.Y.Z`.

The workflow refuses to run if the tag does not match the version in `Cargo.toml`.

## If a workflow fails

- Open the failed run with `gh run view --log-failed`.
- The build uses Ubuntu 24.04's packaged Rust crates with `--locked`. If Ubuntu ever
  ships a newer version of one of them, Cargo stops with a lock-file error. Run
  `cargo update --offline` on an up-to-date Ubuntu 24.04 machine (see `BUILDING.md`),
  commit `Cargo.lock`, then move the tag:
  `git tag -fa v1.2.0 -m "Universal Transliterator 1.2.0" && git push -f origin v1.2.0`.
- To build a release again for an existing tag, run the workflow by hand:
  `gh workflow run release.yml -f tag=v1.2.0`.
