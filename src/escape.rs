//! Escaping of *substituted values* for each parse mode.
//!
//! Only placeholder values are escaped — the template body itself is passed through
//! verbatim, which is what lets a template contain real markup. That split is inherited
//! from v1 and is the whole reason a hostile branch name cannot inject formatting.

use crate::model::ParseMode;

/// Characters MarkdownV2 treats as markup, per the Telegram Bot API spec.
///
/// `\` is handled separately because it must be escaped before the others, otherwise the
/// backslashes we insert would themselves be escaped on a second pass.
pub const MARKDOWN_V2_SPECIALS: &[char] =
    &['_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!'];

/// Escapes every MarkdownV2 metacharacter, plus the escape character itself.
pub fn escape_markdown_v2(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + input.len() / 4);
    for ch in input.chars() {
        if ch == '\\' || MARKDOWN_V2_SPECIALS.contains(&ch) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Escapes the three characters Telegram's HTML parser reacts to.
///
/// `&` first, so the ampersands introduced by `&lt;` / `&gt;` are not re-escaped.
pub fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + input.len() / 8);
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            other => out.push(other),
        }
    }
    out
}

/// Dispatches on parse mode. `ParseMode::None` is the identity.
pub fn escape(input: &str, mode: ParseMode) -> String {
    match mode {
        ParseMode::MarkdownV2 => escape_markdown_v2(input),
        ParseMode::Html => escape_html(input),
        ParseMode::None => input.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_v2_escapes_every_special() {
        for &c in MARKDOWN_V2_SPECIALS {
            let escaped = escape_markdown_v2(&c.to_string());
            assert_eq!(escaped, format!("\\{c}"), "char {c:?}");
        }
    }

    #[test]
    fn markdown_v2_escapes_backslash_once() {
        assert_eq!(escape_markdown_v2("a\\b"), "a\\\\b");
        // A backslash already followed by a special must not collapse into one escape.
        assert_eq!(escape_markdown_v2("\\."), "\\\\\\.");
    }

    #[test]
    fn markdown_v2_leaves_ordinary_text_alone() {
        assert_eq!(escape_markdown_v2("feature/добавить emoji ✅"), "feature/добавить emoji ✅");
    }

    #[test]
    fn html_escapes_ampersand_first() {
        assert_eq!(escape_html("<b>a & b</b>"), "&lt;b&gt;a &amp; b&lt;/b&gt;");
        assert_eq!(escape_html("&lt;"), "&amp;lt;");
    }

    #[test]
    fn none_is_identity() {
        let s = "*not* escaped <at> all & _fine_";
        assert_eq!(escape(s, ParseMode::None), s);
    }

    /// The property v1 could only assert case by case: after escaping, every special
    /// character in the output is preceded by a backslash that is itself not escaped.
    #[test]
    fn property_no_unescaped_special_survives() {
        let corpus = [
            "",
            "plain",
            "a.b_c*d",
            "\\\\\\",
            "!!!___```",
            "русский текст с точкой.",
            "emoji 🎉 and (parens)",
            "{{.Actor}}",
            "-1001234567890",
        ];
        for input in corpus {
            let escaped = escape_markdown_v2(input);
            let chars: Vec<char> = escaped.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                let c = chars[i];
                if c == '\\' {
                    // An escape must consume exactly one following character.
                    assert!(i + 1 < chars.len(), "dangling backslash in {escaped:?}");
                    i += 2;
                    continue;
                }
                assert!(
                    !MARKDOWN_V2_SPECIALS.contains(&c),
                    "unescaped {c:?} in {escaped:?} (input {input:?})"
                );
                i += 1;
            }
        }
    }
}
