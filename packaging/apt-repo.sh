#!/usr/bin/env bash
# Builds Paintbox's apt repository from .deb files, signed with a GPG key.
# Usage: packaging/apt-repo.sh DEB_DIR OUT_DIR KEY_ID
#
# OUT_DIR gets:
#   pool/main/p/paintbox/*.deb
#   dists/stable/main/binary-amd64/Packages(.gz)
#   dists/stable/Release, Release.gpg, InRelease (signed)
#   paintbox.gpg (the public key, for /etc/apt/keyrings)
# The Apt workflow runs this after each release and publishes OUT_DIR on GitHub Pages.
set -euo pipefail
debs=${1:?Usage: $0 DEB_DIR OUT_DIR KEY_ID}
out=${2:?Usage: $0 DEB_DIR OUT_DIR KEY_ID}
key=${3:?Usage: $0 DEB_DIR OUT_DIR KEY_ID}

mkdir -p "$out/pool/main/p/paintbox" "$out/dists/stable/main/binary-amd64"
cp "$debs"/*.deb "$out/pool/main/p/paintbox/"
cd "$out"
apt-ftparchive packages pool > dists/stable/main/binary-amd64/Packages
gzip -9 -k -f dists/stable/main/binary-amd64/Packages
apt-ftparchive \
  -o APT::FTPArchive::Release::Origin=Paintbox \
  -o APT::FTPArchive::Release::Label=Paintbox \
  -o APT::FTPArchive::Release::Suite=stable \
  -o APT::FTPArchive::Release::Codename=stable \
  -o APT::FTPArchive::Release::Architectures=amd64 \
  -o APT::FTPArchive::Release::Components=main \
  -o APT::FTPArchive::Release::Description="Paintbox, a familiar paint program" \
  release dists/stable > dists/stable/Release.new
mv dists/stable/Release.new dists/stable/Release
rm -f dists/stable/InRelease dists/stable/Release.gpg
gpg --batch --yes --local-user "$key" --clearsign -o dists/stable/InRelease dists/stable/Release
gpg --batch --yes --local-user "$key" --armor --detach-sign -o dists/stable/Release.gpg dists/stable/Release
gpg --batch --yes --export "$key" > paintbox.gpg
echo "apt repository ready in $out"
