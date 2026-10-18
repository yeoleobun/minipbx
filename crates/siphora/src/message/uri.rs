use std::fmt;
use std::str::FromStr;

use super::{Params, ParseError};
use crate::grammar::parse_all;
use crate::grammar::uri::sip_uri;

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
        parse_all(sip_uri, s.trim()).ok_or_else(|| ParseError::Uri(s.to_owned()))
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
