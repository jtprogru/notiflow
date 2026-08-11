#!/usr/bin/env python3
"""Render the Action's inputs and outputs tables straight out of action.yml.

The v1 README drifted from action.yml because both were maintained by hand. Generating the
tables means the only way to change the documented contract is to change the contract.

Deliberately dependency-free: no PyYAML, because the docs job should not need a Python
environment beyond what every runner already has. action.yml is written by this repository
and stays inside the small subset parsed here.
"""

from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
ACTION = ROOT / "action.yml"
OUT_DIR = ROOT / "docs" / "src" / "generated"


def unquote(value: str) -> str:
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "'\"":
        return value[1:-1]
    return value


def parse_section(lines: list[str], section: str) -> dict[str, dict[str, str]]:
    """Pull one top-level mapping (`inputs:` / `outputs:`) out of action.yml.

    Handles the three shapes this file uses: `key: value`, `key: >- <folded block>`, and
    nested two-space entries. Anything else is a signal that action.yml grew a construct
    this generator has to learn, so it raises rather than guessing.
    """
    entries: dict[str, dict[str, str]] = {}
    in_section = False
    current: str | None = None
    field: str | None = None
    folded: list[str] = []

    def flush_folded() -> None:
        nonlocal folded, field
        if current is not None and field is not None and folded:
            entries[current][field] = " ".join(part.strip() for part in folded).strip()
        folded = []
        field = None

    for raw in lines:
        line = raw.rstrip("\n")
        if not line.strip() or line.lstrip().startswith("#"):
            continue

        if not line.startswith(" "):
            flush_folded()
            in_section = line.strip() == f"{section}:"
            current = None
            continue
        if not in_section:
            continue

        indent = len(line) - len(line.lstrip())
        stripped = line.strip()

        if indent == 2 and stripped.endswith(":"):
            flush_folded()
            current = stripped[:-1]
            entries[current] = {}
            continue

        if indent == 4 and current is not None:
            key, _, value = stripped.partition(":")
            value = value.strip()
            if value in (">-", ">", "|", "|-"):
                flush_folded()
                field = key
                continue
            flush_folded()
            entries[current][key] = unquote(value)
            continue

        if indent >= 6 and field is not None:
            folded.append(stripped)
            continue

        raise SystemExit(f"gen-action-tables: unhandled line in {section}: {line!r}")

    flush_folded()
    return entries


def escape_cell(text: str) -> str:
    return text.replace("|", "\\|")


def render_inputs(inputs: dict[str, dict[str, str]]) -> str:
    out = ["| Input | Required | Default | Description |", "|---|---|---|---|"]
    for name, fields in inputs.items():
        required = "yes" if fields.get("required") == "true" else "no"
        default = fields.get("default", "")
        # `${{ github.token }}` would be evaluated by GitHub if it appeared verbatim in a
        # rendered page inside this repository's own workflows.
        default = re.sub(r"\$\{\{(.+?)\}\}", r"`${{\1}}`", default)
        default = f"`{default}`" if default and not default.startswith("`") else default or "—"
        out.append(
            f"| `{name}` | {required} | {default} | {escape_cell(fields.get('description', ''))} |"
        )
    return "\n".join(out) + "\n"


def render_outputs(outputs: dict[str, dict[str, str]]) -> str:
    out = ["| Output | Description |", "|---|---|"]
    for name, fields in outputs.items():
        out.append(f"| `{name}` | {escape_cell(fields.get('description', ''))} |")
    return "\n".join(out) + "\n"


def main() -> int:
    lines = ACTION.read_text().splitlines(keepends=True)
    inputs = parse_section(lines, "inputs")
    outputs = parse_section(lines, "outputs")
    if not inputs or not outputs:
        print("gen-action-tables: parsed nothing; action.yml changed shape", file=sys.stderr)
        return 1

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    (OUT_DIR / "action-inputs.md").write_text(render_inputs(inputs))
    (OUT_DIR / "action-outputs.md").write_text(render_outputs(outputs))
    print(f"generated action-inputs.md ({len(inputs)} inputs), action-outputs.md ({len(outputs)} outputs)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
