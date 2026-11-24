#!/bin/sh
# Checks that the release tag (e.g. v0.1.0), Cargo.toml, the RPM spec and the AppStream
# metainfo name one version (D-149). Run from the repository's root.
set -eu
tag=${1#v}
cargo=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
spec=$(sed -n 's/^Version: *//p' packaging/rpm/rustbot-creator.spec)
metainfo=$(sed -n 's/.*<release version="\([^"]*\)".*/\1/p' \
    packaging/linux/io.github.kotoanidev.RustBotCreator.metainfo.xml | head -n 1)
for found in "$cargo" "$spec" "$metainfo"; do
    if [ "$found" != "$tag" ]; then
        echo "Version mismatch: tag $tag, Cargo.toml $cargo, spec $spec, metainfo $metainfo" >&2
        exit 1
    fi
done
echo "Version $tag"
