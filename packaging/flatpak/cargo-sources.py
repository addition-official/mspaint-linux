#!/usr/bin/env python3
"""Write cargo-sources.json, the list of crates flatpak-builder downloads so Paintbox
builds offline (Flathub builds have no network). Same output as flatpak-builder-tools'
flatpak-cargo-generator for crates.io dependencies. Run after Cargo.lock changes:

    python3 packaging/flatpak/cargo-sources.py
"""
import json
import pathlib
import tomllib

root = pathlib.Path(__file__).resolve().parents[2]
lock = tomllib.loads((root / "Cargo.lock").read_text())
sources = []
for pkg in lock["package"]:
    src = pkg.get("source")
    if src is None:
        continue  # Paintbox itself
    if src != "registry+https://github.com/rust-lang/crates.io-index":
        raise SystemExit(f"{pkg['name']}: only crates.io dependencies are supported, not {src}")
    name, version, checksum = pkg["name"], pkg["version"], pkg["checksum"]
    dest = f"cargo/vendor/{name}-{version}"
    sources.append({
        "type": "archive",
        "archive-type": "tar-gzip",
        "url": f"https://static.crates.io/crates/{name}/{name}-{version}.crate",
        "sha256": checksum,
        "dest": dest,
    })
    sources.append({
        "type": "inline",
        "contents": json.dumps({"package": checksum, "files": {}}),
        "dest": dest,
        "dest-filename": ".cargo-checksum.json",
    })
sources.append({
    "type": "inline",
    "contents": '[source.vendored-sources]\ndirectory = "cargo/vendor"\n\n[source.crates-io]\nreplace-with = "vendored-sources"\n',
    "dest": "cargo",
    "dest-filename": "config",
})
out = root / "packaging/flatpak/cargo-sources.json"
out.write_text(json.dumps(sources, indent=4) + "\n")
print(f"{out.relative_to(root)}: {(len(sources) - 1) // 2} crates")
