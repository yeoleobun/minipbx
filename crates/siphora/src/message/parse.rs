//! Parses a complete message, as received in one UDP datagram.

use super::header::Headers;
use super::{Message, ParseError, Request, Response, StatusCode};

const VERSION: &str = "SIP/2.0";

pub(super) fn parse_message(buf: &[u8]) -> Result<Message, ParseError> {
    // Empty lines before the start line are ignored (RFC 3261 §7.5).
    let start = buf
        .iter()
        .position(|b| *b != b'\r' && *b != b'\n')
        .ok_or(ParseError::Incomplete)?;
    let buf = &buf[start..];

    let (head_len, body_start) = find_head_end(buf).ok_or(ParseError::Incomplete)?;
    let head = std::str::from_utf8(&buf[..head_len]).map_err(|_| ParseError::Utf8)?;
    let mut lines = head.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l));

    let start_line = lines.next().ok_or(ParseError::Incomplete)?;
    let headers = parse_headers(lines)?;

    let rest = &buf[body_start..];
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

    if let Some(status_line) = start_line.strip_prefix("SIP/") {
        let mut parts = status_line.splitn(3, ' ');
        let (version, code, reason) = (parts.next(), parts.next(), parts.next());
        let status = code
            .and_then(|c| c.parse().ok())
            .and_then(StatusCode::new)
            .filter(|_| version == Some("2.0"))
            .ok_or_else(|| ParseError::StartLine(start_line.to_owned()))?;
        Ok(Message::Response(Response {
            status,
            reason: reason.unwrap_or("").to_owned(),
            headers,
            body,
        }))
    } else {
        let err = || ParseError::StartLine(start_line.to_owned());
        let mut parts = start_line.split(' ');
        let method = parts.next().and_then(|m| m.parse().ok()).ok_or_else(err)?;
        let uri = parts
            .next()
            .filter(|u| !u.is_empty())
            .ok_or_else(err)?
            .parse()?;
        if parts.next() != Some(VERSION) || parts.next().is_some() {
            return Err(err());
        }
        Ok(Message::Request(Request {
            method,
            uri,
            headers,
            body,
        }))
    }
}

/// Returns (length of the header section, offset of the body).
fn find_head_end(buf: &[u8]) -> Option<(usize, usize)> {
    for i in 0..buf.len() {
        if buf[i..].starts_with(b"\r\n\r\n") {
            return Some((i, i + 4));
        }
        if buf[i..].starts_with(b"\n\n") {
            return Some((i, i + 2));
        }
    }
    None
}

fn parse_headers<'a>(lines: impl Iterator<Item = &'a str>) -> Result<Headers, ParseError> {
    // Unfold continuation lines first (RFC 3261 §7.3.1).
    let mut unfolded: Vec<String> = Vec::new();
    for line in lines {
        if line.starts_with([' ', '\t']) {
            let prev = unfolded
                .last_mut()
                .ok_or_else(|| ParseError::HeaderLine(line.to_owned()))?;
            prev.push(' ');
            prev.push_str(line.trim());
        } else {
            unfolded.push(line.to_owned());
        }
    }

    let mut headers = Headers::new();
    for line in &unfolded {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| ParseError::HeaderLine(line.clone()))?;
        let name = name.trim_end();
        if !super::text::is_token(name) {
            return Err(ParseError::HeaderLine(line.clone()));
        }
        headers.push(name, value.trim());
    }
    Ok(headers)
}
