use crate::guest::ValueType;

impl From<interface::ValueType> for ValueType {
    fn from(v: interface::ValueType) -> Self {
        match v {
            interface::ValueType::Text => ValueType::Text,
            interface::ValueType::TextArray => ValueType::TextArray,
            interface::ValueType::Bigint => ValueType::Bigint,
            interface::ValueType::BigintArray => ValueType::BigintArray,
            interface::ValueType::Int => ValueType::Int,
            interface::ValueType::IntArray => ValueType::IntArray,
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
            ValueType::Text => interface::ValueType::Text,
            ValueType::TextArray => interface::ValueType::TextArray,
            ValueType::Bigint => interface::ValueType::Bigint,
            ValueType::BigintArray => interface::ValueType::BigintArray,
            ValueType::Int => interface::ValueType::Int,
            ValueType::IntArray => interface::ValueType::IntArray,
            ValueType::Bool => interface::ValueType::Bool,
            ValueType::BoolArray => interface::ValueType::BoolArray,
            ValueType::Bytea => interface::ValueType::Bytea,
            ValueType::ByteaArray => interface::ValueType::ByteaArray,
        }
    }
}
