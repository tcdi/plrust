
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

#[no_mangle]
#[cfg(feature = "guest")]
unsafe extern "C" fn guest_dealloc(ptr: u64, size: u64, align: u64) {
    let layout = std::alloc::Layout::from_size_align(size as usize, align as usize).unwrap();
    std::alloc::dealloc(ptr as *mut u8, layout)
}

#[no_mangle]
#[cfg(feature = "guest")]
unsafe extern "C" fn guest_alloc(size: u64, align: u64) -> u64 {
    let layout = std::alloc::Layout::from_size_align(size as usize, align as usize).unwrap();
    std::alloc::alloc(layout) as u64
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