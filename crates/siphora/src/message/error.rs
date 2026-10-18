#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("message has no end of header section")]
    Incomplete,
    #[error("header section is not valid UTF-8")]
    Utf8,
    #[error("invalid start line: {0:?}")]
    StartLine(String),
    #[error("invalid header line: {0:?}")]
    HeaderLine(String),
    #[error("invalid URI: {0:?}")]
    Uri(String),
    #[error("missing {0} header")]
    MissingHeader(&'static str),
    #[error("invalid {name} header: {value:?}")]
    HeaderValue { name: &'static str, value: String },
    #[error("body is {actual} bytes but Content-Length is {expected}")]
    BodyLength { expected: usize, actual: usize },
}
