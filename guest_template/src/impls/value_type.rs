use crate::guest::ValueType;

impl From<interface::ValueType> for ValueType {
    fn from(v: interface::ValueType) -> Self {
        match v {
            interface::ValueType::String => ValueType::String,
            interface::ValueType::StringArray => ValueType::StringArray,
            interface::ValueType::I64 => ValueType::I64,
            interface::ValueType::I64Array => ValueType::I64Array,
            interface::ValueType::I32 => ValueType::I32,
            interface::ValueType::I32Array => ValueType::I32Array,
            interface::ValueType::Bool => ValueType::Bool,
            interface::ValueType::BoolArray => ValueType::BoolArray,
            interface::ValueType::Bytea => ValueType::Bytea,
            interface::ValueType::ByteaArray => ValueType::ByteaArray,
        }
    }
}

impl From<ValueType> for interface::ValueType {
    fn from(v: ValueType) -> Self {
        match v {
            ValueType::String => interface::ValueType::String,
            ValueType::StringArray => interface::ValueType::StringArray,
            ValueType::I64 => interface::ValueType::I64,
            ValueType::I64Array => interface::ValueType::I64Array,
            ValueType::I32 => interface::ValueType::I32,
            ValueType::I32Array => interface::ValueType::I32Array,
            ValueType::Bool => interface::ValueType::Bool,
            ValueType::BoolArray => interface::ValueType::BoolArray,
            ValueType::Bytea => interface::ValueType::Bytea,
            ValueType::ByteaArray => interface::ValueType::ByteaArray,
        }
    }
}
