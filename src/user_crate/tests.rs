
#[cfg(any(test, feature = "pg_test"))]
#[pgx::pg_schema]
mod tests {
    use pgx::*;

    use crate::user_crate::*;
    use eyre::WrapErr;
    use quote::quote;
    use syn::parse_quote;
    use toml::toml;

    #[pg_test]
    fn function_workflow() {
        fn wrapped() -> eyre::Result<()> {
            let fn_oid = 0 as pg_sys::Oid;
            let target_dir = crate::gucs::work_dir();
            let pg_config = PathBuf::from(crate::gucs::pg_config());

            let variant = {
                let argument_oids_and_names =
                    vec![(PgOid::from(PgBuiltInOids::TEXTOID.value()), None)];
                let return_oid = PgOid::from(PgBuiltInOids::TEXTOID.value());
                let is_strict = true;
                let return_set = false;
                CrateVariant::function(argument_oids_and_names, return_oid, return_set, is_strict)?
            };
            let user_deps = toml::value::Table::default();
            let user_code = syn::parse2(quote! {
                { Some(arg0.to_string()) }
            })?;

            let generated =
                UserCrate::generated_for_tests(fn_oid, user_deps, user_code, variant);

            let generated_lib_rs = generated.lib_rs()?;
            let fixture_lib_rs = parse_quote! {
                use pgx::*;
                #[pg_extern]
                fn plrust_fn_oid_0(arg0: &str) -> Option<String> {
                    Some(arg0.to_string())
                }
            };
            assert_eq!(
                generated_lib_rs,
                fixture_lib_rs,
                "Generated `lib.rs` differs from test (output formatted)\n\nGenerated:\n{}\nFixture:\n{}\n",
                prettyplease::unparse(&generated_lib_rs),
                prettyplease::unparse(&fixture_lib_rs)
            );

            let generated_cargo_toml = generated.cargo_toml()?;
            let fixture_cargo_toml = toml! {
                [package]
                edition = "2021"
                name = "plrust_fn_oid_0"
                version = "0.0.0"

                [features]
                default = ["pgx/pg14"]

                [lib]
                crate-type = ["cdylib"]

                [dependencies]
                pgx = "0.4.3"

                [profile.release]
                codegen-units = 1_usize
                lto = "fat"
                opt-level = 3_usize
                panic = "unwind"
            };
            assert_eq!(
                generated_cargo_toml,
                *fixture_cargo_toml.as_table().unwrap(),
                "Generated `Cargo.toml` differs from test (output formatted)\n\nGenerated:\n{}\nFixture:\n{}\n",
                toml::to_string(&generated_cargo_toml)?,
                toml::to_string(&fixture_cargo_toml)?,
            );

            let parent_dir = tempdir::TempDir::new("plrust-generated-crate-function-workflow")
                .wrap_err("Creating temp dir")?;
            let provisioned = generated.provision(parent_dir.path())?;

            let built =
                provisioned.build(parent_dir.path(), pg_config, Some(target_dir.as_path()))?;

            let _shared_object = built.shared_object();

            Ok(())
        }
        wrapped().unwrap()
    }
}
