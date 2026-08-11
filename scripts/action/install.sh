#!/usr/bin/env bash
# Download, verify and unpack the notiflow release binary.
#
# Inputs (env):
#   VERSION            vX.Y.Z tag
#   TARGET             Rust target triple
#   EXT                tar.gz | zip
#   BINARY             notiflow | notiflow.exe
#   DEST               directory to install into
#   VERIFY_SIGNATURE   true to require a cosign verification (default false)
#
# The sha256 in checksums.txt comes from the same origin as the archive, so on its own it
# only proves the download was not corrupted. The cosign check is what proves the archive
# is the one the release workflow actually built.
set -euo pipefail

: "${VERSION:?VERSION is required}"
: "${TARGET:?TARGET is required}"
: "${EXT:?EXT is required}"
: "${BINARY:?BINARY is required}"
: "${DEST:?DEST is required}"

repo="jtprogru/notiflow"
base="https://github.com/${repo}/releases/download/${VERSION}"
archive="notiflow-${TARGET}.${EXT}"

mkdir -p "$DEST"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "Downloading ${base}/${archive}"
curl -fsSL --retry 3 --retry-delay 2 -o "${tmp}/${archive}" "${base}/${archive}"
curl -fsSL --retry 3 --retry-delay 2 -o "${tmp}/checksums.txt" "${base}/checksums.txt"

# sha256sum is GNU coreutils; macOS ships shasum instead.
if command -v sha256sum >/dev/null 2>&1; then
  (cd "$tmp" && grep -E "[ *]${archive}\$" checksums.txt | sha256sum -c -)
else
  expected="$(grep -E "[ *]${archive}\$" "${tmp}/checksums.txt" | awk '{print $1}')"
  actual="$(shasum -a 256 "${tmp}/${archive}" | awk '{print $1}')"
  if [ -z "$expected" ] || [ "$expected" != "$actual" ]; then
    echo "::error::sha256 mismatch for ${archive}: expected '${expected}', got '${actual}'" >&2
    exit 1
  fi
fi
echo "Checksum verified for ${archive}"

if [ "${VERIFY_SIGNATURE:-false}" = "true" ]; then
  if ! command -v cosign >/dev/null 2>&1; then
    echo "::error::verify-signature=true but cosign is not installed; add sigstore/cosign-installer" >&2
    exit 1
  fi
  curl -fsSL --retry 3 -o "${tmp}/${archive}.bundle" "${base}/${archive}.bundle"
  cosign verify-blob "${tmp}/${archive}" \
    --bundle "${tmp}/${archive}.bundle" \
    --certificate-identity-regexp "^https://github.com/${repo}/\.github/workflows/release\.yml@refs/tags/" \
    --certificate-oidc-issuer "https://token.actions.githubusercontent.com"
  echo "Signature verified for ${archive}"
fi

case "$EXT" in
  tar.gz) tar -xzf "${tmp}/${archive}" -C "$DEST" ;;
  zip)
    # `unzip` is not guaranteed on a Windows runner — git-bash does not ship it. 7-Zip and
    # PowerShell both are, so try each rather than fail on somebody else's image.
    if command -v unzip >/dev/null 2>&1; then
      unzip -q -o "${tmp}/${archive}" -d "$DEST"
    elif command -v 7z >/dev/null 2>&1; then
      7z x -y -bso0 -bsp0 -o"$DEST" "${tmp}/${archive}"
    elif command -v powershell >/dev/null 2>&1; then
      powershell -NoProfile -Command \
        "Expand-Archive -Path '$(cygpath -w "${tmp}/${archive}" 2>/dev/null || echo "${tmp}/${archive}")' -DestinationPath '$(cygpath -w "$DEST" 2>/dev/null || echo "$DEST")' -Force"
    else
      echo "::error::no unzip, 7z or powershell available to extract ${archive}" >&2
      exit 1
    fi
    ;;
  *)
    echo "::error::unknown archive extension '${EXT}'" >&2
    exit 1
    ;;
esac

chmod +x "${DEST}/${BINARY}" 2>/dev/null || true

reported="$("${DEST}/${BINARY}" --version 2>&1 | awk '{print $2}' || true)"
if [ "$reported" = "${VERSION#v}" ]; then
  echo "Installed notiflow ${reported} -> ${DEST}/${BINARY}"
else
  echo "::warning::installed binary reports version '${reported}', expected '${VERSION#v}'"
fi
