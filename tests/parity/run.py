#!/usr/bin/env python3
"""Run the golden corpus through v1 bash and v2 Rust, and diff them.

The corpus is the artefact that turns "we rewrote it and it seems to work" into a claim
that can fail. Every case records what v1 produces; a case may also record what v2
produces, but only alongside a `divergence` note explaining why the two differ. A v2
result that differs without a declared divergence is a regression, not a fix.

Usage:
    run.py                 # verify
    run.py --bless         # record expectations from the current implementations
    run.py --filter render # only cases whose id contains "render"
"""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
PARITY = ROOT / "tests" / "parity"
CORPUS = PARITY / "corpus"
V1_DRIVER = PARITY / "v1-driver.sh"
ACTION_RUN = ROOT / "scripts" / "action" / "run.sh"

# Environment every case starts from, matching the bats fixture so v1 expectations recorded
# here and assertions written in tests/parity/bats agree.
BASE_ENV = {
    "GITHUB_REPOSITORY": "jtprogru/notiflow",
    "GITHUB_WORKFLOW": "CI",
    "GITHUB_JOB": "test",
    "GITHUB_RUN_ID": "1",
    "GITHUB_RUN_NUMBER": "1",
    "GITHUB_SHA": "abcdef0123456789abcdef0123456789abcdef01",
    "GITHUB_ACTOR": "tester",
    "GITHUB_REF": "refs/heads/main",
    "GITHUB_REF_NAME": "main",
    "GITHUB_EVENT_NAME": "push",
    "GITHUB_SERVER_URL": "https://github.com",
    "NF_BOT_TOKEN": "123456:AAHdqTcvCH1vGWJxfSeofSAs0K5PALDsaw",
    "NF_CHAT_ID": "-1001234567890",
    "NF_STATUS": "success",
}

# Long outputs are compared by length plus tail instead of being pasted into the corpus.
TAIL_LEN = 12


class CaseFailure(Exception):
    pass


def utf16_len(s: str) -> int:
    return len(s.encode("utf-16-le")) // 2


def build_env(case: dict) -> dict:
    env = dict(os.environ)
    # Anything inherited from the caller's shell would silently change a rendering.
    for key in list(env):
        if key.startswith(("NF_", "GITHUB_", "NOTIFLOW_", "INPUT_")):
            del env[key]
    env.update(BASE_ENV)
    for key, value in case.get("env", {}).items():
        if value is None:
            env.pop(key, None)
        else:
            env[key] = value
    for key, spec in case.get("env_repeat", {}).items():
        env[key] = spec["unit"] * spec["times"]
    # The corpus compares two Action runtimes, so v2 must believe it is inside one: that is
    # what disables the local-git placeholder fallback and enforces the api_base allowlist.
    # NF_OUTPUT_FORMAT=json still routes the report to stdout, so nothing needs $GITHUB_OUTPUT.
    env["GITHUB_ACTIONS"] = "true"
    env.pop("GITHUB_OUTPUT", None)
    env.pop("GITHUB_STEP_SUMMARY", None)
    return env


def run_v1(case: dict, env: dict) -> tuple[int, str]:
    mode = {"render": "render", "request": "request", "exit": "validate"}[case["kind"]]
    proc = subprocess.run(
        ["bash", str(V1_DRIVER), mode],
        env=env,
        capture_output=True,
        text=True,
    )
    return proc.returncode, proc.stdout


def run_v2(case: dict, env: dict, binary: str) -> tuple[int, str]:
    env = dict(env)
    env["NF_BIN"] = binary
    env["NF_DRY_RUN"] = "true"
    env["NF_OUTPUT_FORMAT"] = "json"
    proc = subprocess.run(
        ["bash", str(ACTION_RUN)],
        env=env,
        capture_output=True,
        text=True,
    )
    if case["kind"] == "exit" or proc.returncode != 0:
        return proc.returncode, proc.stdout
    try:
        report = json.loads(proc.stdout.strip().splitlines()[-1])
    except (ValueError, IndexError) as exc:
        raise CaseFailure(
            f"v2 produced no JSON report (exit={proc.returncode})\n"
            f"stdout: {proc.stdout!r}\nstderr: {proc.stderr!r}"
        ) from exc
    request = report.get("request")
    if request is None:
        raise CaseFailure(f"v2 report has no request: {report!r}")
    if case["kind"] == "render":
        return 0, request["text"]
    return 0, json.dumps(request, sort_keys=True)


def observed(case: dict, code: int, out: str) -> dict:
    """The comparable observation for one implementation's run of one case.

    The exit code is always part of it. An early version of this harness compared only
    stdout for render cases, and quietly missed that v1 aborts mid-truncation on BSD iconv
    — it had already printed most of the message, so the output looked plausible.
    """
    if case["kind"] == "exit":
        return {"exit": code}
    if code != 0:
        return {"exit": code, "out": out}
    if case["kind"] == "request":
        # v1 emits jq's pretty output; normalise both sides to sorted compact JSON.
        return {"exit": 0, "out": json.dumps(json.loads(out), sort_keys=True)}
    return {"exit": 0, "out": out}


def expectation_fields(observation: dict) -> dict:
    """How an observation is stored: inline when short, length plus tail when not."""
    if "out" not in observation:
        return {"exit": observation["exit"]}
    out = observation["out"]
    if utf16_len(out) > 200:
        return {
            "exit": observation["exit"],
            "utf16_len": utf16_len(out),
            "tail": out[-TAIL_LEN:],
        }
    return {"exit": observation["exit"], "value": out}


def check(expected: dict, actual: dict) -> str | None:
    if expected.get("exit") != actual.get("exit"):
        return f"exit {actual.get('exit')} != {expected.get('exit')}"
    if "value" not in expected and "utf16_len" not in expected:
        return None
    got = actual.get("out")
    if got is None:
        return "expected output but the run produced none"
    if "value" in expected:
        return None if expected["value"] == got else f"got {got!r}, want {expected['value']!r}"
    if utf16_len(got) != expected["utf16_len"]:
        return f"utf16 length {utf16_len(got)} != {expected['utf16_len']}"
    if got[-TAIL_LEN:] != expected["tail"]:
        return f"tail {got[-TAIL_LEN:]!r} != {expected['tail']!r}"
    return None


def load_corpus(filter_: str | None) -> list[tuple[pathlib.Path, dict, int]]:
    cases = []
    for path in sorted(CORPUS.glob("*.json")):
        data = json.loads(path.read_text())
        for index, case in enumerate(data):
            if filter_ and filter_ not in case["id"]:
                continue
            cases.append((path, case, index))
    return cases


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bless", action="store_true", help="record expectations")
    parser.add_argument("--filter", dest="filter", help="only ids containing this substring")
    parser.add_argument(
        "--binary",
        default=str(ROOT / "target" / "debug" / "notiflow"),
        help="path to the v2 binary",
    )
    args = parser.parse_args()

    if not pathlib.Path(args.binary).exists():
        print(f"v2 binary not found at {args.binary}; run `make build` first", file=sys.stderr)
        return 2

    cases = load_corpus(args.filter)
    if not cases:
        print("no cases matched", file=sys.stderr)
        return 2

    failures: list[str] = []
    updates: dict[pathlib.Path, list[dict]] = {}
    diverged = 0

    for path, case, index in cases:
        env = build_env(case)
        try:
            v1 = observed(case, *run_v1(case, env))
            v2 = observed(case, *run_v2(case, env, args.binary))
        except CaseFailure as exc:
            failures.append(f"{case['id']}: {exc}")
            continue

        declared = case.get("divergence")
        differ = v1 != v2

        if differ and not declared:
            failures.append(
                f"{case['id']}: v2 diverges from v1 with no declared reason\n"
                f"    v1: {v1!r}\n    v2: {v2!r}"
            )
        if declared and not differ:
            failures.append(
                f"{case['id']}: declares a divergence ({declared}) but v1 and v2 agree — "
                "delete the declaration or fix the case"
            )
        if differ:
            diverged += 1

        if args.bless:
            case["expected_v1"] = expectation_fields(v1)
            if declared:
                case["expected_v2"] = expectation_fields(v2)
            else:
                case.pop("expected_v2", None)
            updates.setdefault(path, json.loads(path.read_text()))[index] = case
            continue

        if "expected_v1" not in case:
            failures.append(f"{case['id']}: no recorded expectation; run `make parity-bless`")
            continue
        problem = check(case["expected_v1"], v1)
        if problem:
            failures.append(f"{case['id']}: v1 drifted from the record — {problem}")
        expected_v2 = case.get("expected_v2", case["expected_v1"])
        problem = check(expected_v2, v2)
        if problem:
            failures.append(f"{case['id']}: v2 does not match the record — {problem}")

    if args.bless:
        for path, data in updates.items():
            path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n")
        print(f"blessed {len(cases)} case(s) across {len(updates)} file(s)")
        return 0

    for failure in failures:
        print(f"FAIL {failure}", file=sys.stderr)
    print(
        f"parity: {len(cases)} case(s), {diverged} declared divergence(s), {len(failures)} failure(s)"
    )
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
