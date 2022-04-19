mod impls;

wit_bindgen_rust::import!("../wit/host.wit");

pub use host::{ValueParam, ValueResult, Error, ConversionError, ValueType, get_one, get_one_with_args};


impl host::Error {
    fn conversion(value: host::ValueResult, into: host::ValueType) -> Self {
        Self::ConversionError(host::ConversionError { value, into })
    }
}