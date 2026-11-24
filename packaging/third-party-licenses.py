#!/usr/bin/env python3
"""Writes the license notices of every crate built into the app, for its packages.

The packages ship one program with hundreds of crates inside, and most of their licenses
(MIT, BSD, Apache-2.0) ask for their notices to come with it. Reads `cargo metadata` as
JSON on stdin, after a build, so every crate's source is in cargo's cache, and writes
the notices to stdout. Identical texts are printed once, with the crates that use them.

    cargo metadata --format-version 1 --locked --offline \\
        --filter-platform x86_64-unknown-linux-gnu \\
        | python3 -I packaging/third-party-licenses.py > THIRD-PARTY-LICENSES.txt
"""

import json
import os
import sys

APP = "rustbot-creator"
# The standard texts the app ships (D-136), for crates without a license file.
STANDARD = {
    "MIT": "crates/rustbot-creator/licenses/MIT.txt",
    "Apache-2.0": "crates/rustbot-creator/licenses/Apache-2.0.txt",
}
NAMES = ("license", "licence", "copying", "notice", "unlicense", "copyright")


def linked_crates(metadata):
    """The crates from a registry or git that the app links: no dev or build deps."""
    packages = {package["id"]: package for package in metadata["packages"]}
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    root = next(p["id"] for p in metadata["packages"] if p["name"] == APP)
    seen, stack = set(), [root]
    while stack:
        current = stack.pop()
        if current in seen:
            continue
        seen.add(current)
        for dep in nodes[current]["deps"]:
            if any(kind["kind"] is None for kind in dep["dep_kinds"]):
                stack.append(dep["pkg"])
    crates = [packages[i] for i in seen if packages[i]["source"]]
    return sorted(crates, key=lambda p: (p["name"], p["version"]))


def license_texts(package):
    """The crate's license files, by file name."""
    folder = os.path.dirname(package["manifest_path"])
    texts = []
    for name in sorted(os.listdir(folder)):
        path = os.path.join(folder, name)
        if name.lower().startswith(NAMES) and os.path.isfile(path):
            with open(path, encoding="utf-8", errors="replace") as file:
                texts.append(file.read().strip())
    if package.get("license_file"):
        path = os.path.join(folder, package["license_file"])
        if os.path.isfile(path):
            with open(path, encoding="utf-8", errors="replace") as file:
                text = file.read().strip()
            if text not in texts:
                texts.append(text)
    return texts


def main():
    crates = linked_crates(json.load(sys.stdin))
    by_text = {}
    without = []
    for crate in crates:
        label = f"{crate['name']} {crate['version']}"
        texts = license_texts(crate)
        if not texts:
            without.append(crate)
        for text in texts:
            by_text.setdefault(text, []).append(label)

    out = sys.stdout
    out.write(
        "RustBot Creator includes the following crates. Their licenses, as their authors\n"
        "state them, and the texts that came with them, follow.\n\n"
    )
    for crate in crates:
        source = crate.get("repository") or "https://crates.io/crates/" + crate["name"]
        out.write(f"{crate['name']} {crate['version']}: {crate.get('license')} ({source})\n")
    if without:
        out.write(
            "\nThese crates ship no license file; their license is the one named above:\n"
        )
        for crate in without:
            out.write(f"  {crate['name']} {crate['version']}\n")
    root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
    for spdx, path in STANDARD.items():
        users = [c for c in without if spdx in (c.get("license") or "")]
        if users:
            with open(os.path.join(root, path), encoding="utf-8") as file:
                text = file.read().replace("{holder}", "the crate's authors").strip()
            labels = [f"{c['name']} {c['version']} (the standard {spdx} text)" for c in users]
            by_text.setdefault(text, []).extend(labels)
    for text, labels in sorted(by_text.items(), key=lambda item: item[1][0]):
        out.write("\n" + "=" * 78 + "\n")
        out.write("Used by: " + ", ".join(labels) + "\n\n")
        out.write(text + "\n")


if __name__ == "__main__":
    main()
