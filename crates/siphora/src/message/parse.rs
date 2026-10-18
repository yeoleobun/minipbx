//! Parses a complete message, as received in one UDP datagram.
//!
//! `SIP-message = Request / Response`, where
//! `Request = Request-Line *( message-header ) CRLF [ message-body ]` and
//! `Response = Status-Line *( message-header ) CRLF [ message-body ]`.

use super::header::Headers;
use super::{Message, ParseError, Request, Response};
use crate::grammar::core::crlf;
use crate::grammar::message::{message_header, request_line, status_line};

pub(super) fn parse_message(buf: &[u8]) -> Result<Message, ParseError> {
    // CRLFs before the start line are ignored (RFC 3261 §7.5).
    let mut buf = buf;
    while let Some(rest) = buf.strip_prefix(b"\r\n") {
        buf = rest;
    }

    let head_len = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or(ParseError::Incomplete)?
        + 4;
    let head = std::str::from_utf8(&buf[..head_len]).map_err(|_| ParseError::Utf8)?;
    let rest = &buf[head_len..];

    let start_error = || ParseError::StartLine(first_line(head).to_owned());
    let (mut input, start) = if head.starts_with("SIP/") {
        let (input, (version, status, reason)) = status_line(head).map_err(|_| start_error())?;
        (input, StartLine::Status(version, status, reason))
    } else {
        let (input, (method, uri, version)) = request_line(head).map_err(|_| start_error())?;
        (input, StartLine::Request(version, method, uri))
    };
    if start.version() != "2.0" {
        return Err(start_error());
    }

    let mut headers = Headers::new();
    while crlf(input).is_err() {
        let (next, (name, value)) = message_header(input)
            .map_err(|_| ParseError::HeaderLine(first_line(input).to_owned()))?;
        headers.push(name, value);
        input = next;
    }

    let body = match headers.content_length()? {
        Some(len) if rest.len() < len => {
            return Err(ParseError::BodyLength {
                expected: len,
                actual: rest.len(),
            });
        }
        // Bytes after Content-Length in a datagram are discarded (§18.3).
        Some(len) => rest[..len].to_vec(),
        None => rest.to_vec(),
    };

    Ok(match start {
        StartLine::Request(_, method, uri) => Message::Request(Request {
            method,
            uri,
            headers,
            body,
        }),
        StartLine::Status(_, status, reason) => Message::Response(Response {
            status,
            reason: reason.to_owned(),
            headers,
            body,
        }),
    })
}

enum StartLine<'a> {
    Request(&'a str, super::Method, super::Uri),
    Status(&'a str, super::StatusCode, &'a str),
}

impl StartLine<'_> {
    fn version(&self) -> &str {
        match self {
            StartLine::Request(v, ..) | StartLine::Status(v, ..) => v,
        }
    }
}

fn first_line(s: &str) -> &str {
    s.split("\r\n").next().unwrap_or(s)
}
