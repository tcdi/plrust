use crate::host::{ValueParam, ValueResult, ValueType};

impl TryFrom<ValueResult> for i64 {
    type Error = crate::host::Error;
    fn try_from(v: ValueResult) -> Result<i64, Self::Error> {
        match v {
            ValueResult::Bigint(s) => Ok(s),
            v => Err(crate::host::Error::conversion(v.into(), ValueType::Bigint)),
        }
    }
}

impl From<i64> for ValueResult {
    fn from(s: i64) -> Self {
        ValueResult::Bigint(s)
    }
}

impl TryFrom<ValueResult> for Vec<Option<i64>> {
    type Error = crate::host::Error;
    fn try_from(v: ValueResult) -> Result<Vec<Option<i64>>, Self::Error> {
        match v {
            ValueResult::BigintArray(s) => Ok(s),
            v => Err(crate::host::Error::conversion(v.into(), ValueType::BigintArray)),
        }
    }
}

impl From<Vec<Option<i64>>> for ValueResult {
    fn from(s: Vec<Option<i64>>) -> Self {
        ValueResult::BigintArray(s)
    }
}

impl TryFrom<ValueResult> for i32 {
    type Error = crate::host::Error;
    fn try_from(v: ValueResult) -> Result<i32, Self::Error> {
        match v {
            ValueResult::Int(s) => Ok(s),
            v => Err(crate::host::Error::conversion(v.into(), ValueType::Int)),
        }
    }
}

impl From<i32> for ValueResult {
    fn from(s: i32) -> Self {
        ValueResult::Int(s)
    }
}

impl TryFrom<ValueResult> for Vec<Option<i32>> {
    type Error = crate::host::Error;
    fn try_from(v: ValueResult) -> Result<Vec<Option<i32>>, Self::Error> {
        match v {
            ValueResult::IntArray(s) => Ok(s),
            v => Err(crate::host::Error::conversion(v.into(), ValueType::IntArray)),
        }
    }
}

impl From<Vec<Option<i32>>> for ValueResult {
    fn from(s: Vec<Option<i32>>) -> Self {
        ValueResult::IntArray(s)
    }
}

impl TryFrom<ValueResult> for bool {
    type Error = crate::host::Error;
    fn try_from(v: ValueResult) -> Result<bool, Self::Error> {
        match v {
            ValueResult::Bool(s) => Ok(s),
            v => Err(crate::host::Error::conversion(v.into(), ValueType::Bool)),
        }
    }
}

impl From<bool> for ValueResult {
    fn from(s: bool) -> Self {
        ValueResult::Bool(s)
    }
}

impl TryFrom<ValueResult> for Vec<Option<bool>> {
    type Error = crate::host::Error;
    fn try_from(v: ValueResult) -> Result<Vec<Option<bool>>, Self::Error> {
        match v {
            ValueResult::BoolArray(s) => Ok(s),
            v => Err(crate::host::Error::conversion(v.into(), ValueType::Bool)),
        }
    }
}

impl From<Vec<Option<bool>>> for ValueResult {
    fn from(s: Vec<Option<bool>>) -> Self {
        ValueResult::BoolArray(s)
    }
}

impl TryFrom<ValueResult> for String {
    type Error = crate::host::Error;
    fn try_from(v: ValueResult) -> Result<String, Self::Error> {
        match v {
            ValueResult::Text(s) => Ok(s),
            v => Err(crate::host::Error::conversion(v.into(), ValueType::Text)),
        }
    }
}

impl From<String> for ValueResult {
    fn from(s: String) -> Self {
        ValueResult::Text(s)
    }
}

impl TryFrom<ValueResult> for Vec<Option<String>> {
    type Error = crate::host::Error;
    fn try_from(v: ValueResult) -> Result<Vec<Option<String>>, Self::Error> {
        match v {
            ValueResult::TextArray(s) => Ok(s),
            v => Err(crate::host::Error::conversion(v.into(), ValueType::TextArray)),
        }
    }
}

impl From<Vec<Option<String>>> for ValueResult {
    fn from(s: Vec<Option<String>>) -> Self {
        ValueResult::TextArray(s)
    }
}

impl TryFrom<ValueResult> for Vec<u8> {
    type Error = crate::host::Error;
    fn try_from(v: ValueResult) -> Result<Vec<u8>, Self::Error> {
        match v {
            ValueResult::Bytea(s) => Ok(s),
            v => Err(crate::host::Error::conversion(v.into(), ValueType::Bytea)),
        }
    }
}

impl From<Vec<u8>> for ValueResult {
    fn from(s: Vec<u8>) -> Self {
        ValueResult::Bytea(s)
    }
}

impl TryFrom<ValueResult> for Vec<Option<Vec<u8>>> {
    type Error = crate::host::Error;
    fn try_from(v: ValueResult) -> Result<Vec<Option<Vec<u8>>>, Self::Error> {
        match v {
            ValueResult::ByteaArray(s) => Ok(s),
            v => Err(crate::host::Error::conversion(v.into(), ValueType::ByteaArray)),
        }
    }
}

impl From<Vec<Option<Vec<u8>>>> for ValueResult {
    fn from(s: Vec<Option<Vec<u8>>>) -> Self {
        ValueResult::ByteaArray(s)
    }
}


impl<'a> From<ValueParam<'a>> for ValueResult {
    fn from(v: ValueParam) -> Self {
        match v {
            ValueParam::Text(i) => ValueResult::Text(i.to_string()),
            ValueParam::TextArray(i) => ValueResult::TextArray(i.into_iter().map(|opt_v| opt_v.map(|v| v.to_string())).collect()),
            ValueParam::Int(i) => ValueResult::Int(i),
            ValueParam::IntArray(i) => ValueResult::IntArray(i.to_vec()),
            ValueParam::Bigint(i) => ValueResult::Bigint(i),
            ValueParam::BigintArray(i) => ValueResult::BigintArray(i.to_vec()),
            ValueParam::Bool(i) => ValueResult::Bool(i),
            ValueParam::BoolArray(i) => ValueResult::BoolArray(i.to_vec()),
            ValueParam::Bytea(i) => ValueResult::Bytea(i.to_vec()),
            ValueParam::ByteaArray(i) => ValueResult::ByteaArray(i.into_iter().map(|opt_v| opt_v.map(|v| v.to_vec())).collect()),
        }
    }
}
