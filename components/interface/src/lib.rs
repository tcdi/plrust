mod impls;
use impls::value_type::HasValueType;

wit_bindgen_rust::import!("../wit/host.wit");

pub use host::{
    ConversionError, Error, ValueParam, ValueResult, ValueType,
};

pub fn get_one<R>(query: &str) -> Result<Option<R>, host::Error> 
where R: HasValueType + From<ValueResult> {
    let retval = host::get_one(query, R::VALUE_TYPE);
    retval.map(|opt| opt.map(|val| val.into()))
}

pub fn get_one_with_args<'a, R>(query: &str, args: &[ValueParam<'a>]) -> Result<Option<R>, host::Error> 
where R: HasValueType + TryFrom<ValueResult> {
    match host::get_one_with_args(query, args, R::VALUE_TYPE)? {
        Some(val) => 
            Ok(Some(val.try_into()
                .map_err(|_e| host::Error::CoerceError(R::VALUE_TYPE))?)),
        None => Ok(None),
    }
}

impl host::Error {
    fn conversion(value: host::ValueResult, into: host::ValueType) -> Self {
        Self::ConversionError(host::ConversionError { value, into })
    }
}
