//! Typed views of the headers the transaction and dialog layers depend on.

use std::fmt;
use std::str::FromStr;

use super::header::names;
use super::{Method, Params, ParseError, Uri};
use crate::grammar::header::{address, cseq_value, via_parm};
use crate::grammar::parse_all;

/// `CSeq: 314159 INVITE`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSeq {
    pub seq: u32,
    pub method: Method,
}

impl FromStr for CSeq {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, ParseError> {
        parse_all(cseq_value, s.trim()).ok_or_else(|| ParseError::HeaderValue {
            name: names::CSEQ,
            value: s.to_owned(),
        })
    }
}

impl fmt::Display for CSeq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.seq, self.method)
    }
}

/// One Via value: `SIP/2.0/UDP host:port;branch=z9hG4bK...`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Via {
    /// Upper-cased transport, e.g. "UDP".
    pub transport: String,
    pub host: String,
    pub port: Option<u16>,
    pub params: Params,
}

impl Via {
    /// The magic cookie that marks an RFC 3261 branch (§8.1.1.7).
    pub const BRANCH_COOKIE: &str = "z9hG4bK";

    pub fn branch(&self) -> Option<&str> {
        self.params.get("branch")
    }
}

impl FromStr for Via {
    type Err = ParseError;

    /// Parses a single `via-parm`; see [`super::Headers::vias`] for lists.
    fn from_str(s: &str) -> Result<Self, ParseError> {
        parse_all(via_parm, s.trim()).ok_or_else(|| ParseError::HeaderValue {
            name: names::VIA,
            value: s.to_owned(),
        })
    }
}

impl fmt::Display for Via {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SIP/2.0/{} {}", self.transport, self.host)?;
        if let Some(port) = self.port {
            write!(f, ":{port}")?;
        }
        write!(f, "{}", self.params)
    }
}

/// An address as used by From, To and Contact:
/// `"Bob" <sip:bob@example.com>;tag=a6c85cf`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameAddr {
    pub display_name: Option<String>,
    pub uri: Uri,
    /// Header parameters, after the URI.
    pub params: Params,
}

impl NameAddr {
    pub fn new(uri: Uri) -> Self {
        Self {
            display_name: None,
            uri,
            params: Params::default(),
        }
    }

    pub fn tag(&self) -> Option<&str> {
        self.params.get("tag")
    }
}

impl FromStr for NameAddr {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, ParseError> {
        parse_all(address, s.trim()).ok_or_else(|| ParseError::HeaderValue {
            name: "address",
            value: s.to_owned(),
        })
    }
}

impl fmt::Display for NameAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = &self.display_name {
            let escaped = name.replace('\\', "\\\\").replace('"', "\\\"");
            write!(f, "\"{escaped}\" ")?;
        }
        write!(f, "<{}>{}", self.uri, self.params)
    }
}
