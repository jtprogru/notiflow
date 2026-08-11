.DEFAULT_GOAL := help
SHELL := /usr/bin/env bash

CARGO   ?= cargo
BIN     := notiflow
MSRV    := $(shell awk -F'"' '/^rust-version/ {print $$2}' Cargo.toml)
TARGET  ?=
DEBUG_BIN := target/debug/$(BIN)

# Generated documentation fragments. `gen-check` fails when these drift from the code,
# which is what stops the tables in docs/ from quietly going stale.
GEN_DIR   := docs/src/generated
GEN_FILES := $(GEN_DIR)/exit-codes.md $(GEN_DIR)/placeholders.md $(GEN_DIR)/cli.md \
             $(GEN_DIR)/action-inputs.md $(GEN_DIR)/action-outputs.md

.PHONY: help
help: ## Show this help
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n\nTargets:\n"} \
	     /^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

# --- build ------------------------------------------------------------------

.PHONY: build
build: ## Build the debug binary
	$(CARGO) build

.PHONY: release
release: ## Build the optimized binary
	$(CARGO) build --release --locked

.PHONY: run
run: ## Run the binary (make run ARGS="send -m hi")
	$(CARGO) run -- $(ARGS)

.PHONY: install
install: ## Install the binary into ~/.cargo/bin
	$(CARGO) install --path . --locked

# --- quality ----------------------------------------------------------------

.PHONY: fmt
fmt: ## Format the Rust sources
	$(CARGO) fmt

.PHONY: fmt-check
fmt-check: ## Fail if the sources are not formatted
	$(CARGO) fmt --check

.PHONY: clippy
clippy: ## Lint with clippy, warnings are errors
	$(CARGO) clippy --all-targets --all-features -- -D warnings

.PHONY: shellcheck
shellcheck: ## Lint the shell wrappers
	@command -v shellcheck >/dev/null || { echo "shellcheck not found; run: make install-tools"; exit 1; }
	shellcheck -x scripts/install.sh scripts/action/*.sh tests/parity/v1-driver.sh
	@if command -v shfmt >/dev/null; then shfmt -d -i 2 -ci scripts tests/parity/v1-driver.sh; fi

.PHONY: actionlint
actionlint: ## Lint the workflow definitions
	@if command -v actionlint >/dev/null; then actionlint; \
	 else echo "actionlint not found — skipping (run: make install-tools)"; fi

.PHONY: lint
lint: fmt-check clippy shellcheck actionlint ## Run every linter

.PHONY: check
check: ## Type-check without producing a binary
	$(CARGO) check --all-targets

# --- tests ------------------------------------------------------------------

.PHONY: test
test: ## Run the Rust unit, integration and doc tests
	$(CARGO) test --all-features

.PHONY: test-v1
test-v1: ## Run the frozen v1 bats suite (the parity reference)
	@command -v bats >/dev/null || { echo "bats not found; run: make install-tools"; exit 1; }
	bats tests/parity/bats

.PHONY: test-action
test-action: build ## Run the Action wrapper acceptance tests
	@command -v bats >/dev/null || { echo "bats not found; run: make install-tools"; exit 1; }
	bats tests/action

.PHONY: action-smoke
action-smoke: build ## Drive the Action wrapper against a local mock Telegram
	NF_BIN=$(CURDIR)/$(DEBUG_BIN) bash scripts/action/smoke.sh

.PHONY: parity
parity: build ## Diff the golden corpus between v1 bash and v2 Rust
	python3 tests/parity/run.py

.PHONY: parity-bless
parity-bless: build ## Re-record the corpus expectations (review the diff before committing)
	python3 tests/parity/run.py --bless

.PHONY: msrv
msrv: ## Verify the crate still builds on its minimum supported Rust version
	@command -v rustup >/dev/null || { echo "rustup is required for the MSRV check"; exit 1; }
	rustup toolchain install $(MSRV) --profile minimal --no-self-update
	$(CARGO) +$(MSRV) check --all-targets --locked

.PHONY: audit
audit: ## Check dependencies against the RustSec advisory database
	@command -v cargo-audit >/dev/null || $(CARGO) install cargo-audit --locked
	$(CARGO) audit

.PHONY: deny
deny: ## Check licences, advisories, sources and duplicate dependencies
	@command -v cargo-deny >/dev/null || $(CARGO) install cargo-deny --locked
	$(CARGO) deny check

# --- generated docs ---------------------------------------------------------

.PHONY: gen
gen: build ## Regenerate the documentation fragments from the code
	@mkdir -p $(GEN_DIR)
	$(DEBUG_BIN) docs exit-codes    > $(GEN_DIR)/exit-codes.md
	$(DEBUG_BIN) docs placeholders  > $(GEN_DIR)/placeholders.md
	$(DEBUG_BIN) docs cli           > $(GEN_DIR)/cli.md
	python3 scripts/gen-action-tables.py
	@echo "regenerated: $(GEN_FILES)"

.PHONY: gen-check
gen-check: ## Fail when the committed fragments differ from a fresh generation
	@$(MAKE) --no-print-directory gen
	@if ! git diff --quiet -- $(GEN_DIR); then \
	  echo "generated docs are out of date; run 'make gen' and commit the result" >&2; \
	  git --no-pager diff -- $(GEN_DIR); \
	  exit 1; \
	fi

# --- documentation site -----------------------------------------------------

.PHONY: docs-install
docs-install: ## Install the docs site dependencies
	cd docs && npm ci

.PHONY: docs-dev
docs-dev: ## Serve the docs site with live reload
	cd docs && npm run dev

.PHONY: docs-build
docs-build: gen ## Build the static docs site
	cd docs && npm run build

.PHONY: docs-preview
docs-preview: ## Serve the built docs site
	cd docs && npm run preview

# --- distribution -----------------------------------------------------------

.PHONY: dist
dist: ## Cross-build one release archive (make dist TARGET=aarch64-apple-darwin)
	@test -n "$(TARGET)" || { echo "usage: make dist TARGET=<rust-target>"; exit 1; }
	$(CARGO) build --release --locked --target $(TARGET)
	@mkdir -p dist
	@if [[ "$(TARGET)" == *windows* ]]; then \
	  (cd target/$(TARGET)/release && zip -q -X "$(CURDIR)/dist/$(BIN)-$(TARGET).zip" $(BIN).exe); \
	else \
	  tar -czf dist/$(BIN)-$(TARGET).tar.gz -C target/$(TARGET)/release $(BIN); \
	fi
	@echo "dist/$(BIN)-$(TARGET).*"

.PHONY: dist-all
dist-all: ## Cross-build every target the release matrix produces
	@for t in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu \
	          x86_64-unknown-linux-musl aarch64-unknown-linux-musl \
	          x86_64-apple-darwin aarch64-apple-darwin x86_64-pc-windows-msvc; do \
	  $(MAKE) --no-print-directory dist TARGET=$$t || echo "skipped $$t"; \
	done

.PHONY: version-check
version-check: ## Verify Cargo.toml and VERSION agree with a tag (make version-check TAG=v2.0.0)
	@test -n "$(TAG)" || { echo "usage: make version-check TAG=vX.Y.Z"; exit 1; }
	@want="$(TAG)"; want="$${want#v}"; \
	 cargo_version="$$(awk -F'"' '/^version = / {print $$2; exit}' Cargo.toml)"; \
	 file_version="$$(tr -d '[:space:]' < VERSION)"; file_version="$${file_version#v}"; \
	 fail=0; \
	 if [ "$$cargo_version" != "$$want" ]; then \
	   echo "Cargo.toml version is $$cargo_version, tag says $$want" >&2; fail=1; fi; \
	 if [ "$$file_version" != "$$want" ]; then \
	   echo "VERSION is $$file_version, tag says $$want" >&2; fail=1; fi; \
	 if [ $$fail -ne 0 ]; then \
	   echo "run 'make release-prep VERSION=$$want' before tagging" >&2; exit 1; fi; \
	 echo "version $$want is consistent across Cargo.toml, VERSION and the tag"

.PHONY: release-prep
release-prep: ## Stamp a version into Cargo.toml and VERSION (make release-prep VERSION=2.0.0)
	@test -n "$(VERSION)" || { echo "usage: make release-prep VERSION=X.Y.Z"; exit 1; }
	@v="$(VERSION)"; v="$${v#v}"; \
	 sed -i.bak -E '0,/^version = /s//version = "'"$$v"'"/' Cargo.toml && rm -f Cargo.toml.bak; \
	 printf 'v%s\n' "$$v" > VERSION; \
	 $(CARGO) update --workspace --quiet; \
	 echo "stamped $$v — now update CHANGELOG.md, commit, and tag v$$v"

.PHONY: checksums
checksums: ## Write dist/checksums.txt for whatever is in dist/
	@cd dist && { command -v sha256sum >/dev/null && sha256sum $(BIN)-* || shasum -a 256 $(BIN)-*; } > checksums.txt
	@cat dist/checksums.txt

# --- aggregate --------------------------------------------------------------

.PHONY: ci
ci: lint test gen-check parity ## Everything CI runs, in the order CI runs it

.PHONY: install-tools
install-tools: ## Install the external tools the targets above expect
	@if [ "$$(uname)" = "Darwin" ]; then \
	  brew install bats-core shellcheck shfmt actionlint jq; \
	elif command -v apt-get >/dev/null; then \
	  sudo apt-get update && sudo apt-get install -y bats shellcheck jq; \
	  curl -sSfL -o /tmp/shfmt https://github.com/mvdan/sh/releases/download/v3.7.0/shfmt_v3.7.0_linux_amd64; \
	  sudo install -m 0755 /tmp/shfmt /usr/local/bin/shfmt; \
	  bash <(curl -sSfL https://raw.githubusercontent.com/rhysd/actionlint/main/scripts/download-actionlint.bash); \
	  sudo install -m 0755 ./actionlint /usr/local/bin/actionlint; \
	else \
	  echo "unsupported OS — install bats-core, shellcheck, shfmt, actionlint and jq by hand"; exit 1; \
	fi
	$(CARGO) install cargo-deny cargo-audit --locked

.PHONY: clean
clean: ## Remove build artefacts
	$(CARGO) clean
	rm -rf dist docs/dist docs/.astro
