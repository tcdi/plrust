use crate::guest;

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


impl From<interface::ValueResult> for guest::Value {
    fn from(v: interface::ValueResult) -> Self {
        match v {
            interface::ValueResult::String(i) => guest::Value::String(i),
            interface::ValueResult::I64(i) => guest::Value::I64(i),
            interface::ValueResult::I32(i) => guest::Value::I32(i),
            interface::ValueResult::Bool(i) => guest::Value::Bool(i),
        }
    }
}

impl<'a> From<interface::ValueParam<'a>> for guest::Value {
    fn from(v: interface::ValueParam) -> Self {
        match v {
            interface::ValueParam::String(i) => guest::Value::String(i.to_string()),
            interface::ValueParam::I64(i) => guest::Value::I64(i),
            interface::ValueParam::I32(i) => guest::Value::I32(i),
            interface::ValueParam::Bool(i) => guest::Value::Bool(i),
        }
    }
}


impl From<guest::Value> for interface::ValueResult {
    fn from(v: guest::Value) -> Self {
        match v {
            guest::Value::String(i) => interface::ValueResult::String(i),
            guest::Value::I64(i) => interface::ValueResult::I64(i),
            guest::Value::I32(i) => interface::ValueResult::I32(i),
            guest::Value::Bool(i) => interface::ValueResult::Bool(i),
        }
    }
}

// ValueType

impl From<interface::ValueType> for guest::ValueType {
    fn from(v: interface::ValueType) -> Self {
        match v {
            interface::ValueType::String => guest::ValueType::String,
            interface::ValueType::I64 => guest::ValueType::I64,
            interface::ValueType::I32 => guest::ValueType::I32,
            interface::ValueType::Bool => guest::ValueType::Bool,
        }
    }
}

impl From<guest::ValueType> for interface::ValueType {
    fn from(v: guest::ValueType) -> Self {
        match v {
            guest::ValueType::String => interface::ValueType::String,
            guest::ValueType::I64 => interface::ValueType::I64,
            guest::ValueType::I32 => interface::ValueType::I32,
            guest::ValueType::Bool => interface::ValueType::Bool,
        }
    }
}

// Errors

impl From<interface::Error> for guest::Error {
    fn from(v: interface::Error) -> Self {
        match v {
            interface::Error::ConversionError(e) => Self::ConversionError(e.into()),
            interface::Error::CoerceError(e) => Self::CoerceError(e.into()),
        }
    }
}

impl From<guest::Error> for interface::Error {
    fn from(v: guest::Error) -> Self {
        match v {
            guest::Error::ConversionError(e) => Self::ConversionError(e.into()),
            guest::Error::CoerceError(e) => Self::CoerceError(e.into()),
        }
    }
}

impl From<interface::ConversionError> for guest::ConversionError {
    fn from(v: interface::ConversionError) -> Self {
        Self { value: v.value.into(), into: v.into.into() }
    }
}

impl From<guest::ConversionError> for interface::ConversionError {
    fn from(v: guest::ConversionError) -> Self {
        Self { value: v.value.into(), into: v.into.into() }
    }
}

impl guest::Error {
    fn conversion(value: guest::Value, into: guest::ValueType) -> Self {
        Self::ConversionError(guest::ConversionError { value, into })
    }
}