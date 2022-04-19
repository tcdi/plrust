use crate::host::{ValueParam, ValueType};


impl<'a> TryFrom<ValueParam<'a>> for &'a str {
    type Error = crate::host::Error;
    fn try_from(v: ValueParam<'a>) -> Result<&'a str, Self::Error> {
        match v {
            ValueParam::String(s) => Ok(s),
            v => Err(crate::host::Error::conversion(v.into(), ValueType::String)),
        }
    }
}

impl<'a> From<&'a str> for ValueParam<'a> {
    fn from(s: &'a str) -> Self {
        ValueParam::String(s)
    }
}

impl<'a> From<i64> for ValueParam<'a> {
    fn from(s: i64) -> Self {
        ValueParam::I64(s)
    }
}

impl<'a> TryFrom<ValueParam<'a>> for i64 {
    type Error = crate::host::Error;
    fn try_from(v: ValueParam<'a>) -> Result<i64, Self::Error> {
        match v {
            ValueParam::I64(s) => Ok(s),
            v => Err(crate::host::Error::conversion(v.into(), ValueType::I64)),
        }
    }
}

impl<'a> From<i32> for ValueParam<'a> {
    fn from(s: i32) -> Self {
        ValueParam::I32(s)
    }
}

impl<'a> TryFrom<ValueParam<'a>> for i32 {
    type Error = crate::host::Error;
    fn try_from(v: ValueParam<'a>) -> Result<i32, Self::Error> {
        match v {
            ValueParam::I32(s) => Ok(s),
            v => Err(crate::host::Error::conversion(v.into(), ValueType::I32)),
        }
    }
}

impl<'a> From<bool> for ValueParam<'a> {
    fn from(s: bool) -> Self {
        ValueParam::Bool(s)
    }
}

impl<'a> TryFrom<ValueParam<'a>> for bool {
    type Error = crate::host::Error;
    fn try_from(v: ValueParam<'a>) -> Result<bool, Self::Error> {
        match v {
            ValueParam::Bool(s) => Ok(s),
            v => Err(crate::host::Error::conversion(v.into(), ValueType::Bool)),
        }
    }
}