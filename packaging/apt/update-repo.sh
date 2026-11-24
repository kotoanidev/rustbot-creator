#!/bin/sh
# Adds .deb files to the signed apt repository on GitHub Pages (D-145):
#   update-repo.sh <the gh-pages checkout> <key fingerprint> <package.deb>...
# The key is already in gpg's keyring: the release workflow imports it from a secret.
# How users add the repository is in docs/RELEASING.md.
set -eu
site=$1
key=$2
shift 2
repo="$site/apt"
pool="$repo/pool/main/r/rustbot-creator"
mkdir -p "$pool" "$repo/dists/stable/main/binary-amd64"
cp "$@" "$pool/"

cd "$repo"
# The old signatures go first, so the new Release does not list them.
rm -f dists/stable/Release dists/stable/InRelease dists/stable/Release.gpg
apt-ftparchive packages pool > dists/stable/main/binary-amd64/Packages
gzip -9 -k -f dists/stable/main/binary-amd64/Packages
apt-ftparchive \
    -o APT::FTPArchive::Release::Origin="RustBot Creator" \
    -o APT::FTPArchive::Release::Label="RustBot Creator" \
    -o APT::FTPArchive::Release::Suite=stable \
    -o APT::FTPArchive::Release::Codename=stable \
    -o APT::FTPArchive::Release::Architectures=amd64 \
    -o APT::FTPArchive::Release::Components=main \
    release dists/stable > Release.tmp
mv Release.tmp dists/stable/Release
gpg --batch --yes --local-user "$key" --clearsign -o dists/stable/InRelease dists/stable/Release
gpg --batch --yes --local-user "$key" --armor --detach-sign -o dists/stable/Release.gpg \
    dists/stable/Release
gpg --batch --yes --export "$key" > rustbot-creator.gpg
