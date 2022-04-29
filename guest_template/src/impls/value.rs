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

impl TryFrom<Value> for Vec<Option<String>> {
    type Error = crate::guest::Error;
    fn try_from(v: Value) -> Result<Vec<Option<String>>, Self::Error> {
        match v {
            Value::StringArray(s) => Ok(s),
            v => Err(crate::guest::Error::conversion(v.into(), ValueType::StringArray)),
        }
    }
}

impl From<Vec<Option<String>>> for Value {
    fn from(s: Vec<Option<String>>) -> Self {
        Value::StringArray(s)
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

impl TryFrom<Value> for Vec<Option<i64>> {
    type Error = crate::guest::Error;
    fn try_from(v: Value) -> Result<Vec<Option<i64>>, Self::Error> {
        match v {
            Value::I64Array(s) => Ok(s),
            v => Err(crate::guest::Error::conversion(v.into(), ValueType::I64Array)),
        }
    }
}

impl From<Vec<Option<i64>>> for Value {
    fn from(s: Vec<Option<i64>>) -> Self {
        Value::I64Array(s)
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

impl TryFrom<Value> for Vec<Option<i32>> {
    type Error = crate::guest::Error;
    fn try_from(v: Value) -> Result<Vec<Option<i32>>, Self::Error> {
        match v {
            Value::I32Array(s) => Ok(s),
            v => Err(crate::guest::Error::conversion(v.into(), ValueType::I32Array)),
        }
    }
}

impl From<Vec<Option<i32>>> for Value {
    fn from(s: Vec<Option<i32>>) -> Self {
        Value::I32Array(s)
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
            interface::ValueResult::String(i) => Value::String(i),
            interface::ValueResult::StringArray(i) => Value::StringArray(i),
            interface::ValueResult::I64(i) => Value::I64(i),
            interface::ValueResult::I64Array(i) => Value::I64Array(i),
            interface::ValueResult::I32(i) => Value::I32(i),
            interface::ValueResult::I32Array(i) => Value::I32Array(i),
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
            interface::ValueParam::String(i) => Value::String(i.to_string()),
            interface::ValueParam::StringArray(i) => Value::StringArray(i.into_iter().map(|opt_v| opt_v.map(|v| v.to_string())).collect()),
            interface::ValueParam::I64(i) => Value::I64(i),
            interface::ValueParam::I64Array(i) => Value::I64Array(i.to_vec()),
            interface::ValueParam::I32(i) => Value::I32(i),
            interface::ValueParam::I32Array(i) => Value::I32Array(i.to_vec()),
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
            Value::String(i) => interface::ValueResult::String(i),
            Value::StringArray(i) => interface::ValueResult::StringArray(i),
            Value::I64(i) => interface::ValueResult::I64(i),
            Value::I64Array(i) => interface::ValueResult::I64Array(i),
            Value::I32(i) => interface::ValueResult::I32(i),
            Value::I32Array(i) => interface::ValueResult::I32Array(i),
            Value::Bool(i) => interface::ValueResult::Bool(i),
            Value::BoolArray(i) => interface::ValueResult::BoolArray(i),
            Value::Bytea(i) => interface::ValueResult::Bytea(i),
            Value::ByteaArray(i) => interface::ValueResult::ByteaArray(i),
        }
    }
}
