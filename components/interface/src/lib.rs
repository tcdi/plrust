mod impls;

wit_bindgen_rust::import!("../wit/host.wit");

pub use host::{
    get_one, get_one_with_args, ConversionError, Error, ValueParam, ValueResult, ValueType,
};

impl host::Error {
    fn conversion(value: host::ValueResult, into: host::ValueType) -> Self {
        Self::ConversionError(host::ConversionError { value, into })
    }
}
