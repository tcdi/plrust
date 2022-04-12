wit_bindgen_wasmtime::export!("src/wit/host.wit");
wit_bindgen_wasmtime::import!("src/wit/guest.wit");

use host::{ValueParam, ValueResult};
use pgx::IntoDatum;

#[derive(Default)]
pub struct Host;

impl host::Host for Host {
    fn get_one_with_args(
        &mut self,
        query: &str,
        args: Vec<host::ValueParam<'_>>
    ) -> Result<host::ValueResult, host::Error> {
        let prepared_args = args.into_iter().map(|v| {
            match v {
                ValueParam::String(s) => (pgx::pg_sys::PgBuiltInOids::TEXTOID.oid(), s.into_datum()),
                _ => panic!("oh no"),
            }
        }).collect();
        let s: String = pgx::spi::Spi::get_one_with_args(query, prepared_args).unwrap();
        Ok(ValueResult::String(s))
    }
}