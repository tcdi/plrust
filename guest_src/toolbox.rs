impl TryInto<String> for crate::guest::Value {
    type Error = crate::guest::Error;
    fn try_into(self) -> Result<String, Self::Error> {
        match self {
            crate::guest::Value::String(s) => Ok(s),
            _ => panic!("Not a string"),
        }
    }
}

impl From<String> for crate::guest::Value {
    fn from(s: String) -> Self {
        crate::guest::Value::String(s)
    }
}

impl<'a> TryInto<&'a str> for crate::host::ValueParam<'a> {
    type Error = crate::guest::Error;
    fn try_into(self) -> Result<&'a str, Self::Error> {
        match self {
            crate::host::ValueParam::String(s) => Ok(s),
            _ => panic!("Not a string"),
        }
    }
}

impl<'a> From<&'a str> for crate::host::ValueParam<'a> {
    fn from(s: &'a str) -> Self {
        crate::host::ValueParam::String(s)
    }
}

impl TryInto<String> for crate::host::ValueResult {
    type Error = crate::guest::Error;
    fn try_into(self) -> Result<String, Self::Error> {
        match self {
            crate::host::ValueResult::String(s) => Ok(s),
            _ => panic!("Not a string"),
        }
    }
}

impl From<String> for crate::host::ValueResult {
    fn from(s: String) -> Self {
        crate::host::ValueResult::String(s)
    }
}

impl From<crate::host::Error> for crate::guest::Error {
    fn from(v: crate::host::Error) -> Self {
        match v {
            crate::host::Error::Message(s) => crate::guest::Error::Message(s),
        }
    }
}

impl From<crate::guest::Error> for crate::host::Error {
    fn from(v: crate::guest::Error) -> Self {
        match v {
            crate::guest::Error::Message(s) => crate::host::Error::Message(s),
        }
    }
}