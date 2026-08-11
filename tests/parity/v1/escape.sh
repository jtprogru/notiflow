# shellcheck shell=bash
# Escape functions for Telegram parse_mode values.
# Source-only — no shebang. Compatible with bash 3.2+.

[ -n "${_NF_ESCAPE_LOADED:-}" ] && return 0
_NF_ESCAPE_LOADED=1

# nf::escape_md_v2 <text>
# Escapes every special character from the MarkdownV2 spec plus backslash.
# Spec: _ * [ ] ( ) ~ ` > # + - = | { } . !  (and the escape char \ itself)
nf::escape_md_v2() {
  # Order matters: escape backslash first so we don't double-escape later ones.
  # Character class: ] must come right after [, and - must come last (or first).
  printf '%s' "$1" |
    sed -e 's/\\/\\\\/g' \
      -e 's/[][_*()~`>#+=|{}.!-]/\\&/g'
}

# nf::escape_html <text>
# HTML-escapes &, <, > for Telegram parse_mode=HTML. Ampersand must go first.
nf::escape_html() {
  printf '%s' "$1" |
    sed -e 's/&/\&amp;/g' \
      -e 's/</\&lt;/g' \
      -e 's/>/\&gt;/g'
}

# nf::escape_none <text>
# Identity. Provided so callers can dispatch by parse_mode without branching.
nf::escape_none() {
  printf '%s' "$1"
}
