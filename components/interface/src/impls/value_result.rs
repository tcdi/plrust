use crate::host::{ValueResult, ValueParam, ValueType};

impl TryFrom<ValueResult> for i64 {
    type Error = crate::host::Error;
    fn try_from(v: ValueResult) -> Result<i64, Self::Error> {
        match v {
            ValueResult::I64(s) => Ok(s),
            v => Err(crate::host::Error::conversion(v.into(), ValueType::I64)),
        }
    }
}

impl From<i64> for ValueResult {
    fn from(s: i64) -> Self {
        ValueResult::I64(s)
    }
}

impl TryFrom<ValueResult> for i32 {
    type Error = crate::host::Error;
    fn try_from(v: ValueResult) -> Result<i32, Self::Error> {
        match v {
            ValueResult::I32(s) => Ok(s),
            v => Err(crate::host::Error::conversion(v.into(), ValueType::I32)),
        }
    }
}

impl From<i32> for ValueResult {
    fn from(s: i32) -> Self {
        ValueResult::I32(s)
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

impl<'a> From<ValueParam<'a>> for ValueResult {
    fn from(v: ValueParam) -> Self {
        match v {
            ValueParam::String(i) => ValueResult::String(i.to_string()),
            ValueParam::I64(i) => ValueResult::I64(i),
            ValueParam::I32(i) => ValueResult::I32(i),
            ValueParam::Bool(i) => ValueResult::Bool(i),
        }
    }
}


impl TryFrom<ValueResult> for String {
    type Error = crate::host::Error;
    fn try_from(v: ValueResult) -> Result<String, Self::Error> {
        match v {
            ValueResult::String(s) => Ok(s),
            v => Err(crate::host::Error::conversion(v.into(), ValueType::String)),
        }
    }
}

impl From<String> for ValueResult {
    fn from(s: String) -> Self {
        ValueResult::String(s)
    }
}
