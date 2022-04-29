use crate::guest::{Value, ValueType};

impl TryFrom<Value> for String {
    type Error = crate::guest::Error;
    fn try_from(v: Value) -> Result<String, Self::Error> {
        match v {
            Value::Text(s) => Ok(s),
            v => Err(crate::guest::Error::conversion(v.into(), ValueType::Text)),
        }
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Text(s)
    }
}

impl TryFrom<Value> for Vec<Option<String>> {
    type Error = crate::guest::Error;
    fn try_from(v: Value) -> Result<Vec<Option<String>>, Self::Error> {
        match v {
            Value::TextArray(s) => Ok(s),
            v => Err(crate::guest::Error::conversion(v.into(), ValueType::TextArray)),
        }
    }
}

impl From<Vec<Option<String>>> for Value {
    fn from(s: Vec<Option<String>>) -> Self {
        Value::TextArray(s)
    }
}

impl TryFrom<Value> for i64 {
    type Error = crate::guest::Error;
    fn try_from(v: Value) -> Result<i64, Self::Error> {
        match v {
            Value::Bigint(s) => Ok(s),
            v => Err(crate::guest::Error::conversion(v.into(), ValueType::Bigint)),
        }
    }
}

impl From<i64> for Value {
    fn from(s: i64) -> Self {
        Value::Bigint(s)
    }
}

impl TryFrom<Value> for Vec<Option<i64>> {
    type Error = crate::guest::Error;
    fn try_from(v: Value) -> Result<Vec<Option<i64>>, Self::Error> {
        match v {
            Value::BigintArray(s) => Ok(s),
            v => Err(crate::guest::Error::conversion(v.into(), ValueType::BigintArray)),
        }
    }
}

impl From<Vec<Option<i64>>> for Value {
    fn from(s: Vec<Option<i64>>) -> Self {
        Value::BigintArray(s)
    }
}

impl TryFrom<Value> for i32 {
    type Error = crate::guest::Error;
    fn try_from(v: Value) -> Result<i32, Self::Error> {
        match v {
            Value::Int(s) => Ok(s),
            v => Err(crate::guest::Error::conversion(v.into(), ValueType::Int)),
        }
    }
}

impl From<i32> for Value {
    fn from(s: i32) -> Self {
        Value::Int(s)
    }
}

impl TryFrom<Value> for Vec<Option<i32>> {
    type Error = crate::guest::Error;
    fn try_from(v: Value) -> Result<Vec<Option<i32>>, Self::Error> {
        match v {
            Value::IntArray(s) => Ok(s),
            v => Err(crate::guest::Error::conversion(v.into(), ValueType::IntArray)),
        }
    }
}

impl From<Vec<Option<i32>>> for Value {
    fn from(s: Vec<Option<i32>>) -> Self {
        Value::IntArray(s)
    }
}

impl TryFrom<Value> for bool {
    type Error = crate::guest::Error;
    fn try_from(v: Value) -> Result<bool, Self::Error> {
        match v {
            Value::Bool(s) => Ok(s),
            v => Err(crate::guest::Error::conversion(v.into(), ValueType::Bool)),
        }
    }
}

impl From<bool> for Value {
    fn from(s: bool) -> Self {
        Value::Bool(s)
    }
}

impl TryFrom<Value> for Vec<Option<bool>> {
    type Error = crate::guest::Error;
    fn try_from(v: Value) -> Result<Vec<Option<bool>>, Self::Error> {
        match v {
            Value::BoolArray(s) => Ok(s),
            v => Err(crate::guest::Error::conversion(v.into(), ValueType::BoolArray)),
        }
    }
}

impl From<Vec<Option<bool>>> for Value {
    fn from(s: Vec<Option<bool>>) -> Self {
        Value::BoolArray(s)
    }
}

impl TryFrom<Value> for Vec<u8> {
    type Error = crate::guest::Error;
    fn try_from(v: Value) -> Result<Vec<u8>, Self::Error> {
        match v {
            Value::Bytea(s) => Ok(s),
            v => Err(crate::guest::Error::conversion(v.into(), ValueType::Bytea)),
        }
    }
}

impl From<Vec<u8>> for Value {
    fn from(s: Vec<u8>) -> Self {
        Value::Bytea(s)
    }
}

impl TryFrom<Value> for Vec<Option<Vec<u8>>> {
    type Error = crate::guest::Error;
    fn try_from(v: Value) -> Result<Vec<Option<Vec<u8>>>, Self::Error> {
        match v {
            Value::ByteaArray(s) => Ok(s),
            v => Err(crate::guest::Error::conversion(v.into(), ValueType::ByteaArray)),
        }
    }
}

impl From<Vec<Option<Vec<u8>>>> for Value {
    fn from(s: Vec<Option<Vec<u8>>>) -> Self {
        Value::ByteaArray(s)
    }
}

impl From<interface::ValueResult> for Value {
    fn from(v: interface::ValueResult) -> Self {
        match v {
            interface::ValueResult::Text(i) => Value::Text(i),
            interface::ValueResult::TextArray(i) => Value::TextArray(i),
            interface::ValueResult::Bigint(i) => Value::Bigint(i),
            interface::ValueResult::BigintArray(i) => Value::BigintArray(i),
            interface::ValueResult::Int(i) => Value::Int(i),
            interface::ValueResult::IntArray(i) => Value::IntArray(i),
            interface::ValueResult::Bool(i) => Value::Bool(i),
            interface::ValueResult::BoolArray(i) => Value::BoolArray(i),
            interface::ValueResult::Bytea(i) => Value::Bytea(i),
            interface::ValueResult::ByteaArray(i) => Value::ByteaArray(i),
        }
    }
}

impl<'a> From<interface::ValueParam<'a>> for Value {
    fn from(v: interface::ValueParam) -> Self {
        match v {
            interface::ValueParam::Text(i) => Value::Text(i.to_string()),
            interface::ValueParam::TextArray(i) => Value::TextArray(i.into_iter().map(|opt_v| opt_v.map(|v| v.to_string())).collect()),
            interface::ValueParam::Bigint(i) => Value::Bigint(i),
            interface::ValueParam::BigintArray(i) => Value::BigintArray(i.to_vec()),
            interface::ValueParam::Int(i) => Value::Int(i),
            interface::ValueParam::IntArray(i) => Value::IntArray(i.to_vec()),
            interface::ValueParam::Bool(i) => Value::Bool(i),
            interface::ValueParam::BoolArray(i) => Value::BoolArray(i.to_vec()),
            interface::ValueParam::Bytea(i) => Value::Bytea(i.to_vec()),
            interface::ValueParam::ByteaArray(i) => Value::ByteaArray(i.into_iter().map(|opt_v| opt_v.map(|v| v.to_vec())).collect()),
        }
    }
}

impl From<Value> for interface::ValueResult {
    fn from(v: Value) -> Self {
        match v {
            Value::Text(i) => interface::ValueResult::Text(i),
            Value::TextArray(i) => interface::ValueResult::TextArray(i),
            Value::Bigint(i) => interface::ValueResult::Bigint(i),
            Value::BigintArray(i) => interface::ValueResult::BigintArray(i),
            Value::Int(i) => interface::ValueResult::Int(i),
            Value::IntArray(i) => interface::ValueResult::IntArray(i),
            Value::Bool(i) => interface::ValueResult::Bool(i),
            Value::BoolArray(i) => interface::ValueResult::BoolArray(i),
            Value::Bytea(i) => interface::ValueResult::Bytea(i),
            Value::ByteaArray(i) => interface::ValueResult::ByteaArray(i),
        }
    }
}
