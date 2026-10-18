//! A Rust SIP signaling toolkit.
//!
//! Layers, bottom up (RFC 3261 §5):
//! - [`message`]: message model, parser and serializer
//! - transport: not started
//! - transaction: not started
//! - dialog: not started

pub mod message;

pub use message::{
    CSeq, Headers, Message, Method, NameAddr, Params, ParseError, Request, Response, Scheme,
    StatusCode, Uri, Via, names,
};
