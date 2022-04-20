use crate::guest::ValueType;

impl From<interface::ValueType> for ValueType {
    fn from(v: interface::ValueType) -> Self {
        match v {
            interface::ValueType::String => ValueType::String,
            interface::ValueType::I64 => ValueType::I64,
            interface::ValueType::I32 => ValueType::I32,
            interface::ValueType::Bool => ValueType::Bool,
        }
    }
}

impl From<ValueType> for interface::ValueType {
    fn from(v: ValueType) -> Self {
        match v {
            ValueType::String => interface::ValueType::String,
            ValueType::I64 => interface::ValueType::I64,
            ValueType::I32 => interface::ValueType::I32,
            ValueType::Bool => interface::ValueType::Bool,
        }
    }
}
