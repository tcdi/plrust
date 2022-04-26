use crate::{guest, host};
use pgx::{IntoDatum, pg_sys::Datum, spi::Spi, SpiHeapTupleDataEntry};
use std::fmt::{Display, Formatter};

#[derive(Default)]
pub struct Host;

impl host::Host for Host {
    fn get_one_with_args(
        &mut self,
        query: &str,
        args: Vec<host::ValueParam<'_>>,
        returns: host::ValueType,
    ) -> Result<Option<host::ValueResult>, host::Error> {
        let prepared_args = args
            .into_iter()
            .map(|v| match v {
                host::ValueParam::String(s) => {
                    (pgx::pg_sys::PgBuiltInOids::TEXTOID.oid(), s.into_datum())
                },
                host::ValueParam::StringArray(s) => {
                    (pgx::pg_sys::PgBuiltInOids::TEXTARRAYOID.oid(), s.into_datum())
                },
                host::ValueParam::I32(s) => {
                    (pgx::pg_sys::PgBuiltInOids::INT4OID.oid(), s.into_datum())
                },
                host::ValueParam::I32Array(s) => {
                    (pgx::pg_sys::PgBuiltInOids::INT4ARRAYOID.oid(), s.into_datum())
                },
                host::ValueParam::I64(s) => {
                    (pgx::pg_sys::PgBuiltInOids::INT8OID.oid(), s.into_datum())
                },
                host::ValueParam::I64Array(s) => {
                    (pgx::pg_sys::PgBuiltInOids::INT8ARRAYOID.oid(), s.into_datum())
                },
                host::ValueParam::Bool(s) => {
                    (pgx::pg_sys::PgBuiltInOids::BOOLOID.oid(), s.into_datum())
                },
                host::ValueParam::BoolArray(s) => {
                    (pgx::pg_sys::PgBuiltInOids::BOOLARRAYOID.oid(), s.into_datum())
                },
            })
            .collect();

        match returns {
            host::ValueType::String => {
                let s: Option<String> = pgx::spi::Spi::get_one_with_args(query, prepared_args);
                Ok(s.map(|i| i.into()))
            }
            host::ValueType::StringArray => {
                let s: Option<Vec<Option<String>>> = pgx::spi::Spi::get_one_with_args(query, prepared_args);
                Ok(s.map(|i| i.into()))
            }
            host::ValueType::I64 => {
                let s: Option<i64> = pgx::spi::Spi::get_one_with_args(query, prepared_args);
                Ok(s.map(|i| i.into()))
            }
            host::ValueType::I64Array => {
                let s: Option<Vec<Option<i64>>> = pgx::spi::Spi::get_one_with_args(query, prepared_args);
                Ok(s.map(|i| i.into()))
            }
            host::ValueType::I32 => {
                let s: Option<i32> = pgx::spi::Spi::get_one_with_args(query, prepared_args);
                Ok(s.map(|i| i.into()))
            }
            host::ValueType::I32Array => {
                let s: Option<Vec<Option<i32>>> = pgx::spi::Spi::get_one_with_args(query, prepared_args);
                Ok(s.map(|i| i.into()))
            }
            host::ValueType::Bool => {
                let s: Option<bool> = pgx::spi::Spi::get_one_with_args(query, prepared_args);
                Ok(s.map(|i| i.into()))
            }
            host::ValueType::BoolArray => {
                let s: Option<Vec<Option<bool>>> = pgx::spi::Spi::get_one_with_args(query, prepared_args);
                Ok(s.map(|i| i.into()))
            }
        }
    }

    fn get_one(
        &mut self,
        query: &str,
        returns: host::ValueType,
    ) -> Result<Option<host::ValueResult>, host::Error> {
        match returns {
            host::ValueType::String => {
                let s: Option<String> = pgx::spi::Spi::get_one(query);
                Ok(s.map(|i| i.into()))
            }
            host::ValueType::StringArray => {
                let s: Option<Vec<Option<String>>> = pgx::spi::Spi::get_one(query);
                Ok(s.map(|i| i.into()))
            }
            host::ValueType::I64 => {
                let s: Option<i64> = pgx::spi::Spi::get_one(query);
                Ok(s.map(|i| i.into()))
            }
            host::ValueType::I64Array => {
                let s: Option<Vec<Option<i64>>> = pgx::spi::Spi::get_one(query);
                Ok(s.map(|i| i.into()))
            }
            host::ValueType::I32 => {
                let s: Option<i32> = pgx::spi::Spi::get_one(query);
                Ok(s.map(|i| i.into()))
            }
            host::ValueType::I32Array => {
                let s: Option<Vec<Option<i32>>> = pgx::spi::Spi::get_one(query);
                Ok(s.map(|i| i.into()))
            }
            host::ValueType::Bool => {
                let s: Option<bool> = pgx::spi::Spi::get_one(query);
                Ok(s.map(|i| i.into()))
            }
            host::ValueType::BoolArray => {
                let s: Option<Vec<Option<bool>>> = pgx::spi::Spi::get_one(query);
                Ok(s.map(|i| i.into()))
            }
        }
    }
}

impl Display for host::ValueType {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            host::ValueType::String => write!(f, "String"),
            host::ValueType::StringArray => write!(f, "Vec<String>"),
            host::ValueType::I32 => write!(f, "i32"),
            host::ValueType::I32Array => write!(f, "Vec<i32>"),
            host::ValueType::I64 => write!(f, "i64"),
            host::ValueType::I64Array => write!(f, "Vec<i64>"),
            host::ValueType::Bool => write!(f, "bool"),
            host::ValueType::BoolArray => write!(f, "Vec<bool>"),
        }
    }
}

impl Display for guest::ValueType {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            guest::ValueType::String => write!(f, "String"),
            guest::ValueType::StringArray => write!(f, "Vec<String>"),
            guest::ValueType::I32 => write!(f, "i32"),
            guest::ValueType::I32Array => write!(f, "Vec<i32>"),
            guest::ValueType::I64 => write!(f, "i64"),
            guest::ValueType::I64Array => write!(f, "Vec<i64>"),
            guest::ValueType::Bool => write!(f, "bool"),
            guest::ValueType::BoolArray => write!(f, "Vec<bool>"),
        }
    }
}

impl From<String> for host::ValueResult {
    fn from(s: String) -> Self {
        host::ValueResult::String(s)
    }
}

impl From<Vec<Option<String>>> for host::ValueResult {
    fn from(s: Vec<Option<String>>) -> Self {
        host::ValueResult::StringArray(s)
    }
}

impl From<i64> for host::ValueResult {
    fn from(s: i64) -> Self {
        host::ValueResult::I64(s)
    }
}

impl From<Vec<Option<i64>>> for host::ValueResult {
    fn from(s: Vec<Option<i64>>) -> Self {
        host::ValueResult::I64Array(s)
    }
}

impl From<i32> for host::ValueResult {
    fn from(s: i32) -> Self {
        host::ValueResult::I32(s)
    }
}

impl From<Vec<Option<i32>>> for host::ValueResult {
    fn from(s: Vec<Option<i32>>) -> Self {
        host::ValueResult::I32Array(s)
    }
}

impl From<bool> for host::ValueResult {
    fn from(s: bool) -> Self {
        host::ValueResult::Bool(s)
    }
}

impl From<Vec<Option<bool>>> for host::ValueResult {
    fn from(s: Vec<Option<bool>>) -> Self {
        host::ValueResult::BoolArray(s)
    }
}