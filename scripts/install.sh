#!/usr/bin/env sh
# notiflow installer.
#
#   curl -fsSL https://raw.githubusercontent.com/jtprogru/notiflow/main/scripts/install.sh | sh
#   curl -fsSL .../install.sh | sh -s -- --version v2.0.0 --bin-dir ~/.local/bin
#
# POSIX sh, not bash: this has to run on an alpine container whose only shell is ash.
# Verifies the archive against the release's checksums.txt before unpacking anything.
set -eu

REPO="jtprogru/notiflow"
VERSION=""
BIN_DIR="${NOTIFLOW_BIN_DIR:-/usr/local/bin}"
MUSL=""

usage() {
  cat <<'EOF'
Usage: install.sh [--version vX.Y.Z] [--bin-dir DIR] [--musl]

  --version   release to install (default: the latest one)
  --bin-dir   where to put the binary (default: /usr/local/bin)
  --musl      force the statically linked Linux build
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --version)
      VERSION="${2:?--version needs a value}"
      shift 2
      ;;
    --bin-dir)
      BIN_DIR="${2:?--bin-dir needs a value}"
      shift 2
      ;;
    --musl)
      MUSL=1
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

die() {
  echo "install.sh: $*" >&2
  exit 1
}

need() {
  command -v "$1" >/dev/null 2>&1 || die "$1 is required"
}

need curl
need tar
need uname

os="$(uname -s)"
arch="$(uname -m)"

case "$arch" in
  x86_64 | amd64) arch=x86_64 ;;
  aarch64 | arm64) arch=aarch64 ;;
  *) die "unsupported architecture: $arch" ;;
esac

case "$os" in
  Linux)
    # A system without glibc (alpine and friends) needs the static build. `ldd --version`
    # printing "musl" is the cheapest reliable signal.
    if [ -n "$MUSL" ] || ! ldd --version 2>&1 | grep -qi gnu; then
      target="${arch}-unknown-linux-musl"
    else
      target="${arch}-unknown-linux-gnu"
    fi
    ;;
  Darwin) target="${arch}-apple-darwin" ;;
  *) die "unsupported OS: $os (on Windows, download the .zip from the releases page)" ;;
esac

if [ -z "$VERSION" ]; then
  VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" |
    sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)"
  [ -n "$VERSION" ] || die "could not resolve the latest release; pass --version"
fi

archive="notiflow-${target}.tar.gz"
base="https://github.com/${REPO}/releases/download/${VERSION}"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

echo "Downloading ${archive} (${VERSION})"
curl -fsSL --retry 3 -o "${tmp}/${archive}" "${base}/${archive}" ||
  die "download failed — does ${VERSION} publish ${target}?"
curl -fsSL --retry 3 -o "${tmp}/checksums.txt" "${base}/checksums.txt"

expected="$(grep -E "[ *]${archive}\$" "${tmp}/checksums.txt" | awk '{print $1}')"
[ -n "$expected" ] || die "${archive} is not listed in checksums.txt"

if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "${tmp}/${archive}" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
  actual="$(shasum -a 256 "${tmp}/${archive}" | awk '{print $1}')"
else
  die "neither sha256sum nor shasum is available; cannot verify the download"
fi
[ "$expected" = "$actual" ] || die "sha256 mismatch: expected ${expected}, got ${actual}"
echo "Checksum verified"

tar -xzf "${tmp}/${archive}" -C "$tmp"
chmod +x "${tmp}/notiflow"

if [ -w "$BIN_DIR" ] || mkdir -p "$BIN_DIR" 2>/dev/null && [ -w "$BIN_DIR" ]; then
  mv "${tmp}/notiflow" "${BIN_DIR}/notiflow"
elif command -v sudo >/dev/null 2>&1; then
  echo "${BIN_DIR} is not writable; using sudo"
  sudo install -m 0755 "${tmp}/notiflow" "${BIN_DIR}/notiflow"
else
  die "${BIN_DIR} is not writable and sudo is unavailable; try --bin-dir ~/.local/bin"
fi

echo "Installed $("${BIN_DIR}/notiflow" --version) to ${BIN_DIR}/notiflow"

case ":${PATH}:" in
  *":${BIN_DIR}:"*) ;;
  *) echo "Note: ${BIN_DIR} is not on your PATH" ;;
esac
