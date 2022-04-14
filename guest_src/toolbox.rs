use crate::{guest, host};

// Value

impl TryFrom<guest::Value> for String {
    type Error = guest::Error;
    fn try_from(v: guest::Value) -> Result<String, Self::Error> {
        match v {
            guest::Value::String(s) => Ok(s),
            v => Err(guest::Error::conversion(v.into(), guest::ValueType::String)),
        }
    }
}

impl From<String> for guest::Value {
    fn from(s: String) -> Self {
        guest::Value::String(s)
    }
}

impl TryFrom<guest::Value> for i64 {
    type Error = guest::Error;
    fn try_from(v: guest::Value) -> Result<i64, Self::Error> {
        match v {
            guest::Value::I64(s) => Ok(s),
            v => Err(guest::Error::conversion(v.into(), guest::ValueType::I64)),
        }
    }
}

impl From<i64> for guest::Value {
    fn from(s: i64) -> Self {
        guest::Value::I64(s)
    }
}

impl TryFrom<guest::Value> for i32 {
    type Error = guest::Error;
    fn try_from(v: guest::Value) -> Result<i32, Self::Error> {
        match v {
            guest::Value::I32(s) => Ok(s),
            v => Err(guest::Error::conversion(v.into(), guest::ValueType::I32)),
        }
    }
}

impl From<i32> for guest::Value {
    fn from(s: i32) -> Self {
        guest::Value::I32(s)
    }
}


impl From<host::ValueResult> for guest::Value {
    fn from(v: host::ValueResult) -> Self {
        match v {
            host::ValueResult::String(i) => guest::Value::String(i),
            host::ValueResult::I64(i) => guest::Value::I64(i),
            host::ValueResult::I32(i) => guest::Value::I32(i),
            host::ValueResult::Bool(i) => guest::Value::Bool(i),
        }
    }
}

impl<'a> From<host::ValueParam<'a>> for guest::Value {
    fn from(v: host::ValueParam) -> Self {
        match v {
            host::ValueParam::String(i) => guest::Value::String(i.to_string()),
            host::ValueParam::I64(i) => guest::Value::I64(i),
            host::ValueParam::I32(i) => guest::Value::I32(i),
            host::ValueParam::Bool(i) => guest::Value::Bool(i),
        }
    }
}

// ValueParam

impl<'a> TryFrom<host::ValueParam<'a>> for &'a str {
    type Error = guest::Error;
    fn try_from(v: host::ValueParam<'a>) -> Result<&'a str, Self::Error> {
        match v {
            host::ValueParam::String(s) => Ok(s),
            v => Err(guest::Error::conversion(v.into(), guest::ValueType::String)),
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

impl<'a> TryFrom<host::ValueParam<'a>> for i64 {
    type Error = guest::Error;
    fn try_from(v: host::ValueParam<'a>) -> Result<i64, Self::Error> {
        match v {
            host::ValueParam::I64(s) => Ok(s),
            v => Err(guest::Error::conversion(v.into(), guest::ValueType::I64)),
        }
    }
}

impl<'a> From<i32> for host::ValueParam<'a> {
    fn from(s: i32) -> Self {
        host::ValueParam::I32(s)
    }
}

impl<'a> TryFrom<host::ValueParam<'a>> for i32 {
    type Error = guest::Error;
    fn try_from(v: host::ValueParam<'a>) -> Result<i32, Self::Error> {
        match v {
            host::ValueParam::I32(s) => Ok(s),
            v => Err(guest::Error::conversion(v.into(), guest::ValueType::I32)),
        }
    }
}

impl<'a> From<bool> for host::ValueParam<'a> {
    fn from(s: bool) -> Self {
        host::ValueParam::Bool(s)
    }
}

impl<'a> TryFrom<host::ValueParam<'a>> for bool {
    type Error = guest::Error;
    fn try_from(v: host::ValueParam<'a>) -> Result<bool, Self::Error> {
        match v {
            host::ValueParam::Bool(s) => Ok(s),
            v => Err(guest::Error::conversion(v.into(), guest::ValueType::Bool)),
        }
    }
}

// ValueResult

impl TryFrom<host::ValueResult> for String {
    type Error = guest::Error;
    fn try_from(v: host::ValueResult) -> Result<String, Self::Error> {
        match v {
            host::ValueResult::String(s) => Ok(s),
            v => Err(guest::Error::conversion(v.into(), guest::ValueType::String)),
        }
    }
}

impl From<String> for host::ValueResult {
    fn from(s: String) -> Self {
        host::ValueResult::String(s)
    }
}

impl TryFrom<host::ValueResult> for i64 {
    type Error = guest::Error;
    fn try_from(v: host::ValueResult) -> Result<i64, Self::Error> {
        match v {
            host::ValueResult::I64(s) => Ok(s),
            v => Err(guest::Error::conversion(v.into(), guest::ValueType::I64)),
        }
    }
}

impl From<i64> for host::ValueResult {
    fn from(s: i64) -> Self {
        host::ValueResult::I64(s)
    }
}

impl TryFrom<host::ValueResult> for i32 {
    type Error = guest::Error;
    fn try_from(v: host::ValueResult) -> Result<i32, Self::Error> {
        match v {
            host::ValueResult::I32(s) => Ok(s),
            v => Err(guest::Error::conversion(v.into(), guest::ValueType::I32)),
        }
    }
}

impl From<i32> for host::ValueResult {
    fn from(s: i32) -> Self {
        host::ValueResult::I32(s)
    }
}

impl TryFrom<host::ValueResult> for bool {
    type Error = guest::Error;
    fn try_from(v: host::ValueResult) -> Result<bool, Self::Error> {
        match v {
            host::ValueResult::Bool(s) => Ok(s),
            v => Err(guest::Error::conversion(v.into(), guest::ValueType::Bool)),
        }
    }
}

impl From<bool> for host::ValueResult {
    fn from(s: bool) -> Self {
        host::ValueResult::Bool(s)
    }
}

impl From<guest::Value> for host::ValueResult {
    fn from(v: guest::Value) -> Self {
        match v {
            guest::Value::String(i) => host::ValueResult::String(i),
            guest::Value::I64(i) => host::ValueResult::I64(i),
            guest::Value::I32(i) => host::ValueResult::I32(i),
            guest::Value::Bool(i) => host::ValueResult::Bool(i),
        }
    }
}

// ValueType

impl From<host::ValueType> for guest::ValueType {
    fn from(v: host::ValueType) -> Self {
        match v {
            host::ValueType::String => guest::ValueType::String,
            host::ValueType::I64 => guest::ValueType::I64,
            host::ValueType::I32 => guest::ValueType::I32,
            host::ValueType::Bool => guest::ValueType::Bool,
        }
    }
}

impl From<guest::ValueType> for host::ValueType {
    fn from(v: guest::ValueType) -> Self {
        match v {
            guest::ValueType::String => host::ValueType::String,
            guest::ValueType::I64 => host::ValueType::I64,
            guest::ValueType::I32 => host::ValueType::I32,
            guest::ValueType::Bool => host::ValueType::Bool,
        }
    }
}

// Errors

impl From<host::Error> for guest::Error {
    fn from(v: host::Error) -> Self {
        match v {
            host::Error::ConversionError(e) => Self::ConversionError(e.into()),
            host::Error::CoerceError(e) => Self::CoerceError(e.into()),
        }
    }
}

impl From<guest::Error> for host::Error {
    fn from(v: guest::Error) -> Self {
        match v {
            guest::Error::ConversionError(e) => Self::ConversionError(e.into()),
            guest::Error::CoerceError(e) => Self::CoerceError(e.into()),
        }
    }
}

impl From<host::ConversionError> for guest::ConversionError {
    fn from(v: host::ConversionError) -> Self {
        Self { value: v.value.into(), into: v.into.into() }
    }
}

impl From<guest::ConversionError> for host::ConversionError {
    fn from(v: guest::ConversionError) -> Self {
        Self { value: v.value.into(), into: v.into.into() }
    }
}

impl guest::Error {
    fn conversion(value: guest::Value, into: guest::ValueType) -> Self {
        Self::ConversionError(guest::ConversionError { value, into })
    }
}