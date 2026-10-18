//! Basic rules (RFC 3261 §25.1, with core rules from RFC 2234).

use nom::Parser;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_while_m_n, take_while1};
use nom::character::complete::{char, satisfy, space0, space1};
use nom::combinator::{opt, recognize, value};
use nom::error::Error;
use nom::multi::many0;
use nom::sequence::{delimited, preceded, terminated};

use super::PResult;
use crate::message::Params;

/// `alphanum = ALPHA / DIGIT`
pub fn is_alphanum(c: char) -> bool {
    c.is_ascii_alphanumeric()
}

/// `mark = "-" / "_" / "." / "!" / "~" / "*" / "'" / "(" / ")"`
pub fn is_mark(c: char) -> bool {
    "-_.!~*'()".contains(c)
}

/// `unreserved = alphanum / mark`
pub fn is_unreserved(c: char) -> bool {
    is_alphanum(c) || is_mark(c)
}

/// `reserved = ";" / "/" / "?" / ":" / "@" / "&" / "=" / "+" / "$" / ","`
pub fn is_reserved(c: char) -> bool {
    ";/?:@&=+$,".contains(c)
}

/// A character of `token`:
/// `alphanum / "-" / "." / "!" / "%" / "*" / "_" / "+" / "`" / "'" / "~"`
pub fn is_token_char(c: char) -> bool {
    is_alphanum(c) || "-.!%*_+`'~".contains(c)
}

/// A character of `word` (token characters plus `( ) < > : \ " / [ ] ? { }`).
pub fn is_word_char(c: char) -> bool {
    is_token_char(c) || "()<>:\\\"/[]?{}".contains(c)
}

/// `TEXT-UTF8char = %x21-7E / UTF8-NONASCII`
pub fn is_text_utf8_char(c: char) -> bool {
    c > ' ' && c != '\x7f'
}

/// `qdtext = LWS / %x21 / %x23-5B / %x5D-7E / UTF8-NONASCII`, minus the LWS.
pub fn is_qdtext(c: char) -> bool {
    c == ' ' || c == '\t' || (is_text_utf8_char(c) && c != '"' && c != '\\')
}

/// `CRLF`
pub fn crlf(i: &str) -> PResult<'_, &str> {
    tag("\r\n").parse(i)
}

/// `LWS = [*WSP CRLF] 1*WSP` (linear whitespace, including line folds)
pub fn lws(i: &str) -> PResult<'_, &str> {
    alt((recognize((space0, crlf, space1)), space1)).parse(i)
}

/// `SWS = [LWS]`
pub fn sws(i: &str) -> PResult<'_, &str> {
    recognize(opt(lws)).parse(i)
}

/// `SWS c SWS`, the shape of `SEMI`, `COMMA`, `EQUAL`, `SLASH`, `COLON`, `STAR`.
fn separator<'a>(c: char) -> impl Parser<&'a str, Output = char, Error = Error<&'a str>> {
    delimited(sws, char(c), sws)
}

/// `HCOLON = *( SP / HTAB ) ":" SWS`
pub fn hcolon(i: &str) -> PResult<'_, ()> {
    value((), (space0, char(':'), sws)).parse(i)
}

/// `SEMI = SWS ";" SWS`
pub fn semi(i: &str) -> PResult<'_, char> {
    separator(';').parse(i)
}

/// `COMMA = SWS "," SWS`
pub fn comma(i: &str) -> PResult<'_, char> {
    separator(',').parse(i)
}

/// `EQUAL = SWS "=" SWS`
pub fn equal(i: &str) -> PResult<'_, char> {
    separator('=').parse(i)
}

/// `SLASH = SWS "/" SWS`
pub fn slash(i: &str) -> PResult<'_, char> {
    separator('/').parse(i)
}

/// `COLON = SWS ":" SWS`
pub fn colon(i: &str) -> PResult<'_, char> {
    separator(':').parse(i)
}

/// `STAR = SWS "*" SWS`
pub fn star(i: &str) -> PResult<'_, char> {
    separator('*').parse(i)
}

/// `LAQUOT = SWS "<"`
pub fn laquot(i: &str) -> PResult<'_, char> {
    preceded(sws, char('<')).parse(i)
}

/// `RAQUOT = ">" SWS`
pub fn raquot(i: &str) -> PResult<'_, char> {
    terminated(char('>'), sws).parse(i)
}

/// `token = 1*(alphanum / "-" / "." / "!" / "%" / "*" / "_" / "+" / "`" / "'" / "~")`
pub fn token(i: &str) -> PResult<'_, &str> {
    take_while1(is_token_char).parse(i)
}

/// `word = 1*(alphanum / "-" / "." / "!" / "%" / "*" / "_" / "+" / "`" / "'" / "~" /
///  "(" / ")" / "<" / ">" / ":" / "\" / DQUOTE / "/" / "[" / "]" / "?" / "{" / "}")`
pub fn word(i: &str) -> PResult<'_, &str> {
    take_while1(is_word_char).parse(i)
}

/// `escaped = "%" HEXDIG HEXDIG`
pub fn escaped(i: &str) -> PResult<'_, &str> {
    recognize((
        char('%'),
        take_while_m_n(2, 2, |c: char| c.is_ascii_hexdigit()),
    ))
    .parse(i)
}

/// `quoted-pair = "\" (%x00-09 / %x0B-0C / %x0E-7F)`
pub fn quoted_pair(i: &str) -> PResult<'_, &str> {
    recognize((
        char('\\'),
        satisfy(|c| c.is_ascii() && c != '\n' && c != '\r'),
    ))
    .parse(i)
}

/// `quoted-string = SWS DQUOTE *(qdtext / quoted-pair ) DQUOTE`
///
/// Returns the string as written, quotes included; see [`unquote`].
pub fn quoted_string(i: &str) -> PResult<'_, &str> {
    preceded(
        sws,
        recognize((
            char('"'),
            many0(alt((take_while1(is_qdtext), quoted_pair, lws))),
            char('"'),
        )),
    )
    .parse(i)
}

/// The text of a [`quoted_string`] match, without quotes or escapes.
pub fn unquote(quoted: &str) -> String {
    let inner = &quoted[1..quoted.len() - 1];
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.extend(chars.next()),
            '\r' | '\n' => {}
            c => out.push(c),
        }
    }
    out
}

/// `gen-value = token / host / quoted-string`
pub fn gen_value(i: &str) -> PResult<'_, &str> {
    // hostname and IPv4address are made of token characters.
    alt((quoted_string, super::uri::ipv6_reference, token)).parse(i)
}

/// `generic-param = token [ EQUAL gen-value ]`
pub fn generic_param(i: &str) -> PResult<'_, (String, Option<String>)> {
    (token, opt(preceded(equal, gen_value)))
        .map(|(name, value)| (name.to_owned(), value.map(str::to_owned)))
        .parse(i)
}

/// `*( SEMI generic-param )`, the parameter tail of most headers.
pub fn header_params(i: &str) -> PResult<'_, Params> {
    many0(preceded(semi, generic_param))
        .map(Params::from_iter)
        .parse(i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::parse_all;

    #[test]
    fn lws_covers_spaces_and_line_folds() {
        assert_eq!(lws(" \t x"), Ok(("x", " \t ")));
        assert_eq!(lws("  \r\n\tx"), Ok(("x", "  \r\n\t")));
        // A CRLF not followed by whitespace ends the header instead.
        assert_eq!(lws("  \r\nx"), Ok(("\r\nx", "  ")));
        assert!(lws("\r\nx").is_err());
        assert_eq!(sws("x"), Ok(("x", "")));
    }

    #[test]
    fn separators_absorb_surrounding_whitespace() {
        assert_eq!(semi(" ; x"), Ok(("x", ';')));
        assert_eq!(hcolon(" \t: x"), Ok(("x", ())));
        assert!(hcolon("\r\n : x").is_err());
        assert_eq!(laquot("  <x"), Ok(("x", '<')));
        assert_eq!(raquot("> x"), Ok(("x", '>')));
    }

    #[test]
    fn quoted_string_keeps_raw_text() {
        let raw = r#""a \"b\" c""#;
        assert_eq!(quoted_string(&format!(" {raw};")), Ok((";", raw)));
        assert_eq!(unquote(raw), r#"a "b" c"#);
        assert!(quoted_string(r#""unterminated"#).is_err());
    }

    #[test]
    fn generic_param_values() {
        let p = |s| parse_all(generic_param, s);
        assert_eq!(p("lr"), Some(("lr".into(), None)));
        assert_eq!(p("q = 0.7"), Some(("q".into(), Some("0.7".into()))));
        assert_eq!(
            p("maddr=[::1]"),
            Some(("maddr".into(), Some("[::1]".into())))
        );
        assert_eq!(p(r#"x="a;b""#), Some(("x".into(), Some(r#""a;b""#.into()))));
        assert_eq!(p("=x"), None);
    }
}
