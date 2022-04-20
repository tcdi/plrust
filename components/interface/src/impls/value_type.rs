use crate::host::ValueType;

pub trait HasValueType {
    #[doc(hidden)]
    const VALUE_TYPE: ValueType;
}

impl HasValueType for String {
    const VALUE_TYPE: ValueType = ValueType::String;
}

impl HasValueType for i64 {
    const VALUE_TYPE: ValueType = ValueType::I64;
}

impl HasValueType for i32 {
    const VALUE_TYPE: ValueType = ValueType::I32;
}

impl HasValueType for bool {
    const VALUE_TYPE: ValueType = ValueType::Bool;
}