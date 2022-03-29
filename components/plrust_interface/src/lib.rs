
#[cfg(feature = "host")]
pub fn create_linker_functions(linker: &mut wasmtime::Linker<wasmtime_wasi::WasiCtx>) -> Result<(), wasmtime_wasi::Error> {
    linker.func_wrap("plrust_interface", "unsafe_spi_exec_select_num", crate::spi_exec_select_num)?;
    Ok(())
}


#[cfg(feature = "guest")]
#[link(wasm_import_module = "plrust_interface")]
extern "C" {
    fn unsafe_spi_exec_select_num(i: i32) -> i32; // This simply does "SELECT i"
}

#[cfg(feature = "guest")]
pub fn spi_exec_select_num(i: i32) -> i32 {
    unsafe { unsafe_spi_exec_select_num(i) }
}

#[cfg(feature = "host")]
pub fn spi_exec_select_num(i: i32) -> i32 {
    match pgx::Spi::get_one(format!("SELECT {}", i).as_str()) {
        Some(res) => {
            return res;
        }
        None => {
            pgx::warning!("Spi::get_one returned nothing, returning default value");
            return -1;
        }
    }
}