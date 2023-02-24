pub mod prelude {
    pub use super::*;
}

pub use ::pgx::{
    debug1, debug2, debug3, debug4, debug5, ereport, error, info, log, notice, warning,
};

pub use datum::*;
pub mod datum {
    // traits
    pub use ::pgx::datum::{FromDatum, IntoDatum};

    // // dates & times
    // pub use ::pgx::datum::{Date, Time, TimeWithTimeZone, Timestamp, TimestampWithTimeZone};

    // json
    pub use ::pgx::datum::{Json, JsonB};

    // geometric types
    pub use ::pgx::pg_sys::{Point, BOX};

    // uuid types
    pub use ::pgx::datum::Uuid;

    // range types
    pub use ::pgx::datum::{Range, RangeBound, RangeSubType};

    // dynamic types
    pub use ::pgx::datum::AnyNumeric;

    // others
    pub use ::pgx::pg_sys::Oid;
}

#[doc(hidden)]
pub mod fcinfo {
    pub use ::pgx::fcinfo::pg_getarg;
    pub use ::pgx::fcinfo::pg_return_null;
    pub use ::pgx::fcinfo::pg_return_void;
    pub use ::pgx::fcinfo::srf_first_call_init;
    pub use ::pgx::fcinfo::srf_is_first_call;
    pub use ::pgx::fcinfo::srf_per_call_setup;
    pub use ::pgx::fcinfo::srf_return_done;
    pub use ::pgx::fcinfo::srf_return_next;
}

pub use heap_tuple::*;
pub mod heap_tuple {
    pub use ::pgx::heap_tuple::PgHeapTuple;
}

pub use iter::*;
pub mod iter {
    pub use ::pgx::iter::{SetOfIterator, TableIterator};
}

#[doc(hidden)]
pub use memcxt::*;
#[doc(hidden)]
pub mod memcxt {
    pub use ::pgx::memcxt::PgMemoryContexts;
}

#[doc(hidden)]
pub use pgbox::*;
#[doc(hidden)]
pub mod pgbox {
    pub use ::pgx::pgbox::{PgBox, WhoAllocated};
}

pub use pg_sys::panic::ErrorReportable;
pub use pg_sys::*;
pub mod pg_sys {
    pub use ::pgx::pg_sys::elog::PgLogLevel;
    pub use ::pgx::pg_sys::errcodes::PgSqlErrorCode;
    pub use ::pgx::pg_sys::pg_try::PgTryBuilder;
    pub use ::pgx::pg_sys::Datum;
    pub use ::pgx::pg_sys::FuncCallContext;
    pub use ::pgx::pg_sys::FunctionCallInfo;
    #[doc(hidden)]
    pub use ::pgx::pg_sys::Pg_finfo_record;
    pub use ::pgx::pg_sys::{BuiltinOid, PgBuiltInOids};
    pub use ::pgx::pg_sys::{ItemPointerData, Oid};

    pub mod panic {
        pub use super::submodules::panic::ErrorReportable;
    }

    pub mod oids {
        pub use ::pgx::pg_sys::oids::{NotBuiltinOid, PgBuiltInOids, PgOid};
    }

    pub mod submodules {
        pub mod elog {
            pub use ::pgx::pg_sys::submodules::elog::PgLogLevel;
        }

        pub mod errcodes {
            pub use ::pgx::pg_sys::submodules::errcodes::PgSqlErrorCode;
        }

        pub mod panic {
            pub use ::pgx::pg_sys::submodules::panic::pgx_extern_c_guard;
            pub use ::pgx::pg_sys::submodules::panic::ErrorReportable;
        }
    }
}

pub use spi::Spi;
pub mod spi {
    pub use ::pgx::spi::{
        self, Error, Result, Spi, SpiClient, SpiCursor, SpiErrorCodes, SpiHeapTupleData,
        SpiHeapTupleDataEntry, SpiOkCodes, SpiTupleTable, UnknownVariant,
    };
}

pub use trigger_support::*;
pub mod trigger_support {
    pub use ::pgx::trigger_support::{
        PgTrigger, PgTriggerError, PgTriggerLevel, PgTriggerOperation, PgTriggerWhen, TriggerEvent,
        TriggerTuple,
    };
}

#[doc(hidden)]
pub use pgx_macros::*;
#[doc(hidden)]
pub mod pgx_macros {
    pub use ::pgx::pgx_macros::pg_extern;
    pub use ::pgx::pgx_macros::pg_guard;
    pub use ::pgx::pgx_macros::pg_trigger;
}
