#!/bin/sh
set -eu
: "${IMAGEGEN_TARGET:?}"
mkdir -p dist package
cp "target/$IMAGEGEN_TARGET/release/imagegen" package/imagegen
cp LICENSE NOTICE package/
if [ "$(uname -s)" = Darwin ]; then
  codesign --force --sign - package/imagegen
  codesign --verify package/imagegen
fi
archive="imagegen-$IMAGEGEN_TARGET.tar.gz"
tar -czf "dist/$archive" -C package imagegen LICENSE NOTICE
if command -v sha256sum >/dev/null 2>&1; then
  (cd dist && sha256sum "$archive" > "$archive.sha256")
else
  (cd dist && shasum -a 256 "$archive" > "$archive.sha256")
fi
export IMAGEGEN_ARCHIVE_DIR="$PWD/dist"
export IMAGEGEN_INSTALL_DIR="${RUNNER_TEMP:-/tmp}/imagegen installed $IMAGEGEN_TARGET"
export IMAGEGEN_CONFIG_DIR="${RUNNER_TEMP:-/tmp}/imagegen config $IMAGEGEN_TARGET"
unset APIMART_API_KEY APIMART_IMAGE_MODEL
sh installers/install.sh
"$IMAGEGEN_INSTALL_DIR/imagegen" doctor
"$IMAGEGEN_INSTALL_DIR/imagegen" config set aspect_ratio 16:9
"$IMAGEGEN_INSTALL_DIR/imagegen" generate --prompt 'Portable installation smoke' --dry-run
# Corrupt an archive: installer must reject it and leave the installed binary intact.
cp "dist/$archive" "dist/$archive.good"
printf 'tamper' >> "dist/$archive"
if sh installers/install.sh; then echo 'Corrupt archive was accepted' >&2; exit 1; fi
mv "dist/$archive.good" "dist/$archive"
"$IMAGEGEN_INSTALL_DIR/imagegen" --version
