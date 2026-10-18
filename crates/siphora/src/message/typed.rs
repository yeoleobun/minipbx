//! Typed views of the headers the transaction and dialog layers depend on.

use std::fmt;
use std::str::FromStr;

use super::header::names;
use super::text::{find_unquoted, unquote};
use super::uri::parse_hostport;
use super::{Method, Params, ParseError, Uri};

/// `CSeq: 314159 INVITE`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSeq {
    pub seq: u32,
    pub method: Method,
}

impl FromStr for CSeq {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, ParseError> {
        let err = || ParseError::HeaderValue {
            name: names::CSEQ,
            value: s.to_owned(),
        };
        let mut parts = s.split_whitespace();
        let seq = parts.next().and_then(|n| n.parse().ok()).ok_or_else(err)?;
        let method = parts.next().and_then(|m| m.parse().ok()).ok_or_else(err)?;
        if parts.next().is_some() {
            return Err(err());
        }
        Ok(Self { seq, method })
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

    fn from_str(s: &str) -> Result<Self, ParseError> {
        let err = || ParseError::HeaderValue {
            name: names::VIA,
            value: s.to_owned(),
        };
        // LWS is allowed around each '/' of the sent-protocol.
        let (name, rest) = s.split_once('/').ok_or_else(err)?;
        let (version, rest) = rest.split_once('/').ok_or_else(err)?;
        if !name.trim().eq_ignore_ascii_case("SIP") || version.trim() != "2.0" {
            return Err(err());
        }
        let rest = rest.trim_start();
        let end = rest.find(char::is_whitespace).ok_or_else(err)?;
        let transport = rest[..end].to_ascii_uppercase();
        let (sent_by, params) = match rest[end..].split_once(';') {
            Some((sb, p)) => (sb, Params::parse(p)),
            None => (&rest[end..], Params::default()),
        };
        let (host, port) = parse_hostport(sent_by).ok_or_else(err)?;
        Ok(Self {
            transport,
            host,
            port,
            params,
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
        let err = || ParseError::HeaderValue {
            name: "address",
            value: s.to_owned(),
        };
        let s = s.trim();
        match find_unquoted(s, b'<') {
            Some(open) => {
                let close = open + s[open..].find('>').ok_or_else(err)?;
                let display = s[..open].trim();
                let display_name = match display {
                    "" => None,
                    d if d.starts_with('"') => Some(unquote(d).ok_or_else(err)?),
                    d => Some(d.to_owned()),
                };
                let uri = s[open + 1..close].parse()?;
                let params = match s[close + 1..].trim().strip_prefix(';') {
                    Some(p) => Params::parse(p),
                    None if s[close + 1..].trim().is_empty() => Params::default(),
                    None => return Err(err()),
                };
                Ok(Self {
                    display_name,
                    uri,
                    params,
                })
            }
            // Without angle brackets, everything after ';' is a header
            // parameter, not a URI parameter (RFC 3261 §20.10).
            None => {
                let (uri, params) = match s.split_once(';') {
                    Some((u, p)) => (u, Params::parse(p)),
                    None => (s, Params::default()),
                };
                Ok(Self {
                    display_name: None,
                    uri: uri.parse()?,
                    params,
                })
            }
        }
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
