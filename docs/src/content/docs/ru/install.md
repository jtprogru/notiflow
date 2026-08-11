---
title: Установка
description: Homebrew, crates.io, релизный архив — или вообще ничего, если нужен только Action.
sidebar:
  order: 2
---

Если пользуешься только GitHub Action, ставить ничего не надо: Action сам скачивает и
кэширует бинарь.

## Homebrew

```bash
brew install jtprogru/tap/notiflow
```

Покрывает macOS на Apple silicon и Intel, Linux на x86_64 и arm64. Автодополнения шелла
ставятся вместе с формулой.

## crates.io

```bash
cargo install notiflow
```

Собирает из исходников, поэтому работает на любой платформе, которую поддерживает Rust —
включая те, которых нет в релизной матрице. Нужен Rust 1.85 или новее.

## Установка одной строкой

```bash
curl -fsSL https://raw.githubusercontent.com/jtprogru/notiflow/main/scripts/install.sh | sh
```

Скрипт определяет платформу, резолвит последний релиз, сверяет архив с `checksums.txt` из
того же релиза и ставит в `/usr/local/bin`. На системе без glibc сам выбирает статическую
сборку `musl`.

```bash
curl -fsSL .../install.sh | sh -s -- --version v2.0.0 --bin-dir ~/.local/bin
```

Скачать скрипт из интернета прямо в шелл — ровно столько доверия, сколько это и звучит.
[Прочитай его сначала](https://github.com/jtprogru/notiflow/blob/main/scripts/install.sh)
или возьми другой канал.

## Релизный архив

Каждый релиз публикует статический бинарь на платформу плюс `checksums.txt` и cosign-бандл
на каждый архив.

```bash
VERSION=v2.0.0
TARGET=aarch64-apple-darwin        # таблица ниже
BASE="https://github.com/jtprogru/notiflow/releases/download/${VERSION}"

curl -fsSLO "${BASE}/notiflow-${TARGET}.tar.gz"
curl -fsSLO "${BASE}/checksums.txt"
grep " notiflow-${TARGET}.tar.gz$" checksums.txt | shasum -a 256 -c -

tar -xzf "notiflow-${TARGET}.tar.gz"
sudo install -m 0755 notiflow /usr/local/bin/notiflow
```

| Платформа | Target |
|---|---|
| Linux x86_64 (glibc) | `x86_64-unknown-linux-gnu` |
| Linux arm64 (glibc) | `aarch64-unknown-linux-gnu` |
| Linux x86_64 (статика) | `x86_64-unknown-linux-musl` |
| Linux arm64 (статика) | `aarch64-unknown-linux-musl` |
| macOS Apple silicon | `aarch64-apple-darwin` |
| macOS Intel | `x86_64-apple-darwin` |
| Windows x86_64 | `x86_64-pc-windows-msvc` |

Сборки `musl` полностью статические — то, что нужно внутри контейнера на `alpine` или
`scratch`.

### Проверка подписи

Контрольная сумма лежит там же, где архив, поэтому сама по себе она доказывает только
целостность закачки. Чтобы доказать, что архив собран релизным workflow:

```bash
cosign verify-blob "notiflow-${TARGET}.tar.gz" \
  --bundle "notiflow-${TARGET}.tar.gz.bundle" \
  --certificate-identity-regexp '^https://github.com/jtprogru/notiflow/\.github/workflows/release\.yml@refs/tags/' \
  --certificate-oidc-issuer 'https://token.actions.githubusercontent.com'
```

На релизы также выпускаются
[build provenance attestations](https://docs.github.com/en/actions/security-guides/using-artifact-attestations):

```bash
gh attestation verify "notiflow-${TARGET}.tar.gz" --repo jtprogru/notiflow
```

Дополнительно архивы подписаны GPG-ключом мейнтейнера (файлы `.asc` рядом с артефактами).

## Автодополнения

```bash
notiflow completions bash > /etc/bash_completion.d/notiflow
notiflow completions zsh  > "${fpath[1]}/_notiflow"
notiflow completions fish > ~/.config/fish/completions/notiflow.fish
```

`powershell` и `elvish` тоже поддерживаются.

## Внутри Action

Action резолвит версию, скачивает нужный архив, проверяет sha256 и кладёт в tool cache
раннера. Чтобы дополнительно требовать проверку подписи cosign, поставь cosign раньше и
включи вход:

```yaml
- uses: sigstore/cosign-installer@v3
- uses: jtprogru/notiflow@v2
  with:
    verify_signature: true
    # ...
```

### Какую версию ставит Action

По приоритету:

1. вход `version`, если задан;
2. файл `VERSION` в дереве Action — штампуется при релизе, поэтому `@v2`, `@v2.1.0` и
   `@<sha>` дают детерминированный результат;
3. `GITHUB_ACTION_REF`, если это semver-тег;
4. последний релиз через сеть, с предупреждением.

Только последний вариант может дать разный ответ двум запускам одного и того же
запиненного workflow — поэтому он и предупреждает.
