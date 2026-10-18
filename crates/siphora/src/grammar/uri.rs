//! `SIP-URI` and `SIPS-URI` (RFC 3261 §19.1, §25.1).

use nom::Parser;
use nom::branch::alt;
use nom::bytes::complete::{tag_no_case, take_while1};
use nom::character::complete::{char, digit1};
use nom::combinator::{map_res, opt, recognize, value};
use nom::multi::{many0, many1};
use nom::sequence::{delimited, preceded, terminated};

use super::PResult;
use super::core::{escaped, is_alphanum, is_unreserved};
use crate::message::{Params, Scheme, Uri};

/// `"sip:"` or `"sips:"`; ABNF string literals are case-insensitive.
pub fn scheme(i: &str) -> PResult<'_, Scheme> {
    alt((
        value(Scheme::Sips, tag_no_case("sips:")),
        value(Scheme::Sip, tag_no_case("sip:")),
    ))
    .parse(i)
}

/// `user = 1*( unreserved / escaped / user-unreserved )`, where
/// `user-unreserved = "&" / "=" / "+" / "$" / "," / ";" / "?" / "/"`
pub fn user(i: &str) -> PResult<'_, &str> {
    recognize(many1(alt((
        take_while1(|c| is_unreserved(c) || "&=+$,;?/".contains(c)),
        escaped,
    ))))
    .parse(i)
}

/// `password = *( unreserved / escaped / "&" / "=" / "+" / "$" / "," )`
pub fn password(i: &str) -> PResult<'_, &str> {
    recognize(many0(alt((
        take_while1(|c| is_unreserved(c) || "&=+$,".contains(c)),
        escaped,
    ))))
    .parse(i)
}

/// `userinfo = ( user / telephone-subscriber ) [ ":" password ] "@"`
///
/// `telephone-subscriber` is not parsed separately: its characters are a
/// subset of `user` characters in practice.
pub fn userinfo(i: &str) -> PResult<'_, (&str, Option<&str>)> {
    terminated((user, opt(preceded(char(':'), password))), char('@')).parse(i)
}

/// `hostname = *( domainlabel "." ) toplabel [ "." ]`
///
/// Matched as `1*( alphanum / "-" / "." )`, which also covers `IPv4address`;
/// the placement of '-' and '.' within labels is not checked.
pub fn hostname(i: &str) -> PResult<'_, &str> {
    take_while1(|c| is_alphanum(c) || c == '-' || c == '.').parse(i)
}

/// `IPv6address`, matched as `1*( HEXDIG / ":" / "." )`.
pub fn ipv6_address(i: &str) -> PResult<'_, &str> {
    take_while1(|c: char| c.is_ascii_hexdigit() || c == ':' || c == '.').parse(i)
}

/// `IPv6reference = "[" IPv6address "]"`
pub fn ipv6_reference(i: &str) -> PResult<'_, &str> {
    recognize(delimited(char('['), ipv6_address, char(']'))).parse(i)
}

/// `host = hostname / IPv4address / IPv6reference`
pub fn host(i: &str) -> PResult<'_, &str> {
    alt((ipv6_reference, hostname)).parse(i)
}

/// `port = 1*DIGIT`, limited to what fits in a u16.
pub fn port(i: &str) -> PResult<'_, u16> {
    map_res(digit1, str::parse).parse(i)
}

/// `hostport = host [ ":" port ]`
pub fn hostport(i: &str) -> PResult<'_, (&str, Option<u16>)> {
    (host, opt(preceded(char(':'), port))).parse(i)
}

/// `paramchar = param-unreserved / unreserved / escaped`, one or more, where
/// `param-unreserved = "[" / "]" / "/" / ":" / "&" / "+" / "$"`
fn paramchars(i: &str) -> PResult<'_, &str> {
    recognize(many1(alt((
        take_while1(|c| is_unreserved(c) || "[]/:&+$".contains(c)),
        escaped,
    ))))
    .parse(i)
}

/// `uri-parameter = pname [ "=" pvalue ]`, with `pname = 1*paramchar` and
/// `pvalue = 1*paramchar`. Covers `transport-param`, `lr-param` and the
/// other named parameters, which all have this shape.
pub fn uri_parameter(i: &str) -> PResult<'_, (String, Option<String>)> {
    (paramchars, opt(preceded(char('='), paramchars)))
        .map(|(name, value)| (name.to_owned(), value.map(str::to_owned)))
        .parse(i)
}

/// `uri-parameters = *( ";" uri-parameter )`
pub fn uri_parameters(i: &str) -> PResult<'_, Params> {
    many0(preceded(char(';'), uri_parameter))
        .map(Params::from_iter)
        .parse(i)
}

/// One run of `hnv-unreserved / unreserved / escaped`, where
/// `hnv-unreserved = "[" / "]" / "/" / "?" / ":" / "+" / "$"`
fn hnv_run(i: &str) -> PResult<'_, &str> {
    alt((
        take_while1(|c| is_unreserved(c) || "[]/?:+$".contains(c)),
        escaped,
    ))
    .parse(i)
}

/// `header = hname "=" hvalue`, with `hname = 1*( hnv-unreserved /
/// unreserved / escaped )` and `hvalue = *( hnv-unreserved / unreserved /
/// escaped )`
fn uri_header(i: &str) -> PResult<'_, &str> {
    recognize((many1(hnv_run), char('='), many0(hnv_run))).parse(i)
}

/// `headers = "?" header *( "&" header )`, returned without the "?".
pub fn uri_headers(i: &str) -> PResult<'_, &str> {
    preceded(
        char('?'),
        recognize((uri_header, many0((char('&'), uri_header)))),
    )
    .parse(i)
}

/// `SIP-URI = "sip:" [ userinfo ] hostport uri-parameters [ headers ]`, and
/// `SIPS-URI` likewise.
pub fn sip_uri(i: &str) -> PResult<'_, Uri> {
    (
        scheme,
        opt(userinfo),
        hostport,
        uri_parameters,
        opt(uri_headers),
    )
        .map(|(scheme, userinfo, (host, port), params, headers)| Uri {
            scheme,
            user: userinfo.map(|(u, _)| u.to_owned()),
            password: userinfo.and_then(|(_, p)| p.map(str::to_owned)),
            host: host.to_owned(),
            port,
            params,
            headers: headers.map(str::to_owned),
        })
        .parse(i)
}

/// A SIP URI with no parameters or headers. This is what `addr-spec` can
/// hold in From, To and Contact when there are no angle brackets: anything
/// after ';' is then a header parameter (RFC 3261 §20.10).
pub fn sip_uri_bare(i: &str) -> PResult<'_, Uri> {
    (scheme, opt(userinfo), hostport)
        .map(|(scheme, userinfo, (host, port))| Uri {
            scheme,
            user: userinfo.map(|(u, _)| u.to_owned()),
            password: userinfo.and_then(|(_, p)| p.map(str::to_owned)),
            host: host.to_owned(),
            port,
            params: Params::default(),
            headers: None,
        })
        .parse(i)
}
