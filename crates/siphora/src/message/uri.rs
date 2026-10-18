use std::fmt;
use std::str::FromStr;

use super::{Params, ParseError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scheme {
    Sip,
    Sips,
}

/// A SIP or SIPS URI (RFC 3261 §19.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uri {
    pub scheme: Scheme,
    pub user: Option<String>,
    pub password: Option<String>,
    /// Hostname or IP address; IPv6 references keep their brackets.
    pub host: String,
    pub port: Option<u16>,
    pub params: Params,
    /// Raw `?name=value&...` part, without the `?`.
    pub headers: Option<String>,
}

impl Uri {
    pub fn new(host: impl Into<String>) -> Self {
        Self {
            scheme: Scheme::Sip,
            user: None,
            password: None,
            host: host.into(),
            port: None,
            params: Params::default(),
            headers: None,
        }
    }
}

impl FromStr for Uri {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, ParseError> {
        let err = || ParseError::Uri(s.to_owned());
        let s = s.trim();
        let (scheme, rest) = s.split_once(':').ok_or_else(err)?;
        let scheme = if scheme.eq_ignore_ascii_case("sip") {
            Scheme::Sip
        } else if scheme.eq_ignore_ascii_case("sips") {
            Scheme::Sips
        } else {
            return Err(err());
        };

        let (rest, headers) = match rest.split_once('?') {
            Some((r, h)) => (r, Some(h.to_owned())),
            None => (rest, None),
        };

        // The user part may itself contain ';', so split on the last '@'.
        let (userinfo, hostpart) = match rest.rfind('@') {
            Some(i) => (Some(&rest[..i]), &rest[i + 1..]),
            None => (None, rest),
        };
        let (user, password) = match userinfo {
            Some("") => return Err(err()),
            Some(u) => match u.split_once(':') {
                Some((u, p)) => (Some(u.to_owned()), Some(p.to_owned())),
                None => (Some(u.to_owned()), None),
            },
            None => (None, None),
        };

        let (hostport, params) = match hostpart.split_once(';') {
            Some((hp, p)) => (hp, Params::parse(p)),
            None => (hostpart, Params::default()),
        };
        let (host, port) = parse_hostport(hostport).ok_or_else(err)?;

        Ok(Self {
            scheme,
            user,
            password,
            host,
            port,
            params,
            headers,
        })
    }
}

impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.scheme {
            Scheme::Sip => "sip:",
            Scheme::Sips => "sips:",
        })?;
        if let Some(user) = &self.user {
            f.write_str(user)?;
            if let Some(password) = &self.password {
                write!(f, ":{password}")?;
            }
            f.write_str("@")?;
        }
        f.write_str(&self.host)?;
        if let Some(port) = self.port {
            write!(f, ":{port}")?;
        }
        write!(f, "{}", self.params)?;
        if let Some(headers) = &self.headers {
            write!(f, "?{headers}")?;
        }
        Ok(())
    }
}

/// Parses `host[:port]`, where host may be a bracketed IPv6 reference.
pub(crate) fn parse_hostport(s: &str) -> Option<(String, Option<u16>)> {
    let s = s.trim();
    let (host, port) = if s.starts_with('[') {
        let end = s.find(']')?;
        let port = match &s[end + 1..] {
            "" => None,
            p => Some(p.strip_prefix(':')?),
        };
        (&s[..=end], port)
    } else {
        match s.split_once(':') {
            Some((h, p)) => (h, Some(p)),
            None => (s, None),
        }
    };
    if host.is_empty() || host.contains(char::is_whitespace) {
        return None;
    }
    let port = match port {
        Some(p) => Some(p.trim().parse().ok()?),
        None => None,
    };
    Some((host.to_owned(), port))
}
