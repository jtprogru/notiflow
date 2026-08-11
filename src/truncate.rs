//! Cutting a message down to Telegram's 4096 limit without breaking its markup.
//!
//! Telegram counts UTF-16 code units, so a BMP codepoint costs 1 and most emoji cost 2.
//! v1 got the counting right but cut on a raw code-unit boundary, which could sever a
//! MarkdownV2 escape pair (`\` + `.`), an HTML entity (`&amp;`) or a tag — turning "your
//! message was long" into a 400 from Telegram. v2 cuts on markup-token boundaries, and in
//! HTML mode also closes whatever tags were still open at the cut.

use crate::model::ParseMode;

/// Telegram's `sendMessage` / `editMessageText` text limit, in UTF-16 code units.
pub const TELEGRAM_TEXT_LIMIT: usize = 4096;

/// Appended when text had to be cut. Three ASCII dots — three UTF-16 units.
pub const ELLIPSIS: &str = "...";

/// Length of `s` in UTF-16 code units — the unit Telegram's limit is expressed in.
pub fn utf16_len(s: &str) -> usize {
    s.chars().map(char::len_utf16).sum()
}

/// Truncates to the Telegram limit, preserving markup validity.
pub fn truncate(text: &str, mode: ParseMode) -> String {
    truncate_to(text, mode, TELEGRAM_TEXT_LIMIT)
}

/// Same as [`truncate`] with an explicit limit. Split out so tests do not have to build
/// four-thousand-character fixtures.
pub fn truncate_to(text: &str, mode: ParseMode, limit: usize) -> String {
    if utf16_len(text) <= limit {
        return text.to_string();
    }

    let ellipsis_units = utf16_len(ELLIPSIS);
    let mut used = 0usize;
    let mut cut = 0usize;
    let mut open_tags: Vec<&str> = Vec::new();
    let mut closing_units = 0usize;
    let mut i = 0usize;

    while i < text.len() {
        let token = next_token(text, i, mode);
        let slice = &text[i..token.end];
        let units = utf16_len(slice);

        // What the reserved closing-tag budget becomes if we accept this token.
        let mut prospective_closing = closing_units;
        match token.kind {
            TokenKind::OpenTag(name) => prospective_closing += closing_len(name),
            TokenKind::CloseTag(name) if open_tags.last() == Some(&name) => {
                prospective_closing -= closing_len(name);
            }
            _ => {}
        }

        if used + units + ellipsis_units + prospective_closing > limit {
            break;
        }

        match token.kind {
            TokenKind::OpenTag(name) => open_tags.push(name),
            TokenKind::CloseTag(name) if open_tags.last() == Some(&name) => {
                open_tags.pop();
            }
            _ => {}
        }
        closing_units = prospective_closing;
        used += units;
        cut = token.end;
        i = token.end;
    }

    let mut out = String::with_capacity(cut + ellipsis_units + closing_units);
    out.push_str(&text[..cut]);
    out.push_str(ELLIPSIS);
    for name in open_tags.iter().rev() {
        out.push_str("</");
        out.push_str(name);
        out.push('>');
    }
    out
}

fn closing_len(name: &str) -> usize {
    // `</` + name + `>`; tag names are ASCII, so bytes == UTF-16 units.
    name.len() + 3
}

struct Token<'a> {
    end: usize,
    kind: TokenKind<'a>,
}

enum TokenKind<'a> {
    Plain,
    OpenTag(&'a str),
    CloseTag(&'a str),
}

/// Returns the next indivisible markup token starting at byte index `start`.
fn next_token(text: &str, start: usize, mode: ParseMode) -> Token<'_> {
    let rest = &text[start..];
    let first = rest.chars().next().expect("start is inside the string");

    match mode {
        ParseMode::MarkdownV2 => {
            // A backslash and the character it escapes are one unit.
            if first == '\\' {
                let mut it = rest.char_indices();
                it.next();
                if let Some((idx, ch)) = it.next() {
                    return Token { end: start + idx + ch.len_utf8(), kind: TokenKind::Plain };
                }
            }
        }
        ParseMode::Html => {
            if first == '&' {
                if let Some(end) = entity_end(rest) {
                    return Token { end: start + end, kind: TokenKind::Plain };
                }
            }
            if first == '<' {
                if let Some(gt) = rest.find('>') {
                    let inner = &rest[1..gt];
                    let end = start + gt + 1;
                    if let Some(name) = inner.strip_prefix('/') {
                        return Token { end, kind: TokenKind::CloseTag(tag_name(name)) };
                    }
                    if inner.ends_with('/') || inner.is_empty() {
                        return Token { end, kind: TokenKind::Plain };
                    }
                    return Token { end, kind: TokenKind::OpenTag(tag_name(inner)) };
                }
            }
        }
        ParseMode::None => {}
    }

    Token { end: start + first.len_utf8(), kind: TokenKind::Plain }
}

/// The tag name inside `<...>`: everything before the first whitespace.
fn tag_name(inner: &str) -> &str {
    inner.split_whitespace().next().unwrap_or(inner).trim_end_matches('/')
}

/// Byte length of an HTML entity starting at `rest[0] == '&'`, if one is there.
fn entity_end(rest: &str) -> Option<usize> {
    // Longest real entity we care about is `&quot;`; cap the scan so a stray `&` in a
    // 4000-character message does not turn into a linear search to the end.
    let window = &rest[..rest.len().min(12)];
    let semi = window.find(';')?;
    let body = &window[1..semi];
    if body.is_empty() {
        return None;
    }
    let looks_like_entity = body.chars().all(|c| c.is_ascii_alphanumeric())
        || (body.starts_with('#') && body[1..].chars().all(|c| c.is_ascii_alphanumeric()));
    if looks_like_entity { Some(semi + 1) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_counts_surrogate_pairs_as_two() {
        assert_eq!(utf16_len("abc"), 3);
        assert_eq!(utf16_len("привет"), 6);
        assert_eq!(utf16_len("🎉"), 2);
        assert_eq!(utf16_len("a🎉b"), 4);
    }

    #[test]
    fn short_text_passes_through_untouched() {
        let s = "already short";
        assert_eq!(truncate(s, ParseMode::MarkdownV2), s);
    }

    #[test]
    fn long_text_gets_an_ellipsis_and_fits() {
        let s = "x".repeat(5000);
        let out = truncate(&s, ParseMode::None);
        assert!(out.ends_with(ELLIPSIS));
        assert_eq!(utf16_len(&out), TELEGRAM_TEXT_LIMIT);
    }

    #[test]
    fn never_splits_a_markdown_escape_pair() {
        // Every character costs 2 units as an escape pair, so the cut lands mid-pair
        // unless the tokenizer keeps them together.
        let s = "\\.".repeat(50);
        let out = truncate_to(&s, ParseMode::MarkdownV2, 21);
        assert!(utf16_len(&out) <= 21);
        let body = out.strip_suffix(ELLIPSIS).unwrap();
        assert_eq!(body.len() % 2, 0, "cut inside an escape pair: {out:?}");
        assert!(!body.ends_with('\\'));
    }

    #[test]
    fn never_splits_an_html_entity() {
        let s = "&amp;".repeat(20);
        let out = truncate_to(&s, ParseMode::Html, 23);
        let body = out.strip_suffix(ELLIPSIS).unwrap();
        assert_eq!(body, "&amp;&amp;&amp;&amp;");
    }

    #[test]
    fn closes_html_tags_left_open_by_the_cut() {
        let s = format!("<b>{}</b>", "x".repeat(100));
        let out = truncate_to(&s, ParseMode::Html, 30);
        assert!(out.starts_with("<b>"), "{out}");
        assert!(out.ends_with("...</b>"), "{out}");
        assert!(utf16_len(&out) <= 30);
    }

    #[test]
    fn closes_nested_html_tags_in_reverse_order() {
        let s = format!("<b><i>{}</i></b>", "y".repeat(100));
        let out = truncate_to(&s, ParseMode::Html, 40);
        assert!(out.ends_with("...</i></b>"), "{out}");
    }

    #[test]
    fn never_splits_an_html_tag() {
        let s = format!("{}<blockquote>tail</blockquote>", "z".repeat(20));
        let out = truncate_to(&s, ParseMode::Html, 25);
        assert!(!out.contains("<blockquo"), "tag was severed: {out}");
    }

    #[test]
    fn a_closed_tag_frees_its_reserved_budget() {
        // `<b>x</b>` closes itself, so the reserved `</b>` must be released and the rest
        // of the budget spent on real text.
        let s = format!("<b>x</b>{}", "q".repeat(100));
        let out = truncate_to(&s, ParseMode::Html, 30);
        assert!(!out.ends_with("</b>"), "{out}");
        assert!(out.ends_with(ELLIPSIS));
    }

    #[test]
    fn multibyte_is_never_cut_mid_character() {
        let s = "🎉".repeat(3000);
        let out = truncate(&s, ParseMode::None);
        assert!(out.strip_suffix(ELLIPSIS).unwrap().chars().all(|c| c == '🎉'));
        assert!(utf16_len(&out) <= TELEGRAM_TEXT_LIMIT);
    }

    #[test]
    fn degenerate_limit_yields_just_the_ellipsis() {
        assert_eq!(truncate_to("abcdef", ParseMode::None, 3), ELLIPSIS);
    }
}
