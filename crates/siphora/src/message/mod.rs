//! SIP message model, parser and serializer (RFC 3261 §7).

mod error;
mod header;
mod method;
mod params;
mod parse;
mod status;
mod text;
mod typed;
mod uri;

use std::fmt;

pub use error::ParseError;
pub use header::{Headers, names};
pub use method::Method;
pub use params::Params;
pub use status::StatusCode;
pub use typed::{CSeq, NameAddr, Via};
pub use uri::{Scheme, Uri};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: Method,
    pub uri: Uri,
    pub headers: Headers,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: StatusCode,
    pub reason: String,
    pub headers: Headers,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Request(Request),
    Response(Response),
}

impl Message {
    /// Parses one complete message, such as a UDP datagram.
    pub fn parse(buf: &[u8]) -> Result<Self, ParseError> {
        parse::parse_message(buf)
    }

    pub fn headers(&self) -> &Headers {
        match self {
            Message::Request(r) => &r.headers,
            Message::Response(r) => &r.headers,
        }
    }

    pub fn headers_mut(&mut self) -> &mut Headers {
        match self {
            Message::Request(r) => &mut r.headers,
            Message::Response(r) => &mut r.headers,
        }
    }

    pub fn body(&self) -> &[u8] {
        match self {
            Message::Request(r) => &r.body,
            Message::Response(r) => &r.body,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Message::Request(r) => r.to_bytes(),
            Message::Response(r) => r.to_bytes(),
        }
    }
}

impl Request {
    pub fn new(method: Method, uri: Uri) -> Self {
        Self {
            method,
            uri,
            headers: Headers::new(),
            body: Vec::new(),
        }
    }

    /// Sets the body and keeps Content-Length in step with it.
    pub fn set_body(&mut self, body: Vec<u8>) {
        self.headers
            .set(names::CONTENT_LENGTH, body.len().to_string());
        self.body = body;
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = format!(
            "{} {} SIP/2.0\r\n{}\r\n",
            self.method, self.uri, self.headers
        )
        .into_bytes();
        out.extend_from_slice(&self.body);
        out
    }
}

impl Response {
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            reason: status.canonical_reason().to_owned(),
            headers: Headers::new(),
            body: Vec::new(),
        }
    }

    /// Sets the body and keeps Content-Length in step with it.
    pub fn set_body(&mut self, body: Vec<u8>) {
        self.headers
            .set(names::CONTENT_LENGTH, body.len().to_string());
        self.body = body;
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = format!(
            "SIP/2.0 {} {}\r\n{}\r\n",
            self.status, self.reason, self.headers
        )
        .into_bytes();
        out.extend_from_slice(&self.body);
        out
    }
}

impl From<Request> for Message {
    fn from(r: Request) -> Self {
        Message::Request(r)
    }
}

impl From<Response> for Message {
    fn from(r: Response) -> Self {
        Message::Response(r)
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.to_bytes()))
    }
}
