use crate::host::ValueType;

pub trait HasValueType {
    #[doc(hidden)]
    const VALUE_TYPE: ValueType;
}

impl HasValueType for String {
    const VALUE_TYPE: ValueType = ValueType::String;
}

impl HasValueType for Vec<String> {
    const VALUE_TYPE: ValueType = ValueType::StringArray;
}

impl HasValueType for i64 {
    const VALUE_TYPE: ValueType = ValueType::I64;
}

impl HasValueType for Vec<i64> {
    const VALUE_TYPE: ValueType = ValueType::I64Array;
}

impl HasValueType for i32 {
    const VALUE_TYPE: ValueType = ValueType::I32;
}

impl HasValueType for Vec<i32> {
    const VALUE_TYPE: ValueType = ValueType::I32Array;
}

impl HasValueType for bool {
    const VALUE_TYPE: ValueType = ValueType::Bool;
}

impl HasValueType for Vec<bool> {
    const VALUE_TYPE: ValueType = ValueType::BoolArray;
}
