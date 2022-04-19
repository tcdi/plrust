use crate::guest::{Value, ValueType};

impl TryFrom<Value> for String {
    type Error = crate::guest::Error;
    fn try_from(v: Value) -> Result<String, Self::Error> {
        match v {
            Value::String(s) => Ok(s),
            v => Err(crate::guest::Error::conversion(v.into(), ValueType::String)),
        }
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl TryFrom<Value> for i64 {
    type Error = crate::guest::Error;
    fn try_from(v: Value) -> Result<i64, Self::Error> {
        match v {
            Value::I64(s) => Ok(s),
            v => Err(crate::guest::Error::conversion(v.into(), ValueType::I64)),
        }
    }
}

impl From<i64> for Value {
    fn from(s: i64) -> Self {
        Value::I64(s)
    }
}

impl TryFrom<Value> for i32 {
    type Error = crate::guest::Error;
    fn try_from(v: Value) -> Result<i32, Self::Error> {
        match v {
            Value::I32(s) => Ok(s),
            v => Err(crate::guest::Error::conversion(v.into(), ValueType::I32)),
        }
    }
}

impl From<i32> for Value {
    fn from(s: i32) -> Self {
        Value::I32(s)
    }
}


impl From<interface::ValueResult> for Value {
    fn from(v: interface::ValueResult) -> Self {
        match v {
            interface::ValueResult::String(i) => Value::String(i),
            interface::ValueResult::I64(i) => Value::I64(i),
            interface::ValueResult::I32(i) => Value::I32(i),
            interface::ValueResult::Bool(i) => Value::Bool(i),
        }
    }
}

impl<'a> From<interface::ValueParam<'a>> for Value {
    fn from(v: interface::ValueParam) -> Self {
        match v {
            interface::ValueParam::String(i) => Value::String(i.to_string()),
            interface::ValueParam::I64(i) => Value::I64(i),
            interface::ValueParam::I32(i) => Value::I32(i),
            interface::ValueParam::Bool(i) => Value::Bool(i),
        }
    }
}


impl From<Value> for interface::ValueResult {
    fn from(v: Value) -> Self {
        match v {
            Value::String(i) => interface::ValueResult::String(i),
            Value::I64(i) => interface::ValueResult::I64(i),
            Value::I32(i) => interface::ValueResult::I32(i),
            Value::Bool(i) => interface::ValueResult::Bool(i),
        }
    }
}
