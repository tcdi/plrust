use crate::{guest, host};
use std::fmt::Debug;


// Value

impl TryInto<String> for guest::Value {
    type Error = guest::Error;
    fn try_into(self) -> Result<String, Self::Error> {
        match self {
            guest::Value::String(s) => Ok(s),
            v => Err(guest::Error::conversion(v, "String")),
        }
    }
}

impl From<String> for guest::Value {
    fn from(s: String) -> Self {
        guest::Value::String(s)
    }
}

impl TryInto<i64> for guest::Value {
    type Error = guest::Error;
    fn try_into(self) -> Result<i64, Self::Error> {
        match self {
            guest::Value::I64(s) => Ok(s),
            v => Err(guest::Error::conversion(v, "i64")),
        }
    }
}

impl From<i64> for guest::Value {
    fn from(s: i64) -> Self {
        guest::Value::I64(s)
    }
}

impl TryInto<i32> for guest::Value {
    type Error = guest::Error;
    fn try_into(self) -> Result<i32, Self::Error> {
        match self {
            guest::Value::I32(s) => Ok(s),
            v => Err(guest::Error::conversion(v, "i32")),
        }
    }
}

impl From<i32> for guest::Value {
    fn from(s: i32) -> Self {
        guest::Value::I32(s)
    }
}


// ValueParam

impl<'a> TryInto<&'a str> for host::ValueParam<'a> {
    type Error = guest::Error;
    fn try_into(self) -> Result<&'a str, Self::Error> {
        match self {
            host::ValueParam::String(s) => Ok(s),
            v => Err(guest::Error::conversion(v, "String")),
        }
    }
}

impl<'a> From<&'a str> for host::ValueParam<'a> {
    fn from(s: &'a str) -> Self {
        host::ValueParam::String(s)
    }
}

impl<'a> From<i64> for host::ValueParam<'a> {
    fn from(s: i64) -> Self {
        host::ValueParam::I64(s)
    }
}

impl<'a> TryInto<i64> for host::ValueParam<'a> {
    type Error = guest::Error;
    fn try_into(self) -> Result<i64, Self::Error> {
        match self {
            host::ValueParam::I64(s) => Ok(s),
            v => Err(guest::Error::conversion(v, "i64")),
        }
    }
}

impl<'a> From<i32> for host::ValueParam<'a> {
    fn from(s: i32) -> Self {
        host::ValueParam::I32(s)
    }
}

impl<'a> TryInto<i32> for host::ValueParam<'a> {
    type Error = guest::Error;
    fn try_into(self) -> Result<i32, Self::Error> {
        match self {
            host::ValueParam::I32(s) => Ok(s),
            v => Err(guest::Error::conversion(v, "i32")),
        }
    }
}

// ValueResult

impl TryInto<String> for host::ValueResult {
    type Error = guest::Error;
    fn try_into(self) -> Result<String, Self::Error> {
        match self {
            host::ValueResult::String(s) => Ok(s),
            v => Err(guest::Error::conversion(v, "String")),
        }
    }
}

impl From<String> for host::ValueResult {
    fn from(s: String) -> Self {
        host::ValueResult::String(s)
    }
}

impl TryInto<i64> for host::ValueResult {
    type Error = guest::Error;
    fn try_into(self) -> Result<i64, Self::Error> {
        match self {
            host::ValueResult::I64(s) => Ok(s),
            v => Err(guest::Error::conversion(v, "i64")),
        }
    }
}

impl From<i64> for host::ValueResult {
    fn from(s: i64) -> Self {
        host::ValueResult::I64(s)
    }
}

impl TryInto<i32> for host::ValueResult {
    type Error = guest::Error;
    fn try_into(self) -> Result<i32, Self::Error> {
        match self {
            host::ValueResult::I32(s) => Ok(s),
            v => Err(guest::Error::conversion(v, "i32")),
        }
    }
}

impl From<i32> for host::ValueResult {
    fn from(s: i32) -> Self {
        host::ValueResult::I32(s)
    }
}

// Errors

impl From<host::Error> for guest::Error {
    fn from(v: host::Error) -> Self {
        match v {
            host::Error::ConversionError(s) => guest::Error::ConversionError(s.into()),
        }
    }
}

impl From<guest::Error> for host::Error {
    fn from(v: guest::Error) -> Self {
        match v {
            guest::Error::ConversionError(s) => host::Error::ConversionError(s.into()),
        }
    }
}

impl From<host::ConversionError> for guest::ConversionError {
    fn from(v: host::ConversionError) -> Self {
        match v {
            host::ConversionError { value, into } => guest::ConversionError { value, into },
        }
    }
}

impl From<guest::ConversionError> for host::ConversionError {
    fn from(v: guest::ConversionError) -> Self {
        match v {
            guest::ConversionError { value, into } => host::ConversionError { value, into },
        }
    }
}

impl guest::Error {
    fn conversion(value: impl Debug, into: impl Into<String>) -> Self {
        Self::ConversionError(guest::ConversionError::new(value, into))
    }
}

impl guest::ConversionError {
    fn new(value: impl Debug, into: impl Into<String>) -> Self {
        Self {
            value: format!("{:?}", value),
            into: into.into(),
        }
    }
}