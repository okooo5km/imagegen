#!/bin/sh
# Install prebuilt imagegen without Python, Node, Rust, or root privileges.
set -eu
version=${IMAGEGEN_VERSION:-v2.0.1}
case "$version" in v[0-9]*.[0-9]*.[0-9]*) ;; *) echo 'Invalid IMAGEGEN_VERSION' >&2; exit 1 ;; esac
os=$(uname -s)
arch=$(uname -m)
case "$os:$arch" in
  Darwin:arm64|Darwin:aarch64) target=aarch64-apple-darwin ;;
  Darwin:x86_64) target=x86_64-apple-darwin ;;
  Linux:x86_64|Linux:amd64) target=x86_64-unknown-linux-musl ;;
  Linux:aarch64|Linux:arm64) target=aarch64-unknown-linux-musl ;;
  *) echo "Unsupported platform: $os $arch. See GitHub releases." >&2; exit 1 ;;
esac
archive="imagegen-$target.tar.gz"
base="https://github.com/okooo5km/imagegen/releases/download/$version"
bin=${IMAGEGEN_INSTALL_DIR:-"$HOME/.local/bin"}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT HUP INT TERM
if [ -n "${IMAGEGEN_ARCHIVE_DIR:-}" ]; then
  cp "$IMAGEGEN_ARCHIVE_DIR/$archive" "$work/$archive"
  cp "$IMAGEGEN_ARCHIVE_DIR/$archive.sha256" "$work/$archive.sha256"
else
  command -v curl >/dev/null 2>&1 || { echo 'curl is required to download the release; use a manual archive if unavailable.' >&2; exit 1; }
  curl --proto '=https' --tlsv1.2 -fLsS "$base/$archive" -o "$work/$archive"
  curl --proto '=https' --tlsv1.2 -fLsS "$base/$archive.sha256" -o "$work/$archive.sha256"
fi
expected=$(awk 'NR==1 {print $1}' "$work/$archive.sha256")
if command -v sha256sum >/dev/null 2>&1; then
  actual=$(sha256sum "$work/$archive" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
  actual=$(shasum -a 256 "$work/$archive" | awk '{print $1}')
else
  echo 'A SHA-256 utility (sha256sum or shasum) is required.' >&2; exit 1
fi
[ "$expected" = "$actual" ] || { echo 'Checksum mismatch; nothing installed.' >&2; exit 1; }
tar -xzf "$work/$archive" -C "$work"
[ -f "$work/imagegen" ] || { echo 'Archive contains no imagegen binary.' >&2; exit 1; }
mkdir -p "$bin"
# Copy before replacing so failed downloads never damage an existing installation.
stage="$bin/.imagegen-new-$$"
cp "$work/imagegen" "$stage"
chmod 755 "$stage"
mv -f "$stage" "$bin/imagegen"
"$bin/imagegen" --version
printf 'Installed: %s/imagegen\n' "$bin"
case ":$PATH:" in *":$bin:"*) ;; *)
  printf 'Add the installation directory to PATH. For the default location run:\n  export PATH="$HOME/.local/bin:$PATH"\n'
  printf 'Add that export to your shell profile to keep it in new terminals.\n'
  ;;
esac
