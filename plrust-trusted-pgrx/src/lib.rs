//! `plrust-trusted-pgrx` is a re-export crate based on [pgrx](https://crates.io/crates/pgrx) which exposes
//! the minimal set of `pgrx` internals necessary for `plrust` function compilation.  `plrust-trusted-pgrx`
//! also includes a number of Rust types for interoperating with Postgres types, access to Postgres'
//! "SPI", logging, and trigger support.

/// Use all the things.  
///
/// `plrust` user crates use the `plrust-trusted-pgrx` crate as if it were named `pgrx`, and all user
/// functions contain a `use pgrx::prelude::*;` statement.
pub mod prelude {
    pub use super::*;
}

pub use ::pgrx::{
    debug1, debug2, debug3, debug4, debug5, ereport, error, info, log, notice, warning,
};

pub use datum::*;

/// Safe Rust wrappers for various Postgres types.
pub mod datum {
    // traits
    pub use ::pgrx::datum::{FromDatum, IntoDatum};

    // dates & times
    pub use ::pgrx::datum::{
        Date, DateTimeConversionError, DateTimeParts, HasExtractableParts, Interval, Time,
        TimeWithTimeZone, Timestamp, TimestampWithTimeZone,
    };

    // zero-copy Arrays
    pub use ::pgrx::datum::{Array, ArrayIntoIterator, ArrayIterator, ArrayTypedIterator};

    // json
    pub use ::pgrx::datum::{Json, JsonB};

    // geometric types
    pub use ::pgrx::pg_sys::{Point, BOX};

    // uuid types
    pub use ::pgrx::datum::Uuid;

    // range types
    pub use ::pgrx::datum::{Range, RangeBound, RangeSubType};

    // dynamic types
    pub use ::pgrx::datum::AnyNumeric;

    // others
    pub use ::pgrx::pg_sys::Oid;
}

#[doc(hidden)]
pub mod fcinfo {
    pub use ::pgrx::fcinfo::pg_getarg;
    pub use ::pgrx::fcinfo::pg_return_null;
    pub use ::pgrx::fcinfo::pg_return_void;
    pub use ::pgrx::fcinfo::srf_first_call_init;
    pub use ::pgrx::fcinfo::srf_is_first_call;
    pub use ::pgrx::fcinfo::srf_per_call_setup;
    pub use ::pgrx::fcinfo::srf_return_done;
    pub use ::pgrx::fcinfo::srf_return_next;
}

pub use heap_tuple::*;

/// Support for arbitrary composite types as a "heap tuple".
pub mod heap_tuple {
    pub use ::pgrx::heap_tuple::PgHeapTuple;
}

pub use iter::*;

/// Return iterators from plrust functions
pub mod iter {
    pub use ::pgrx::iter::{SetOfIterator, TableIterator};
}

#[doc(hidden)]
pub use memcxt::*;
#[doc(hidden)]
pub mod memcxt {
    pub use ::pgrx::memcxt::PgMemoryContexts;
}

#[doc(hidden)]
pub use pgbox::*;
#[doc(hidden)]
pub mod pgbox {
    pub use ::pgrx::pgbox::{PgBox, WhoAllocated};
}

pub use pg_sys::panic::ErrorReportable;
pub use pg_sys::*;

/// Lower-level Postgres internals, which are safe to use.
pub mod pg_sys {
    pub use ::pgrx::pg_sys::elog::PgLogLevel;
    pub use ::pgrx::pg_sys::errcodes::PgSqlErrorCode;
    pub use ::pgrx::pg_sys::pg_try::PgTryBuilder;
    pub use ::pgrx::pg_sys::Datum;
    #[doc(hidden)]
    pub use ::pgrx::pg_sys::FuncCallContext;
    #[doc(hidden)]
    pub use ::pgrx::pg_sys::FunctionCallInfo;
    #[doc(hidden)]
    pub use ::pgrx::pg_sys::Pg_finfo_record;
    pub use ::pgrx::pg_sys::{BuiltinOid, PgBuiltInOids};
    pub use ::pgrx::pg_sys::{ItemPointerData, Oid};

    pub mod panic {
        pub use super::submodules::panic::ErrorReportable;
    }

    pub mod oids {
        pub use ::pgrx::pg_sys::oids::{NotBuiltinOid, PgBuiltInOids, PgOid};
    }

    pub mod submodules {
        pub mod elog {
            pub use ::pgrx::pg_sys::submodules::elog::PgLogLevel;
        }

        pub mod errcodes {
            pub use ::pgrx::pg_sys::submodules::errcodes::PgSqlErrorCode;
        }

        pub mod panic {
            pub use ::pgrx::pg_sys::submodules::panic::pgrx_extern_c_guard;
            pub use ::pgrx::pg_sys::submodules::panic::ErrorReportable;
        }
    }
}

pub use spi::Spi;

/// Use Postgres' Server Programming Interface to execute arbitrary SQL.
pub mod spi {
    pub use ::pgrx::spi::{
        self, Error, Result, Spi, SpiClient, SpiCursor, SpiErrorCodes, SpiHeapTupleData,
        SpiHeapTupleDataEntry, SpiOkCodes, SpiTupleTable, UnknownVariant,
    };
}

pub use trigger_support::*;

/// Various types for use when a `plrust` function is a trigger function.
pub mod trigger_support {
    pub use ::pgrx::trigger_support::{
        PgTrigger, PgTriggerError, PgTriggerLevel, PgTriggerOperation, PgTriggerWhen, TriggerEvent,
        TriggerTuple,
    };
}

#[doc(hidden)]
pub use pgrx_macros::*;
#[doc(hidden)]
pub mod pgrx_macros {
    pub use ::pgrx::pgrx_macros::pg_extern;
    pub use ::pgrx::pgrx_macros::pg_guard;
    pub use ::pgrx::pgrx_macros::pg_trigger;
}
