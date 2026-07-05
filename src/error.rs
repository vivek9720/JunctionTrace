use core::fmt;

pub type Result<T> = core::result::Result<T, JunctionError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JunctionError {
    Truncated { needed: usize, remaining: usize },
    BadMagic,
    UnsupportedVersion(u8),
    InvalidSection(u8),
    InvalidVarint,
    InvalidUtf8,
    LimitExceeded(&'static str),
    Malformed(&'static str),
}

impl fmt::Display for JunctionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JunctionError::Truncated { needed, remaining } => {
                write!(
                    f,
                    "truncated input: need {needed} bytes with {remaining} remaining"
                )
            }
            JunctionError::BadMagic => write!(f, "bad JunctionTrace magic"),
            JunctionError::UnsupportedVersion(v) => write!(f, "unsupported version {v}"),
            JunctionError::InvalidSection(k) => write!(f, "invalid section kind {k}"),
            JunctionError::InvalidVarint => write!(f, "invalid variable-length integer"),
            JunctionError::InvalidUtf8 => write!(f, "invalid utf-8 text field"),
            JunctionError::LimitExceeded(name) => write!(f, "limit exceeded for {name}"),
            JunctionError::Malformed(name) => write!(f, "malformed {name}"),
        }
    }
}

impl std::error::Error for JunctionError {}
