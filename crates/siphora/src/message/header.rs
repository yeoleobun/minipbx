use std::fmt;

use super::ParseError;
use super::text::split_unquoted;
use super::typed::{CSeq, NameAddr, Via};

/// Header names, in their canonical spelling.
pub mod names {
    pub const ACCEPT: &str = "Accept";
    pub const ALLOW: &str = "Allow";
    pub const AUTHORIZATION: &str = "Authorization";
    pub const CALL_ID: &str = "Call-ID";
    pub const CONTACT: &str = "Contact";
    pub const CONTENT_LENGTH: &str = "Content-Length";
    pub const CONTENT_TYPE: &str = "Content-Type";
    pub const CSEQ: &str = "CSeq";
    pub const EXPIRES: &str = "Expires";
    pub const FROM: &str = "From";
    pub const MAX_FORWARDS: &str = "Max-Forwards";
    pub const PROXY_AUTHENTICATE: &str = "Proxy-Authenticate";
    pub const PROXY_AUTHORIZATION: &str = "Proxy-Authorization";
    pub const RECORD_ROUTE: &str = "Record-Route";
    pub const ROUTE: &str = "Route";
    pub const SERVER: &str = "Server";
    pub const SUPPORTED: &str = "Supported";
    pub const TO: &str = "To";
    pub const USER_AGENT: &str = "User-Agent";
    pub const VIA: &str = "Via";
    pub const WWW_AUTHENTICATE: &str = "WWW-Authenticate";
}

/// Compact forms (RFC 3261 §7.3.3 and later extensions).
const COMPACT: &[(&str, &str)] = &[
    ("a", "Accept-Contact"),
    ("b", "Referred-By"),
    ("c", "Content-Type"),
    ("d", "Request-Disposition"),
    ("e", "Content-Encoding"),
    ("f", "From"),
    ("i", "Call-ID"),
    ("j", "Reject-Contact"),
    ("k", "Supported"),
    ("l", "Content-Length"),
    ("m", "Contact"),
    ("o", "Event"),
    ("r", "Refer-To"),
    ("s", "Subject"),
    ("t", "To"),
    ("u", "Allow-Events"),
    ("v", "Via"),
    ("x", "Session-Expires"),
];

const KNOWN: &[&str] = &[
    "Accept",
    "Accept-Encoding",
    "Accept-Language",
    "Alert-Info",
    "Allow",
    "Allow-Events",
    "Authorization",
    "Call-ID",
    "Call-Info",
    "Contact",
    "Content-Disposition",
    "Content-Encoding",
    "Content-Length",
    "Content-Type",
    "CSeq",
    "Date",
    "Event",
    "Expires",
    "From",
    "Max-Forwards",
    "Min-Expires",
    "Min-SE",
    "Organization",
    "P-Asserted-Identity",
    "Priority",
    "Proxy-Authenticate",
    "Proxy-Authorization",
    "Proxy-Require",
    "RAck",
    "Reason",
    "Record-Route",
    "Refer-To",
    "Referred-By",
    "Require",
    "Retry-After",
    "Route",
    "RSeq",
    "Server",
    "Session-Expires",
    "Subject",
    "Subscription-State",
    "Supported",
    "Timestamp",
    "To",
    "Unsupported",
    "User-Agent",
    "Via",
    "Warning",
    "WWW-Authenticate",
];

/// Expands compact forms and fixes the casing of well-known names.
pub(crate) fn canonical_name(name: &str) -> String {
    if name.len() == 1 {
        if let Some((_, full)) = COMPACT.iter().find(|(c, _)| c.eq_ignore_ascii_case(name)) {
            return (*full).to_owned();
        }
    }
    match KNOWN.iter().find(|k| k.eq_ignore_ascii_case(name)) {
        Some(k) => (*k).to_owned(),
        None => name.to_owned(),
    }
}

/// Header fields in wire order. Values are kept raw and parsed into typed
/// headers only when asked for, so a proxy or archiver pays for what it reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Headers(Vec<(String, String)>);

impl Headers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, name: &str, value: impl Into<String>) {
        self.0.push((canonical_name(name), value.into()));
    }

    /// Replaces every field named `name` with a single one.
    pub fn set(&mut self, name: &str, value: impl Into<String>) {
        let name = canonical_name(name);
        let mut value = Some(value.into());
        self.0.retain_mut(|(k, v)| {
            if !k.eq_ignore_ascii_case(&name) {
                return true;
            }
            match value.take() {
                Some(new) => {
                    *v = new;
                    true
                }
                None => false,
            }
        });
        if let Some(value) = value {
            self.0.push((name, value));
        }
    }

    pub fn remove(&mut self, name: &str) {
        let name = canonical_name(name);
        self.0.retain(|(k, _)| !k.eq_ignore_ascii_case(&name));
    }

    /// The first field named `name`.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.get_all(name).next()
    }

    /// Every field named `name`, one item per header line.
    pub fn get_all(&self, name: &str) -> impl Iterator<Item = &str> {
        let name = canonical_name(name);
        self.0
            .iter()
            .filter(move |(k, _)| k.eq_ignore_ascii_case(&name))
            .map(|(_, v)| v.as_str())
    }

    /// Every comma-separated value of a list header such as Via or Contact,
    /// across all its header lines. Not for headers whose values can contain
    /// bare commas, such as Date or WWW-Authenticate.
    pub fn values(&self, name: &str) -> impl Iterator<Item = &str> {
        self.get_all(name)
            .flat_map(|v| split_unquoted(v, b','))
            .map(str::trim)
            .filter(|v| !v.is_empty())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn call_id(&self) -> Result<&str, ParseError> {
        self.get(names::CALL_ID)
            .ok_or(ParseError::MissingHeader(names::CALL_ID))
    }

    pub fn cseq(&self) -> Result<CSeq, ParseError> {
        self.get(names::CSEQ)
            .ok_or(ParseError::MissingHeader(names::CSEQ))?
            .parse()
    }

    pub fn vias(&self) -> Result<Vec<Via>, ParseError> {
        self.values(names::VIA).map(str::parse).collect()
    }

    pub fn top_via(&self) -> Result<Via, ParseError> {
        self.values(names::VIA)
            .next()
            .ok_or(ParseError::MissingHeader(names::VIA))?
            .parse()
    }

    pub fn from(&self) -> Result<NameAddr, ParseError> {
        self.get(names::FROM)
            .ok_or(ParseError::MissingHeader(names::FROM))?
            .parse()
    }

    pub fn to(&self) -> Result<NameAddr, ParseError> {
        self.get(names::TO)
            .ok_or(ParseError::MissingHeader(names::TO))?
            .parse()
    }

    /// Contact addresses; empty for `Contact: *`.
    pub fn contacts(&self) -> Result<Vec<NameAddr>, ParseError> {
        self.values(names::CONTACT)
            .filter(|v| *v != "*")
            .map(str::parse)
            .collect()
    }

    pub fn content_length(&self) -> Result<Option<usize>, ParseError> {
        self.get(names::CONTENT_LENGTH)
            .map(|v| {
                v.trim().parse().map_err(|_| ParseError::HeaderValue {
                    name: names::CONTENT_LENGTH,
                    value: v.to_owned(),
                })
            })
            .transpose()
    }
}

impl fmt::Display for Headers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (k, v) in &self.0 {
            write!(f, "{k}: {v}\r\n")?;
        }
        Ok(())
    }
}
