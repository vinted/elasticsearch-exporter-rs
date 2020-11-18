use serde_json::Value;
use std::error::Error as StdError;
use std::fmt;
use std::num::{ParseFloatError, ParseIntError};

/// Metric error wrapper of parsing errors
#[derive(Debug)]
pub struct MetricError {
    kind: Kind,
    value: Option<Value>,
}

impl MetricError {
    /// Unknown error
    pub fn unknown(s: String, value: Option<Value>) -> Self {
        MetricError {
            kind: Kind::Unknown(s),
            value,
        }
    }

    /// Metric error with metadata from parse float error
    pub fn from_parse_float(e: ParseFloatError, value: Option<Value>) -> Self {
        MetricError {
            kind: Kind::ParseFloat(e),
            value,
        }
    }

    /// Metric error with metadata from parse int error
    pub fn from_parse_int(e: ParseIntError, value: Option<Value>) -> Self {
        MetricError {
            kind: Kind::ParseInt(e),
            value,
        }
    }
}

#[derive(Debug)]
enum Kind {
    ParseInt(ParseIntError),
    ParseFloat(ParseFloatError),
    Unknown(String),
}

impl StdError for MetricError {}

impl fmt::Display for MetricError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MetricError kind {:?} value: {:?}",
            self.kind, self.value
        )
    }
}
