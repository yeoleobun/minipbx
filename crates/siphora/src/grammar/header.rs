//! Typed header values (RFC 3261 §25.1, "message-header" alternatives).
//!
//! These parse the value after `HCOLON`, so they compose with
//! [`super::message::message_header`] or run on a stored value.

use nom::Parser;
use nom::branch::alt;
use nom::bytes::complete::tag_no_case;
use nom::character::complete::digit1;
use nom::combinator::{map_res, opt, recognize, verify};
use nom::multi::{many0, separated_list1};
use nom::sequence::{preceded, terminated};

use super::PResult;
use super::core::{
    colon, comma, equal, generic_param, header_params, laquot, lws, quoted_string, raquot, semi,
    slash, star, token, unquote,
};
use super::message::method;
use super::uri::{host, ipv6_address, port, sip_uri, sip_uri_bare};
use crate::message::{CSeq, NameAddr, Params, Uri, Via};

/// `display-name = *(token LWS) / quoted-string`
pub fn display_name(i: &str) -> PResult<'_, Option<String>> {
    alt((
        quoted_string.map(|q| Some(unquote(q))),
        many0(terminated(token, lws)).map(|t| (!t.is_empty()).then(|| t.join(" "))),
    ))
    .parse(i)
}

/// `name-addr = [ display-name ] LAQUOT addr-spec RAQUOT`
pub fn name_addr(i: &str) -> PResult<'_, (Option<String>, Uri)> {
    (display_name, laquot, sip_uri, raquot)
        .map(|(name, _, uri, _)| (name, uri))
        .parse(i)
}

/// `( name-addr / addr-spec ) *( SEMI generic-param )`
///
/// The common shape of `from-spec`, the To value and `contact-param`; their
/// specific parameters (`tag-param`, `c-p-q`, `c-p-expires`) are all
/// `generic-param`s syntactically.
pub fn address(i: &str) -> PResult<'_, NameAddr> {
    (
        alt((name_addr, sip_uri_bare.map(|uri| (None, uri)))),
        header_params,
    )
        .map(|((display_name, uri), params)| NameAddr {
            display_name,
            uri,
            params,
        })
        .parse(i)
}

/// The Contact value: `( STAR / (contact-param *(COMMA contact-param)))`.
/// `STAR` yields an empty list.
pub fn contact_value(i: &str) -> PResult<'_, Vec<NameAddr>> {
    alt((separated_list1(comma, address), star.map(|_| Vec::new()))).parse(i)
}

/// `sent-protocol = protocol-name SLASH protocol-version SLASH transport`
pub fn sent_protocol(i: &str) -> PResult<'_, (&str, &str, &str)> {
    (token, slash, token, slash, token)
        .map(|(name, _, version, _, transport)| (name, version, transport))
        .parse(i)
}

/// `sent-by = host [ COLON port ]`
pub fn sent_by(i: &str) -> PResult<'_, (&str, Option<u16>)> {
    (host, opt(preceded(colon, port))).parse(i)
}

/// `via-received = "received" EQUAL (IPv4address / IPv6address)`
///
/// Listed separately because an unbracketed IPv6 address is not a
/// `gen-value`.
fn via_received(i: &str) -> PResult<'_, (String, Option<String>)> {
    (recognize(tag_no_case("received")), equal, ipv6_address)
        .map(|(name, _, addr)| (name.to_owned(), Some(addr.to_owned())))
        .parse(i)
}

/// `*( SEMI via-params )`, where `via-params = via-ttl / via-maddr /
/// via-received / via-branch / via-extension`; all but `via-received` are
/// `generic-param`s syntactically.
pub fn via_params(i: &str) -> PResult<'_, Params> {
    many0(preceded(semi, alt((via_received, generic_param))))
        .map(Params::from_iter)
        .parse(i)
}

/// `via-parm = sent-protocol LWS sent-by *( SEMI via-params )`, accepting
/// only `SIP/2.0`.
pub fn via_parm(i: &str) -> PResult<'_, Via> {
    (
        verify(sent_protocol, |(name, version, _)| {
            name.eq_ignore_ascii_case("SIP") && *version == "2.0"
        }),
        lws,
        sent_by,
        via_params,
    )
        .map(|((_, _, transport), _, (host, port), params)| Via {
            transport: transport.to_ascii_uppercase(),
            host: host.to_owned(),
            port,
            params,
        })
        .parse(i)
}

/// The Via value: `via-parm *(COMMA via-parm)`
pub fn via_value(i: &str) -> PResult<'_, Vec<Via>> {
    separated_list1(comma, via_parm).parse(i)
}

/// The CSeq value: `1*DIGIT LWS Method`
pub fn cseq_value(i: &str) -> PResult<'_, CSeq> {
    (map_res(digit1, str::parse), lws, method)
        .map(|(seq, _, method)| CSeq { seq, method })
        .parse(i)
}

/// The Content-Length value: `1*DIGIT`
pub fn content_length_value(i: &str) -> PResult<'_, usize> {
    map_res(digit1, str::parse).parse(i)
}
