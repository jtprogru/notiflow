#!/usr/bin/env bats

load helpers

setup() {
  setup_clean_env
  # shellcheck source=../scripts/lib.sh
  source "${NF_ROOT}/scripts/lib.sh"
  # shellcheck source=../scripts/escape.sh
  source "${NF_ROOT}/scripts/escape.sh"
  # shellcheck source=../scripts/render.sh
  source "${NF_ROOT}/scripts/render.sh"
}

@test "render: NF_MESSAGE returned verbatim (REQ-5.1)" {
  export NF_STATUS=failure NF_MESSAGE="hello world" NF_TEMPLATE_FAILURE="X" NF_PARSE_MODE=MarkdownV2
  result=$(nf::render)
  [ "$result" = "hello world" ]
}

@test "render: template_<status> wins over message_template (REQ-5.2)" {
  export NF_STATUS=failure NF_TEMPLATE_FAILURE="A-template" NF_MESSAGE_TEMPLATE="B-fallback" NF_PARSE_MODE=none
  result=$(nf::render)
  [ "$result" = "A-template" ]
}

@test "render: message_template used when no per-status template (REQ-5.3)" {
  export NF_STATUS=success NF_MESSAGE_TEMPLATE="ok" NF_PARSE_MODE=none
  result=$(nf::render)
  [ "$result" = "ok" ]
}

@test "render: default template includes status emoji and repo (REQ-4.1)" {
  export NF_STATUS=success NF_PARSE_MODE=none
  result=$(nf::render)
  [[ "$result" == *"✅"* ]]
  [[ "$result" == *"jtprogru/notiflow"* ]]
  [[ "$result" == *"CI"* ]]
  [[ "$result" == *"abcdef0"* ]]
  [[ "$result" == *"tester"* ]]
}

@test "render: emoji per status (REQ-4.2..4.5, CP-14)" {
  export NF_PARSE_MODE=none NF_MESSAGE_TEMPLATE="{{.StatusEmoji}}|{{.Status}}"
  for pair in 'success|✅' 'failure|❌' 'cancelled|⚠️' 'skipped|⏭'; do
    s=${pair%%|*}
    e=${pair##*|}
    NF_STATUS=$s result=$(nf::render)
    [ "$result" = "$e|$s" ] || {
      echo "fail $s -> got $result" >&2
      return 1
    }
  done
}

@test "render: simple placeholder Repo (REQ-5.4)" {
  export NF_STATUS=success NF_PARSE_MODE=none NF_MESSAGE_TEMPLATE='Repo={{.Repo}}'
  result=$(nf::render)
  [ "$result" = "Repo=jtprogru/notiflow" ]
}

@test "render: all 16 known placeholders substitute" {
  export NF_STATUS=success NF_PARSE_MODE=none
  export NF_MESSAGE_TEMPLATE='r={{.Repo}}|w={{.Workflow}}|j={{.Job}}|s={{.Status}}|e={{.StatusEmoji}}|a={{.Actor}}|ref={{.Ref}}|rn={{.RefName}}|br={{.Branch}}|sha={{.Sha}}|ssha={{.ShortSha}}|ri={{.RunId}}|num={{.RunNumber}}|url={{.RunUrl}}|ev={{.EventName}}|su={{.ServerUrl}}'
  result=$(nf::render)
  [[ "$result" == *"r=jtprogru/notiflow"* ]]
  [[ "$result" == *"w=CI"* ]]
  [[ "$result" == *"j=test"* ]]
  [[ "$result" == *"s=success"* ]]
  [[ "$result" == *"e=✅"* ]]
  [[ "$result" == *"a=tester"* ]]
  [[ "$result" == *"ref=refs/heads/main"* ]]
  [[ "$result" == *"rn=main"* ]]
  [[ "$result" == *"br=main"* ]]
  [[ "$result" == *"sha=abcdef0123456789abcdef0123456789abcdef01"* ]]
  [[ "$result" == *"ssha=abcdef0"* ]]
  [[ "$result" == *"ri=1"* ]]
  [[ "$result" == *"num=1"* ]]
  [[ "$result" == *"url=https://github.com/jtprogru/notiflow/actions/runs/1"* ]]
  [[ "$result" == *"ev=push"* ]]
  [[ "$result" == *"su=https://github.com"* ]]
}

@test "render: unknown placeholder removed with warning (REQ-5.5)" {
  export NF_STATUS=success NF_PARSE_MODE=none NF_MESSAGE_TEMPLATE='X={{.Wat}}Z'
  result=$(nf::render 2>/tmp/nf_render_stderr)
  [ "$result" = "X=Z" ]
  grep -q "UNKNOWN_PLACEHOLDER:Wat" /tmp/nf_render_stderr
}

@test "render: MarkdownV2 escapes value (REQ-5.6)" {
  export NF_STATUS=success NF_PARSE_MODE=MarkdownV2 NF_MESSAGE_TEMPLATE='{{.Repo}}'
  GITHUB_REPOSITORY='foo_bar/baz' result=$(nf::render)
  [ "$result" = 'foo\_bar/baz' ]
}

@test "render: parse_mode=none does not escape" {
  export NF_STATUS=success NF_PARSE_MODE=none NF_MESSAGE_TEMPLATE='{{.Repo}}'
  GITHUB_REPOSITORY='foo_bar/baz' result=$(nf::render)
  [ "$result" = 'foo_bar/baz' ]
}

@test "render: truncate ASCII at 4096 UTF-16 units (REQ-5.7)" {
  export NF_STATUS=success NF_PARSE_MODE=none
  big=$(printf 'a%.0s' $(seq 1 5000))
  NF_MESSAGE="$big" result=$(nf::render)
  [ "${#result}" -eq 4096 ]
  [ "${result: -3}" = "..." ]
}

@test "render: passthrough at exactly 4096 UTF-16 units" {
  export NF_STATUS=success NF_PARSE_MODE=none
  msg=$(printf 'a%.0s' $(seq 1 4096))
  NF_MESSAGE="$msg" result=$(nf::render)
  [ "${#result}" -eq 4096 ]
  [ "${result: -3}" = "aaa" ]
}

@test "render: 4097 input → 4096 with ellipsis" {
  export NF_STATUS=success NF_PARSE_MODE=none
  msg=$(printf 'a%.0s' $(seq 1 4097))
  NF_MESSAGE="$msg" result=$(nf::render)
  [ "${#result}" -eq 4096 ]
  [ "${result: -3}" = "..." ]
}

@test "render: BMP (Cyrillic) at 4096 codepoints passes through" {
  export NF_STATUS=success NF_PARSE_MODE=none
  # Each Cyrillic letter = 1 UTF-16 unit (BMP), 2 UTF-8 bytes.
  msg=$(printf 'а%.0s' $(seq 1 4096))
  NF_MESSAGE="$msg" result=$(nf::render)
  [ "$(_nf::_utf16_units "$result")" -eq 4096 ]
  [ "${result: -1}" = "а" ]
}

@test "render: BMP (Cyrillic) at 4097 codepoints truncates with ellipsis" {
  export NF_STATUS=success NF_PARSE_MODE=none
  msg=$(printf 'а%.0s' $(seq 1 4097))
  NF_MESSAGE="$msg" result=$(nf::render)
  [ "$(_nf::_utf16_units "$result")" -eq 4096 ]
  [ "${result: -3}" = "..." ]
}

@test "render: supplementary-plane emoji each count as 2 UTF-16 units" {
  # 😀 (U+1F600) = surrogate pair = 2 UTF-16 units. 2049 emoji = 4098 units.
  msg=$(printf '😀%.0s' $(seq 1 2049))
  export NF_STATUS=success NF_PARSE_MODE=none
  NF_MESSAGE="$msg" result=$(nf::render)
  # Must fit within Telegram's 4096-unit limit (the very thing the old
  # codepoint-based truncator missed — it would have let 2049 emoji = 8192
  # bytes through and Telegram would have returned 400 MESSAGE_TOO_LONG).
  [ "$(_nf::_utf16_units "$result")" -le 4096 ]
  [ "${result: -3}" = "..." ]
}

@test "render: emoji truncation preserves codepoint boundaries (no half emoji)" {
  msg=$(printf '😀%.0s' $(seq 1 2049))
  export NF_STATUS=success NF_PARSE_MODE=none
  NF_MESSAGE="$msg" result=$(nf::render)
  [ "${result: -3}" = "..." ]
  # Any half-surrogate that survived would either appear as U+FFFD or push
  # the unit count above the limit. The previous test guards the count;
  # this one guards against silent corruption.
  ! printf '%s' "$result" | grep -q $'�'
}

@test "render: placeholder value with embedded newline survives substitution" {
  # The old sed-based substitution stripped \n via `tr -d '\n'`. The bash
  # parameter-expansion replacement preserves them. Workflow names in
  # particular can contain newlines (it's a free-form string from the user).
  export NF_STATUS=success NF_PARSE_MODE=none
  export GITHUB_WORKFLOW=$'line1\nline2'
  result=$(NF_MESSAGE_TEMPLATE='before {{.Workflow}} after' nf::render)
  [ "$result" = $'before line1\nline2 after' ]
}

@test "render: placeholder value with sed-metachars / & \\\\ is literal" {
  # The old code escaped these for sed RHS. Bash param-expansion replacement
  # treats the value literally — no double-escape, no second-order injection.
  export NF_STATUS=success NF_PARSE_MODE=none
  export GITHUB_REPOSITORY='owner/repo'
  result=$(NF_MESSAGE_TEMPLATE='{{.Repo}}' nf::render)
  [ "$result" = 'owner/repo' ]
  export GITHUB_REPOSITORY='a&b\c'
  result=$(NF_MESSAGE_TEMPLATE='{{.Repo}}' nf::render)
  [ "$result" = 'a&b\c' ]
}

@test "utf16_units: counts BMP=1 and supplementary=2 (CP-UTF16)" {
  [ "$(_nf::_utf16_units '')" -eq 0 ]
  [ "$(_nf::_utf16_units 'abc')" -eq 3 ]
  [ "$(_nf::_utf16_units 'абв')" -eq 3 ]
  [ "$(_nf::_utf16_units '😀')" -eq 2 ]
  [ "$(_nf::_utf16_units 'abc😀')" -eq 5 ]
}

# CP-4: template priority property test.
# Env-prefix doesn't propagate into $(...) substitutions, so set/unset explicitly.
@test "prop render: template priority order (CP-4)" {
  export NF_STATUS=failure NF_PARSE_MODE=none

  # message present → message wins over both templates
  export NF_MESSAGE="MSG" NF_TEMPLATE_FAILURE="TS" NF_MESSAGE_TEMPLATE="MT"
  [ "$(nf::render)" = "MSG" ]

  # no message, t_status present → t_status wins
  unset NF_MESSAGE
  [ "$(nf::render)" = "TS" ]

  # only message_template
  unset NF_TEMPLATE_FAILURE
  [ "$(nf::render)" = "MT" ]

  # nothing → default template (contains failure emoji)
  unset NF_MESSAGE_TEMPLATE
  result=$(nf::render)
  [[ "$result" == *"❌"* ]]
}

# CP-5: no leftover {{.X}} after render
@test "prop render: no unsubstituted placeholders (CP-5)" {
  export NF_STATUS=success NF_PARSE_MODE=none
  export NF_MESSAGE_TEMPLATE='{{.Repo}} {{.Workflow}} {{.Job}} {{.Status}} {{.StatusEmoji}} {{.Actor}} {{.Ref}} {{.RefName}} {{.Branch}} {{.Sha}} {{.ShortSha}} {{.RunId}} {{.RunNumber}} {{.RunUrl}} {{.EventName}} {{.ServerUrl}} {{.UnknownA}} {{.UnknownB}}'
  result=$(nf::render 2>/dev/null)
  ! printf '%s' "$result" | grep -qE '\{\{\.[A-Za-z]+\}\}'
}

# CP-7: truncate length invariant across multiple non-empty sizes.
# (n=0 is excluded because empty NF_MESSAGE falls through to the default template.)
@test "prop render: length truncation invariant (CP-7)" {
  export NF_STATUS=success NF_PARSE_MODE=none
  for n in 1 100 4095 4096 4097 8000; do
    msg=$(printf 'a%.0s' $(seq 1 "$n"))
    NF_MESSAGE="$msg" result=$(nf::render)
    if [ "$n" -le 4096 ]; then
      [ "${#result}" -eq "$n" ] || {
        echo "n=$n got ${#result}" >&2
        return 1
      }
    else
      [ "${#result}" -eq 4096 ] || {
        echo "n=$n got ${#result}" >&2
        return 1
      }
      [ "${result: -3}" = "..." ] || {
        echo "n=$n suffix wrong" >&2
        return 1
      }
    fi
  done
}
