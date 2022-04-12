wit_bindgen_wasmtime::export!("src/host.wit");
wit_bindgen_wasmtime::import!("src/guest.wit");

#[derive(Default)]
pub struct Host;

impl host::Host for Host {
    fn get_one_with_args(
        &mut self,
        query: &str,
        args: Vec<host::ValueParam<'_>>
    ) -> host::ValueResult {
        let prepared_args = args.into_iter().map(|v| {
            use pgx::IntoDatum;
            (pgx::pg_sys::PgBuiltInOids::TEXTOID.oid(), match v {
                host::ValueParam::Str(s) => s.into_datum(),
                _ => panic!("oh no"),
            })
        }).collect();
        let s: String = pgx::spi::Spi::get_one_with_args(query, prepared_args).unwrap();
        host::ValueResult::Str(s)
    }
}