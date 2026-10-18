use std::fmt;

/// A response status code in the range 100..=699.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StatusCode(u16);

impl StatusCode {
    pub const TRYING: Self = Self(100);
    pub const RINGING: Self = Self(180);
    pub const SESSION_PROGRESS: Self = Self(183);
    pub const OK: Self = Self(200);
    pub const BAD_REQUEST: Self = Self(400);
    pub const UNAUTHORIZED: Self = Self(401);
    pub const FORBIDDEN: Self = Self(403);
    pub const NOT_FOUND: Self = Self(404);
    pub const METHOD_NOT_ALLOWED: Self = Self(405);
    pub const PROXY_AUTHENTICATION_REQUIRED: Self = Self(407);
    pub const REQUEST_TIMEOUT: Self = Self(408);
    pub const TEMPORARILY_UNAVAILABLE: Self = Self(480);
    pub const CALL_DOES_NOT_EXIST: Self = Self(481);
    pub const BUSY_HERE: Self = Self(486);
    pub const REQUEST_TERMINATED: Self = Self(487);
    pub const NOT_ACCEPTABLE_HERE: Self = Self(488);
    pub const SERVER_INTERNAL_ERROR: Self = Self(500);
    pub const NOT_IMPLEMENTED: Self = Self(501);
    pub const SERVICE_UNAVAILABLE: Self = Self(503);
    pub const DECLINE: Self = Self(603);

    pub fn new(code: u16) -> Option<Self> {
        (100..=699).contains(&code).then_some(Self(code))
    }

    pub fn as_u16(self) -> u16 {
        self.0
    }

    pub fn is_provisional(self) -> bool {
        self.0 < 200
    }

    pub fn is_success(self) -> bool {
        (200..300).contains(&self.0)
    }

    pub fn is_final(self) -> bool {
        self.0 >= 200
    }

    /// The reason phrase RFC 3261 §21 suggests, or "" for unlisted codes.
    pub fn canonical_reason(self) -> &'static str {
        match self.0 {
            100 => "Trying",
            180 => "Ringing",
            181 => "Call Is Being Forwarded",
            182 => "Queued",
            183 => "Session Progress",
            200 => "OK",
            202 => "Accepted",
            300 => "Multiple Choices",
            301 => "Moved Permanently",
            302 => "Moved Temporarily",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            407 => "Proxy Authentication Required",
            408 => "Request Timeout",
            415 => "Unsupported Media Type",
            420 => "Bad Extension",
            423 => "Interval Too Brief",
            480 => "Temporarily Unavailable",
            481 => "Call/Transaction Does Not Exist",
            482 => "Loop Detected",
            483 => "Too Many Hops",
            484 => "Address Incomplete",
            486 => "Busy Here",
            487 => "Request Terminated",
            488 => "Not Acceptable Here",
            491 => "Request Pending",
            500 => "Server Internal Error",
            501 => "Not Implemented",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            504 => "Server Time-out",
            505 => "Version Not Supported",
            600 => "Busy Everywhere",
            603 => "Decline",
            604 => "Does Not Exist Anywhere",
            606 => "Not Acceptable",
            _ => "",
        }
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
