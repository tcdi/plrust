impl From<interface::Error> for crate::guest::Error {
    fn from(v: interface::Error) -> Self {
        match v {
            interface::Error::ConversionError(e) => Self::ConversionError(e.into()),
            interface::Error::CoerceError(e) => Self::CoerceError(e.into()),
        }
    }
}

impl From<crate::guest::Error> for interface::Error {
    fn from(v: crate::guest::Error) -> Self {
        match v {
            crate::guest::Error::ConversionError(e) => Self::ConversionError(e.into()),
            crate::guest::Error::CoerceError(e) => Self::CoerceError(e.into()),
        }
    }
}

impl From<interface::ConversionError> for crate::guest::ConversionError {
    fn from(v: interface::ConversionError) -> Self {
        Self {
            value: v.value.into(),
            into: v.into.into(),
        }
    }
}

impl From<crate::guest::ConversionError> for interface::ConversionError {
    fn from(v: crate::guest::ConversionError) -> Self {
        Self {
            value: v.value.into(),
            into: v.into.into(),
        }
    }
}

impl crate::guest::Error {
    pub(crate) fn conversion(value: crate::guest::Value, into: crate::guest::ValueType) -> Self {
        Self::ConversionError(crate::guest::ConversionError { value, into })
    }
}
