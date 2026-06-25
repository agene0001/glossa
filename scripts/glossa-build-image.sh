#!/usr/bin/env bash
#
# Put the Cargo/Tauri build cache on an APFS disk image on the external drive.
#
# Why: this repo lives on exFAT, which has no extended-attribute support, so
# macOS scatters `._*` AppleDouble sidecars that Tauri's build script chokes on
# (it can't build directly on exFAT). The internal disk is also tight. An APFS
# disk image gives real xattr support *and* lives on the roomy external drive.
#
# This script is idempotent: it creates the image if missing, mounts it if not
# mounted, and (re)points ./target at the mount. Run it once per login session
# (disk images don't auto-mount) before building. Unmount with:
#   hdiutil detach /Volumes/GlossaBuild
set -euo pipefail

IMAGE="/Volumes/Crucial X9/cs_projects/glossa-build.sparseimage"
VOLNAME="GlossaBuild"
MOUNT="/Volumes/$VOLNAME"
SIZE="80g" # sparse: only the space actually used is consumed, up to this max
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [ ! -e "$IMAGE" ]; then
	echo "Creating APFS build image (sparse, ${SIZE} max) at:"
	echo "  $IMAGE"
	hdiutil create -type SPARSE -fs APFS -size "$SIZE" -volname "$VOLNAME" "$IMAGE" >/dev/null
fi

if [ ! -d "$MOUNT" ]; then
	echo "Mounting image → $MOUNT"
	hdiutil attach "$IMAGE" >/dev/null
fi

if [ -L "$REPO/target" ]; then
	rm "$REPO/target"
elif [ -e "$REPO/target" ]; then
	echo "error: $REPO/target exists and is not a symlink — refusing to remove it." >&2
	exit 1
fi
ln -s "$MOUNT" "$REPO/target"

echo "Ready: ./target → $(readlink "$REPO/target")"
echo "Build as usual (e.g. npm run dev / cargo build)."
