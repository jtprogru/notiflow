#!/usr/bin/env bash
# Map the runner's OS/arch onto the Rust target triple used in release asset names.
#
# Release archives are named notiflow-<target>.<ext>:
#   notiflow-x86_64-unknown-linux-gnu.tar.gz
#   notiflow-aarch64-unknown-linux-gnu.tar.gz
#   notiflow-x86_64-apple-darwin.tar.gz
#   notiflow-aarch64-apple-darwin.tar.gz
#   notiflow-x86_64-pc-windows-msvc.zip
#
# Writes `target`, `ext` and `binary` to $GITHUB_OUTPUT.
set -euo pipefail

case "${RUNNER_OS:-}/${RUNNER_ARCH:-}" in
  Linux/X64) target="x86_64-unknown-linux-gnu" ;;
  Linux/ARM64) target="aarch64-unknown-linux-gnu" ;;
  macOS/X64) target="x86_64-apple-darwin" ;;
  macOS/ARM64) target="aarch64-apple-darwin" ;;
  Windows/X64) target="x86_64-pc-windows-msvc" ;;
  Windows/ARM64)
    # Windows on ARM runs x86_64 binaries through emulation, and building a native
    # aarch64-pc-windows-msvc archive for the handful of such runners is not worth a
    # seventh matrix leg. Emulation is correct, just slower to start.
    echo "::notice::no native aarch64 Windows build; using the x86_64 binary under emulation"
    target="x86_64-pc-windows-msvc"
    ;;
  *)
    echo "::error::unsupported runner platform '${RUNNER_OS:-?}/${RUNNER_ARCH:-?}'" >&2
    exit 1
    ;;
esac

if [ "${RUNNER_OS:-}" = "Windows" ]; then
  ext="zip"
  binary="notiflow.exe"
else
  ext="tar.gz"
  binary="notiflow"
fi

{
  echo "target=${target}"
  echo "ext=${ext}"
  echo "binary=${binary}"
} >>"$GITHUB_OUTPUT"

echo "Resolved platform: ${target} (${ext})"
