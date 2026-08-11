---
title: Installation
description: Homebrew, crates.io, a release archive, or nothing at all if you only use the Action.
sidebar:
  order: 2
---

If you only use the GitHub Action you do not have to install anything — the Action
downloads and caches the binary for you.

## Homebrew

```bash
brew install jtprogru/tap/notiflow
```

Covers macOS on Apple silicon and Intel, and Linux on x86_64 and arm64. Shell completions
are installed with the formula.

## crates.io

```bash
cargo install notiflow
```

Builds from source, so it works on any target Rust supports — including ones the release
matrix does not ship. Needs Rust 1.85 or newer.

## One-line install

```bash
curl -fsSL https://raw.githubusercontent.com/jtprogru/notiflow/main/scripts/install.sh | sh
```

The script detects your platform, resolves the latest release, verifies the archive against
the release's `checksums.txt`, and installs into `/usr/local/bin`. On a system without
glibc it picks the static `musl` build automatically.

```bash
curl -fsSL .../install.sh | sh -s -- --version v2.0.0 --bin-dir ~/.local/bin
```

Piping a script from the internet into a shell is exactly as much trust as it sounds like.
[Read it first](https://github.com/jtprogru/notiflow/blob/main/scripts/install.sh), or use
one of the other channels.

## Release archive

Every release publishes a static binary per platform, plus `checksums.txt` and a cosign
bundle for each archive.

```bash
VERSION=v2.0.0
TARGET=aarch64-apple-darwin        # see the table below
BASE="https://github.com/jtprogru/notiflow/releases/download/${VERSION}"

curl -fsSLO "${BASE}/notiflow-${TARGET}.tar.gz"
curl -fsSLO "${BASE}/checksums.txt"
grep " notiflow-${TARGET}.tar.gz$" checksums.txt | shasum -a 256 -c -

tar -xzf "notiflow-${TARGET}.tar.gz"
sudo install -m 0755 notiflow /usr/local/bin/notiflow
```

| Platform | Target |
|---|---|
| Linux x86_64 (glibc) | `x86_64-unknown-linux-gnu` |
| Linux arm64 (glibc) | `aarch64-unknown-linux-gnu` |
| Linux x86_64 (static) | `x86_64-unknown-linux-musl` |
| Linux arm64 (static) | `aarch64-unknown-linux-musl` |
| macOS Apple silicon | `aarch64-apple-darwin` |
| macOS Intel | `x86_64-apple-darwin` |
| Windows x86_64 | `x86_64-pc-windows-msvc` |

The `musl` builds are fully static, which is what you want inside an `alpine` or `scratch`
container.

### Verifying the signature

The checksum comes from the same server as the archive, so on its own it only proves the
download was not corrupted. To prove the archive is the one the release workflow built:

```bash
cosign verify-blob "notiflow-${TARGET}.tar.gz" \
  --bundle "notiflow-${TARGET}.tar.gz.bundle" \
  --certificate-identity-regexp '^https://github.com/jtprogru/notiflow/\.github/workflows/release\.yml@refs/tags/' \
  --certificate-oidc-issuer 'https://token.actions.githubusercontent.com'
```

Releases also carry [build provenance attestations](https://docs.github.com/en/actions/security-guides/using-artifact-attestations):

```bash
gh attestation verify "notiflow-${TARGET}.tar.gz" --repo jtprogru/notiflow
```

The archives are additionally signed with the maintainer's GPG key (`.asc` files alongside
each artefact).

## Shell completions

```bash
notiflow completions bash > /etc/bash_completion.d/notiflow
notiflow completions zsh  > "${fpath[1]}/_notiflow"
notiflow completions fish > ~/.config/fish/completions/notiflow.fish
```

`powershell` and `elvish` are supported too.

## Inside the Action

The Action resolves a version, downloads the matching archive, verifies its sha256 and
caches it in the runner tool cache. To also require a cosign signature check, install
cosign first and turn the input on:

```yaml
- uses: sigstore/cosign-installer@v3
- uses: jtprogru/notiflow@v2
  with:
    verify_signature: true
    # ...
```

### Which version the Action installs

In order of precedence:

1. the `version` input, when set;
2. the `VERSION` file in the checked-out Action tree — stamped at release time, so `@v2`,
   `@v2.1.0` and `@<sha>` all resolve deterministically;
3. `GITHUB_ACTION_REF`, when it looks like a semver tag;
4. the newest release, resolved over the network, with a warning.

Only the last one can give two runs of the same pinned workflow different answers, which
is why it warns.
