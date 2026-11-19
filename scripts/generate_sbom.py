#!/usr/bin/env python3
"""Generate a deterministic SPDX 2.3 JSON inventory from Cargo metadata.

Native Debian/Ubuntu packages may be supplied as a tab-separated file with
package, version, and homepage columns. This script performs no network I/O.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
from typing import Any


def spdx_id(prefix: str, value: str) -> str:
    cleaned = re.sub(r"[^A-Za-z0-9.-]+", "-", value).strip("-")
    return f"SPDXRef-{prefix}-{cleaned}"


def cargo_metadata(root: pathlib.Path) -> dict[str, Any]:
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1"],
        cwd=root,
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    )
    return json.loads(result.stdout)


def cargo_package(package: dict[str, Any]) -> dict[str, Any]:
    name = package["name"]
    version = package["version"]
    item: dict[str, Any] = {
        "SPDXID": spdx_id("Cargo", f"{name}-{version}"),
        "name": name,
        "versionInfo": version,
        "downloadLocation": "NOASSERTION",
        "filesAnalyzed": False,
        "licenseConcluded": "NOASSERTION",
        "licenseDeclared": package.get("license") or "NOASSERTION",
        "copyrightText": "NOASSERTION",
        "externalRefs": [
            {
                "referenceCategory": "PACKAGE-MANAGER",
                "referenceType": "purl",
                "referenceLocator": f"pkg:cargo/{name}@{version}",
            }
        ],
    }
    source = package.get("source")
    if source:
        item["sourceInfo"] = f"Cargo source identifier: {source}"
    return item


def copy_cargo_licenses(
    packages: list[dict[str, Any]], destination: pathlib.Path
) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    for package in packages:
        if not package.get("source"):
            continue
        source_dir = pathlib.Path(package["manifest_path"]).parent
        package_dir = destination / f'{package["name"]}-{package["version"]}'
        notices = sorted(
            path
            for path in source_dir.iterdir()
            if path.is_file()
            and re.match(r"^(LICENSE|LICENCE|COPYING|COPYRIGHT|NOTICE)", path.name, re.I)
        )
        if not notices:
            continue
        package_dir.mkdir(parents=True, exist_ok=True)
        for notice in notices:
            shutil.copyfile(notice, package_dir / notice.name)


def native_packages(path: pathlib.Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    packages: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line or line.startswith("#"):
            continue
        columns = line.split("\t")
        name, version = columns[:2]
        homepage = columns[2] if len(columns) > 2 else ""
        item: dict[str, Any] = {
            "SPDXID": spdx_id("Debian", f"{name}-{version}"),
            "name": name,
            "versionInfo": version,
            "downloadLocation": "NOASSERTION",
            "filesAnalyzed": False,
            "licenseConcluded": "NOASSERTION",
            "licenseDeclared": "NOASSERTION",
            "copyrightText": "NOASSERTION",
            "externalRefs": [
                {
                    "referenceCategory": "PACKAGE-MANAGER",
                    "referenceType": "purl",
                    "referenceLocator": f"pkg:deb/ubuntu/{name}@{version}",
                }
            ],
        }
        if homepage:
            item["homepage"] = homepage
        packages.append(item)
    return packages


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=pathlib.Path, default=pathlib.Path.cwd())
    parser.add_argument("--native", type=pathlib.Path)
    parser.add_argument("--license-dir", type=pathlib.Path)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    root = args.root.resolve()
    metadata = cargo_metadata(root)
    if args.license_dir:
        copy_cargo_licenses(metadata["packages"], args.license_dir)
    packages = [cargo_package(item) for item in metadata["packages"]]
    if args.native:
        packages.extend(native_packages(args.native))
    packages.sort(key=lambda item: (item["name"], item.get("versionInfo", "")))

    lock_hash = hashlib.sha256((root / "Cargo.lock").read_bytes()).hexdigest()
    epoch = int(os.environ.get("SOURCE_DATE_EPOCH", "1792195200"))
    created = dt.datetime.fromtimestamp(epoch, tz=dt.timezone.utc).strftime(
        "%Y-%m-%dT%H:%M:%SZ"
    )
    namespace = (
        "https://universal-transliterator.invalid/spdx/1.0.0/" + lock_hash
    )
    document: dict[str, Any] = {
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": "Universal-Transliterator-1.0.0",
        "documentNamespace": namespace,
        "creationInfo": {
            "created": created,
            "creators": ["Tool: scripts/generate_sbom.py"],
            "licenseListVersion": "3.24",
        },
        "documentDescribes": [item["SPDXID"] for item in packages],
        "packages": packages,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(document, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
