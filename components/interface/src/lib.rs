wit_bindgen_rust::import!("../wit/host.wit");

pub use host::{ValueParam, ValueResult, Error, ConversionError, ValueType, get_one, get_one_with_args};

impl<'a> TryFrom<host::ValueParam<'a>> for &'a str {
    type Error = host::Error;
    fn try_from(v: host::ValueParam<'a>) -> Result<&'a str, Self::Error> {
        match v {
            host::ValueParam::String(s) => Ok(s),
            v => Err(host::Error::conversion(v.into(), host::ValueType::String)),
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
    type Error = host::Error;
    fn try_from(v: host::ValueParam<'a>) -> Result<i64, Self::Error> {
        match v {
            host::ValueParam::I64(s) => Ok(s),
            v => Err(host::Error::conversion(v.into(), host::ValueType::I64)),
        }
    }
}

impl<'a> From<i32> for host::ValueParam<'a> {
    fn from(s: i32) -> Self {
        host::ValueParam::I32(s)
    }
}

impl<'a> TryFrom<host::ValueParam<'a>> for i32 {
    type Error = host::Error;
    fn try_from(v: host::ValueParam<'a>) -> Result<i32, Self::Error> {
        match v {
            host::ValueParam::I32(s) => Ok(s),
            v => Err(host::Error::conversion(v.into(), host::ValueType::I32)),
        }
    }
}

impl<'a> From<bool> for host::ValueParam<'a> {
    fn from(s: bool) -> Self {
        host::ValueParam::Bool(s)
    }
}

impl<'a> TryFrom<host::ValueParam<'a>> for bool {
    type Error = host::Error;
    fn try_from(v: host::ValueParam<'a>) -> Result<bool, Self::Error> {
        match v {
            host::ValueParam::Bool(s) => Ok(s),
            v => Err(host::Error::conversion(v.into(), host::ValueType::Bool)),
        }
    }
}


impl TryFrom<host::ValueResult> for i64 {
    type Error = host::Error;
    fn try_from(v: host::ValueResult) -> Result<i64, Self::Error> {
        match v {
            host::ValueResult::I64(s) => Ok(s),
            v => Err(host::Error::conversion(v.into(), host::ValueType::I64)),
        }
    }
}

impl From<i64> for host::ValueResult {
    fn from(s: i64) -> Self {
        host::ValueResult::I64(s)
    }
}

impl TryFrom<host::ValueResult> for i32 {
    type Error = host::Error;
    fn try_from(v: host::ValueResult) -> Result<i32, Self::Error> {
        match v {
            host::ValueResult::I32(s) => Ok(s),
            v => Err(host::Error::conversion(v.into(), host::ValueType::I32)),
        }
    }
}

impl From<i32> for host::ValueResult {
    fn from(s: i32) -> Self {
        host::ValueResult::I32(s)
    }
}

impl TryFrom<host::ValueResult> for bool {
    type Error = host::Error;
    fn try_from(v: host::ValueResult) -> Result<bool, Self::Error> {
        match v {
            host::ValueResult::Bool(s) => Ok(s),
            v => Err(host::Error::conversion(v.into(), host::ValueType::Bool)),
        }
    }
}

impl From<bool> for host::ValueResult {
    fn from(s: bool) -> Self {
        host::ValueResult::Bool(s)
    }
}

impl<'a> From<host::ValueParam<'a>> for host::ValueResult {
    fn from(v: host::ValueParam) -> Self {
        match v {
            host::ValueParam::String(i) => host::ValueResult::String(i.to_string()),
            host::ValueParam::I64(i) => host::ValueResult::I64(i),
            host::ValueParam::I32(i) => host::ValueResult::I32(i),
            host::ValueParam::Bool(i) => host::ValueResult::Bool(i),
        }
    }
}


impl TryFrom<host::ValueResult> for String {
    type Error = host::Error;
    fn try_from(v: host::ValueResult) -> Result<String, Self::Error> {
        match v {
            host::ValueResult::String(s) => Ok(s),
            v => Err(host::Error::conversion(v.into(), host::ValueType::String)),
        }
    }
}

impl From<String> for host::ValueResult {
    fn from(s: String) -> Self {
        host::ValueResult::String(s)
    }
}


impl host::Error {
    fn conversion(value: host::ValueResult, into: host::ValueType) -> Self {
        Self::ConversionError(host::ConversionError { value, into })
    }
}