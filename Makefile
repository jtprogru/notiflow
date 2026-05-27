.DEFAULT_GOAL := help
SHELL := /usr/bin/env bash

SCRIPTS := scripts
TESTS   := tests

.PHONY: help
help: ## Show this help
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n\nTargets:\n"} \
	     /^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

.PHONY: lint
lint: ## Run shellcheck, shfmt -d and actionlint
	@command -v shellcheck >/dev/null || { echo "shellcheck not found; run: make install-tools"; exit 1; }
	@command -v shfmt      >/dev/null || { echo "shfmt not found; run: make install-tools"; exit 1; }
	shellcheck -x $(SCRIPTS)/*.sh $(TESTS)/*.bash
	shfmt -d -i 2 -ci $(SCRIPTS) $(TESTS)
	@if command -v actionlint >/dev/null; then \
	  actionlint; \
	else \
	  echo "actionlint not found — skipping (run: make install-tools)"; \
	fi

.PHONY: lint-fix
lint-fix: ## Auto-format shell scripts with shfmt
	shfmt -w -i 2 -ci $(SCRIPTS) $(TESTS)

.PHONY: test
test: ## Run bats test suite
	@command -v bats >/dev/null || { echo "bats not found; run: make install-tools"; exit 1; }
	@command -v jq   >/dev/null || { echo "jq not found; install with: brew install jq"; exit 1; }
	bats $(TESTS)

.PHONY: build
build: ## No-op (composite action — nothing to build)
	@echo "composite action — nothing to build"

.PHONY: readme
readme: ## Regenerate README inputs/outputs section (not implemented in v1)
	@echo "TODO: not implemented in v1"

.PHONY: install-tools
install-tools: ## Install bats, shellcheck, shfmt, actionlint
	@if [ "$$(uname)" = "Darwin" ]; then \
	  brew install bats-core shellcheck shfmt actionlint jq; \
	elif command -v apt-get >/dev/null; then \
	  sudo apt-get update && sudo apt-get install -y bats shellcheck jq; \
	  curl -sSfL https://github.com/mvdan/sh/releases/latest/download/shfmt_v3.7.0_linux_amd64 -o /usr/local/bin/shfmt && chmod +x /usr/local/bin/shfmt; \
	  bash <(curl -sSfL https://raw.githubusercontent.com/rhysd/actionlint/main/scripts/download-actionlint.bash); \
	else \
	  echo "Unsupported OS — install bats-core, shellcheck, shfmt, actionlint, jq manually"; exit 1; \
	fi
