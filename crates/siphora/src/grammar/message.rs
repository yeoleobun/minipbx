//! Start lines and header fields (RFC 3261 §25.1, "SIP-message").

use nom::Parser;
use nom::branch::alt;
use nom::bytes::complete::{tag_no_case, take_while, take_while_m_n, take_while1};
use nom::character::complete::{char, digit1};
use nom::combinator::{map_opt, map_res, recognize};
use nom::multi::many0;
use nom::sequence::preceded;

use super::PResult;
use super::core::{crlf, hcolon, is_reserved, is_text_utf8_char, is_unreserved, lws, token};
use super::uri::sip_uri;
use crate::message::{Method, StatusCode, Uri};

/// `Method = INVITEm / ACKm / OPTIONSm / BYEm / CANCELm / REGISTERm /
/// extension-method`, where `extension-method = token`. Case-sensitive.
pub fn method(i: &str) -> PResult<'_, Method> {
    map_res(token, str::parse).parse(i)
}

/// `SIP-Version = "SIP" "/" 1*DIGIT "." 1*DIGIT`, returning `major.minor`.
pub fn sip_version(i: &str) -> PResult<'_, &str> {
    preceded(tag_no_case("SIP/"), recognize((digit1, char('.'), digit1))).parse(i)
}

/// `Request-Line = Method SP Request-URI SP SIP-Version CRLF`
///
/// `Request-URI = SIP-URI / SIPS-URI / absoluteURI`; absoluteURI (tel: and
/// the like) is not supported yet.
pub fn request_line(i: &str) -> PResult<'_, (Method, Uri, &str)> {
    (method, char(' '), sip_uri, char(' '), sip_version, crlf)
        .map(|(method, _, uri, _, version, _)| (method, uri, version))
        .parse(i)
}

/// `Status-Code = 3DIGIT`, in the range 100..=699.
pub fn status_code(i: &str) -> PResult<'_, StatusCode> {
    map_opt(
        take_while_m_n(3, 3, |c: char| c.is_ascii_digit()),
        |s: &str| s.parse().ok().and_then(StatusCode::new),
    )
    .parse(i)
}

/// `Reason-Phrase = *(reserved / unreserved / escaped / UTF8-NONASCII /
/// UTF8-CONT / SP / HTAB)`; `escaped` is accepted as its characters.
pub fn reason_phrase(i: &str) -> PResult<'_, &str> {
    take_while(|c: char| is_reserved(c) || is_unreserved(c) || !c.is_ascii() || " \t%".contains(c))
        .parse(i)
}

/// `Status-Line = SIP-Version SP Status-Code SP Reason-Phrase CRLF`
pub fn status_line(i: &str) -> PResult<'_, (&str, StatusCode, &str)> {
    (
        sip_version,
        char(' '),
        status_code,
        char(' '),
        reason_phrase,
        crlf,
    )
        .map(|(version, _, code, _, reason, _)| (version, code, reason))
        .parse(i)
}

/// `header-value = *(TEXT-UTF8char / UTF8-CONT / LWS)`, as written.
pub fn header_value(i: &str) -> PResult<'_, &str> {
    recognize(many0(alt((take_while1(is_text_utf8_char), lws)))).parse(i)
}

/// `message-header = header-name HCOLON header-value CRLF`, where
/// `header-name = token`.
///
/// The value comes back unfolded: each line fold becomes one space, and
/// leading and trailing whitespace is dropped.
pub fn message_header(i: &str) -> PResult<'_, (&str, String)> {
    (token, hcolon, header_value, crlf)
        .map(|(name, _, value, _)| (name, unfold(value)))
        .parse(i)
}

fn unfold(value: &str) -> String {
    value
        .split("\r\n")
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned()
}
