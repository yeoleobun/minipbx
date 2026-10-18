use std::fmt;
use std::str::FromStr;

use crate::grammar::core::is_token_char;

/// A request method. Methods are case-sensitive (RFC 3261 §7.1).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Method {
    Invite,
    Ack,
    Bye,
    Cancel,
    Register,
    Options,
    Info,
    Update,
    Prack,
    Subscribe,
    Notify,
    Refer,
    Message,
    Publish,
    Other(String),
}

impl Method {
    pub fn as_str(&self) -> &str {
        match self {
            Method::Invite => "INVITE",
            Method::Ack => "ACK",
            Method::Bye => "BYE",
            Method::Cancel => "CANCEL",
            Method::Register => "REGISTER",
            Method::Options => "OPTIONS",
            Method::Info => "INFO",
            Method::Update => "UPDATE",
            Method::Prack => "PRACK",
            Method::Subscribe => "SUBSCRIBE",
            Method::Notify => "NOTIFY",
            Method::Refer => "REFER",
            Method::Message => "MESSAGE",
            Method::Publish => "PUBLISH",
            Method::Other(s) => s,
        }
    }
}

impl FromStr for Method {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        Ok(match s {
            "INVITE" => Method::Invite,
            "ACK" => Method::Ack,
            "BYE" => Method::Bye,
            "CANCEL" => Method::Cancel,
            "REGISTER" => Method::Register,
            "OPTIONS" => Method::Options,
            "INFO" => Method::Info,
            "UPDATE" => Method::Update,
            "PRACK" => Method::Prack,
            "SUBSCRIBE" => Method::Subscribe,
            "NOTIFY" => Method::Notify,
            "REFER" => Method::Refer,
            "MESSAGE" => Method::Message,
            "PUBLISH" => Method::Publish,
            _ if !s.is_empty() && s.chars().all(is_token_char) => Method::Other(s.to_owned()),
            _ => return Err(()),
        })
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
