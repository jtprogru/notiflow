#!/usr/bin/env bats

load helpers

setup() {
  setup_clean_env
  # shellcheck source=../v1/lib.sh
  source "${NF_V1}/lib.sh"
  # shellcheck source=../v1/escape.sh
  source "${NF_V1}/escape.sh"
}

@test "escape md_v2: underscore" {
  [ "$(nf::escape_md_v2 'foo_bar')" = 'foo\_bar' ]
}

@test "escape md_v2: all 18 special characters" {
  for ch in _ '*' '[' ']' '(' ')' '~' '`' '>' '#' '+' - '=' '|' '{' '}' . '!'; do
    result=$(nf::escape_md_v2 "$ch")
    expected='\'"$ch"
    [ "$result" = "$expected" ] || {
      printf 'char=%q expected=%q got=%q\n' "$ch" "$expected" "$result" >&2
      return 1
    }
  done
}

@test "escape md_v2: passthrough on safe text" {
  [ "$(nf::escape_md_v2 'Hello world')" = 'Hello world' ]
}

@test "escape md_v2: backslash is escaped" {
  result=$(nf::escape_md_v2 'foo\bar')
  [ "$result" = 'foo\\bar' ]
}

@test "escape md_v2: combined symbols" {
  result=$(nf::escape_md_v2 'a_b*c[d]')
  [ "$result" = 'a\_b\*c\[d\]' ]
}

@test "escape html: basic special chars" {
  result=$(nf::escape_html '<a&b>')
  [ "$result" = '&lt;a&amp;b&gt;' ]
}

@test "escape html: passthrough on safe text" {
  [ "$(nf::escape_html 'Hello')" = 'Hello' ]
}

@test "escape html: ampersand escaped first" {
  result=$(nf::escape_html '&<>')
  [ "$result" = '&amp;&lt;&gt;' ]
}

@test "escape none: noop" {
  result=$(nf::escape_none '<_*foo*_>')
  [ "$result" = '<_*foo*_>' ]
}

# prop_escape_md_v2_all_specials (CP-6): each spec char is prefixed with backslash.
@test "prop escape md_v2: every special char gets escaped" {
  for ch in _ '*' '[' ']' '(' ')' '~' '`' '>' '#' '+' - '=' '|' '{' '}' . '!' '\'; do
    result=$(nf::escape_md_v2 "$ch")
    first=${result:0:1}
    [ "$first" = '\' ] || {
      printf 'no escape prefix for %q: got %q\n' "$ch" "$result" >&2
      return 1
    }
  done
}
