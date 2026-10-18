//! Lexical helpers shared by the parsers (RFC 3261 §25.1).

pub(crate) fn is_token(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-.!%*_+`'~".contains(&b))
}

/// Splits `s` on `sep`, ignoring separators inside quoted strings and `<...>`.
pub(crate) fn split_unquoted(s: &str, sep: u8) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut in_quotes = false;
    let mut in_angle = false;
    let mut escaped = false;
    for (i, b) in s.bytes().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        match b {
            b'\\' if in_quotes => escaped = true,
            b'"' => in_quotes = !in_quotes,
            b'<' if !in_quotes => in_angle = true,
            b'>' if !in_quotes => in_angle = false,
            _ if b == sep && !in_quotes && !in_angle => {
                parts.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&s[start..]);
    parts
}

/// Byte offset of the first `needle` outside a quoted string.
pub(crate) fn find_unquoted(s: &str, needle: u8) -> Option<usize> {
    let mut in_quotes = false;
    let mut escaped = false;
    for (i, b) in s.bytes().enumerate() {
        if escaped {
            escaped = false;
        } else if b == b'\\' && in_quotes {
            escaped = true;
        } else if b == b'"' {
            in_quotes = !in_quotes;
        } else if b == needle && !in_quotes {
            return Some(i);
        }
    }
    None
}

/// Removes surrounding quotes and backslash escapes from a quoted string.
pub(crate) fn unquote(s: &str) -> Option<String> {
    let inner = s.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            out.push(chars.next()?);
        } else {
            out.push(c);
        }
    }
    Some(out)
}
