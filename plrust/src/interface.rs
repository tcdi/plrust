use crate::{guest, host};
use pgx::{IntoDatum, PgOid, pg_sys::Datum};
use std::fmt::{Display, Formatter};

macro_rules! map_value_type {
    ($returns:expr, $operation:expr) => {
        match $returns {
            host::ValueType::Text => {
                let s: Option<String> = $operation;
                Ok(s.map(|i| i.into()))
            },
            host::ValueType::TextArray => {
                let s: Option<Vec<Option<String>>> = $operation;
                Ok(s.map(|i| i.into()))
            },
            host::ValueType::Bigint => {
                let s: Option<i64> = $operation;
                Ok(s.map(|i| i.into()))
            },
            host::ValueType::BigintArray => {
                let s: Option<Vec<Option<i64>>> = $operation;
                Ok(s.map(|i| i.into()))
            },
            host::ValueType::Int => {
                let s: Option<i32> = $operation;
                Ok(s.map(|i| i.into()))
            },
            host::ValueType::IntArray => {
                let s: Option<Vec<Option<i32>>> = $operation;
                Ok(s.map(|i| i.into()))
            },
            host::ValueType::Bool => {
                let s: Option<bool> = $operation;
                Ok(s.map(|i| i.into()))
            },
            host::ValueType::BoolArray => {
                let s: Option<Vec<Option<bool>>> = $operation;
                Ok(s.map(|i| i.into()))
            },
            host::ValueType::Bytea => {
                let s: Option<Vec<u8>> = $operation;
                Ok(s.map(|i| i.into()))
            },
            host::ValueType::ByteaArray => {
                let s: Option<Vec<Option<Vec<u8>>>> = $operation;
                Ok(s.map(|i| i.into()))
            },
        }
    };
}

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
            .map(host::ValueParam::into_oid_and_datum)
            .collect();

        map_value_type!(returns, pgx::spi::Spi::get_one_with_args(query, prepared_args))
    }

    fn get_one(
        &mut self,
        query: &str,
        returns: host::ValueType,
    ) -> Result<Option<host::ValueResult>, host::Error> {
        map_value_type!(returns, pgx::spi::Spi::get_one(query))
    }
}

impl<'a> host::ValueParam<'a> {
    fn into_oid_and_datum(self) -> (PgOid, Option<Datum>) {
        match self {
            host::ValueParam::Text(s) => {
                (pgx::pg_sys::PgBuiltInOids::TEXTOID.oid(), s.into_datum())
            },
            host::ValueParam::TextArray(s) => {
                (pgx::pg_sys::PgBuiltInOids::TEXTARRAYOID.oid(), s.into_datum())
            },
            host::ValueParam::Int(s) => {
                (pgx::pg_sys::PgBuiltInOids::INT4OID.oid(), s.into_datum())
            },
            host::ValueParam::IntArray(s) => {
                (pgx::pg_sys::PgBuiltInOids::INT4ARRAYOID.oid(), s.into_datum())
            },
            host::ValueParam::Bigint(s) => {
                (pgx::pg_sys::PgBuiltInOids::INT8OID.oid(), s.into_datum())
            },
            host::ValueParam::BigintArray(s) => {
                (pgx::pg_sys::PgBuiltInOids::INT8ARRAYOID.oid(), s.into_datum())
            },
            host::ValueParam::Bool(s) => {
                (pgx::pg_sys::PgBuiltInOids::BOOLOID.oid(), s.into_datum())
            },
            host::ValueParam::BoolArray(s) => {
                (pgx::pg_sys::PgBuiltInOids::BOOLARRAYOID.oid(), s.into_datum())
            },
            host::ValueParam::Bytea(s) => {
                (pgx::pg_sys::PgBuiltInOids::BYTEAOID.oid(), s.into_datum())
            },
            host::ValueParam::ByteaArray(s) => {
                (pgx::pg_sys::PgBuiltInOids::BYTEAARRAYOID.oid(), s.into_datum())
            },
        }
    }
}

impl Display for host::ValueType {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            host::ValueType::Text => write!(f, "String"),
            host::ValueType::TextArray => write!(f, "Vec<String>"),
            host::ValueType::Int => write!(f, "i32"),
            host::ValueType::IntArray => write!(f, "Vec<i32>"),
            host::ValueType::Bigint => write!(f, "i64"),
            host::ValueType::BigintArray => write!(f, "Vec<i64>"),
            host::ValueType::Bool => write!(f, "bool"),
            host::ValueType::BoolArray => write!(f, "Vec<bool>"),
            host::ValueType::Bytea => write!(f, "Vec<u8>"),
            host::ValueType::ByteaArray => write!(f, "Vec<Vec<u8>>"),
        }
    }
}

impl Display for guest::ValueType {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            guest::ValueType::Text => write!(f, "String"),
            guest::ValueType::TextArray => write!(f, "Vec<String>"),
            guest::ValueType::Int => write!(f, "i32"),
            guest::ValueType::IntArray => write!(f, "Vec<i32>"),
            guest::ValueType::Bigint => write!(f, "i64"),
            guest::ValueType::BigintArray => write!(f, "Vec<i64>"),
            guest::ValueType::Bool => write!(f, "bool"),
            guest::ValueType::BoolArray => write!(f, "Vec<bool>"),
            guest::ValueType::Bytea => write!(f, "Vec<u8>"),
            guest::ValueType::ByteaArray => write!(f, "Vec<Vec<u8>>"),
        }
    }
}

impl From<String> for host::ValueResult {
    fn from(s: String) -> Self {
        host::ValueResult::Text(s)
    }
}

impl From<Vec<Option<String>>> for host::ValueResult {
    fn from(s: Vec<Option<String>>) -> Self {
        host::ValueResult::TextArray(s)
    }
}

impl From<i64> for host::ValueResult {
    fn from(s: i64) -> Self {
        host::ValueResult::Bigint(s)
    }
}

impl From<Vec<Option<i64>>> for host::ValueResult {
    fn from(s: Vec<Option<i64>>) -> Self {
        host::ValueResult::BigintArray(s)
    }
}

impl From<i32> for host::ValueResult {
    fn from(s: i32) -> Self {
        host::ValueResult::Int(s)
    }
}

impl From<Vec<Option<i32>>> for host::ValueResult {
    fn from(s: Vec<Option<i32>>) -> Self {
        host::ValueResult::IntArray(s)
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

impl From<Vec<u8>> for host::ValueResult {
    fn from(s: Vec<u8>) -> Self {
        host::ValueResult::Bytea(s)
    }
}

impl From<Vec<Option<Vec<u8>>>> for host::ValueResult {
    fn from(s: Vec<Option<Vec<u8>>>) -> Self {
        host::ValueResult::ByteaArray(s)
    }
}