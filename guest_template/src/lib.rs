// User code is templated into this code when a user's PL/Rust function is created.
wit_bindgen_rust::export!("../components/wit/guest.wit");

mod impls;
struct Guest;

// This code exists as a smoke test so this template can be built outside of a running PL/Rust. It gets replaced.
mod smoke_test {
    use super::*;

    impl guest::Guest for Guest {
        #[allow(unused_variables, unused_mut)] // In case of zero args.
        fn entry(
            mut args: Vec<Option<guest::Value>>,
        ) -> Result<Option<guest::Value>, guest::Error> {
            let retval = dummy_user_fn(args.pop().unwrap().map(|v| v.try_into()).transpose()?)?;
            Ok(retval.map(|v| v.into()))
        }
    }

    fn dummy_user_fn(a: Option<i32>) -> Result<Option<i32>, guest::Error> {
        Ok(a)
    }
}
