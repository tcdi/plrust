use crate::{plrust::crate_name_and_path, plrust_store::PlRustStore, wasm_executor::WasmExecutor};
use eyre::eyre;
use pgx::{
    pg_getarg,
    pg_sys::{self, heap_tuple_get_struct},
    FromDatum, IntoDatum, PgBox, PgBuiltInOids, PgOid,
};
use wasmtime::{Linker, Module, Store};

pub(crate) struct GuestWithOids {
    #[allow(dead_code)] // This is mostly here for debugging.
    fn_oid: pg_sys::Oid,
    store: Store<PlRustStore>,
    guest: crate::guest::Guest<PlRustStore>,
    arg_oids: Vec<PgOid>,
    ret_oid: PgOid,
    strict: bool,
}

impl GuestWithOids {
    pub(crate) fn new(executor: &mut WasmExecutor, fn_oid: pg_sys::Oid) -> eyre::Result<Self> {
        let (crate_name, crate_dir) = crate_name_and_path(fn_oid);
        let wasm = format!("{}.wasm", crate_dir.to_str().unwrap());

        let module = match Module::from_file(executor.engine(), wasm) {
            Ok(m) => m,
            Err(e) => panic!(
                "Could not set up module {}.wasm from directory {:#?}: {}",
                crate_name, crate_dir, e
            ),
        };
        let (arg_oids, ret_oid, strict) = unsafe {
            let proc_tuple = pg_sys::SearchSysCache(
                pg_sys::SysCacheIdentifier_PROCOID as i32,
                fn_oid.into_datum().unwrap(),
                0,
                0,
                0,
            );
            if proc_tuple.is_null() {
                panic!("cache lookup failed for function oid {}", fn_oid);
            }

            let mut is_null = false;
            let argtypes_datum = pg_sys::SysCacheGetAttr(
                pg_sys::SysCacheIdentifier_PROCOID as i32,
                proc_tuple,
                pg_sys::Anum_pg_proc_proargtypes as pg_sys::AttrNumber,
                &mut is_null,
            );
            let argtypes =
                Vec::<pg_sys::Oid>::from_datum(argtypes_datum, is_null, pg_sys::OIDARRAYOID)
                    .unwrap()
                    .iter()
                    .map(|&v| PgOid::from(v))
                    .collect::<Vec<_>>();

            let proc_entry = PgBox::from_pg(heap_tuple_get_struct::<pg_sys::FormData_pg_proc>(
                proc_tuple,
            ));
            let rettype = PgOid::from(proc_entry.prorettype);
            let strict = proc_entry.proisstrict;

            // Make **sure** we have a copy as we're about to release it.
            pg_sys::ReleaseSysCache(proc_tuple);
            (argtypes, rettype, strict)
        };

        let mut linker = Linker::new(executor.engine());
        let mut store = Store::new(executor.engine(), PlRustStore::default());

        wasmtime_wasi::add_to_linker(&mut linker, |cx: &mut PlRustStore| &mut cx.wasi)
            .map_err(|e| eyre!(e))?;
        crate::host::add_to_linker(&mut linker, |cx: &mut PlRustStore| &mut cx.host)
            .map_err(|e| eyre!(e))?;

        let (guest, _guest_instance) =
            crate::guest::Guest::instantiate(&mut store, &module, &mut linker, |cx| {
                &mut cx.guest_data
            })
            .map_err(|e| eyre!(e))?;

        Ok(Self {
            arg_oids,
            ret_oid,
            fn_oid,
            strict,
            store,
            guest,
        })
    }

    pub(crate) fn entry(
        &mut self,
        fcinfo: &pg_sys::FunctionCallInfo,
    ) -> eyre::Result<pg_sys::Datum> {
        if self.strict {
            let args = self
                .arg_oids
                .iter()
                .enumerate()
                .map(|(idx, arg_oid)| build_arg(idx, *arg_oid, fcinfo).expect("Got null arg in strict entry function"))
                .collect::<Vec<_>>();

            let retval = self.guest.strict_entry(&mut self.store, &args)??;

            use crate::guest::ValueResult;
            Ok(match retval {
                ValueResult::String(v) => v.into_datum().unwrap(),
                ValueResult::I32(v) => v.into_datum().unwrap(),
                ValueResult::I64(v) => v.into_datum().unwrap(),
                ValueResult::Bool(v) => v.into_datum().unwrap(),
            })
        } else {
            let args = self
                .arg_oids
                .iter()
                .enumerate()
                .map(|(idx, arg_oid)| build_arg(idx, *arg_oid, fcinfo))
                .collect::<Vec<_>>();

            let retval = self.guest.entry(&mut self.store, &args)??;

            use crate::guest::ValueResult;
            Ok(match retval {
                Some(ValueResult::String(v)) => v.into_datum().unwrap(),
                Some(ValueResult::I32(v)) => v.into_datum().unwrap(),
                Some(ValueResult::I64(v)) => v.into_datum().unwrap(),
                Some(ValueResult::Bool(v)) => v.into_datum().unwrap(),
                None => Option::<()>::None.into_datum().unwrap(),
            })
        }
    }
}

fn build_arg(
    idx: usize,
    oid: PgOid,
    fcinfo: &pg_sys::FunctionCallInfo,
) -> Option<crate::guest::ValueParam<'static>> {
    use crate::guest::ValueParam;
    match oid {
        PgOid::BuiltIn(builtin) => match builtin {
            PgBuiltInOids::TEXTOID => pg_getarg(*fcinfo, idx).map(ValueParam::String),
            PgBuiltInOids::BOOLOID => pg_getarg(*fcinfo, idx).map(ValueParam::Bool),
            PgBuiltInOids::INT8OID => pg_getarg(*fcinfo, idx).map(ValueParam::I64),
            PgBuiltInOids::INT4OID => pg_getarg(*fcinfo, idx).map(ValueParam::I32),
            _ => todo!(),
        },
        _ => todo!(),
    }
}
