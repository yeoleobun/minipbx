//! nom combinators for the RFC 3261 §25 ABNF grammar.
//!
//! Each public function implements the rule named in its doc comment, and
//! larger rules are built from smaller ones, as in the RFC:
//! - [`core`]: basic rules (§25.1): `token`, `LWS`, `SEMI`, `quoted-string`, ...
//! - [`uri`]: `SIP-URI` and its parts (§19.1, §25.1)
//! - [`header`]: typed header values such as `via-parm` and `name-addr`
//! - [`message`]: start lines and `message-header`
//!
//! Parsers take `&str`; the message parser checks the header section is
//! UTF-8 first, which RFC 3261 requires anyway.

pub mod core;
pub mod header;
pub mod message;
pub mod uri;

use nom::Parser;
use nom::combinator::all_consuming;

pub type PResult<'a, T> = nom::IResult<&'a str, T>;

/// Runs `parser` over all of `input`; `None` if it fails or input is left over.
pub fn parse_all<'a, T>(
    parser: impl Parser<&'a str, Output = T, Error = nom::error::Error<&'a str>>,
    input: &'a str,
) -> Option<T> {
    all_consuming(parser).parse(input).ok().map(|(_, v)| v)
}
