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
unsafe fn guest_dealloc(ptr: u64, size: u64, align: u64) {
    let layout = std::alloc::Layout::from_size_align(size as usize, align as usize).unwrap();
    std::alloc::dealloc(ptr as *mut u8, layout)
}

#[no_mangle]
#[cfg(feature = "guest")]
unsafe fn guest_alloc(size: u64, align: u64) -> u64 {
    let layout = std::alloc::Layout::from_size_align(size as usize, align as usize).unwrap();
    std::alloc::alloc(layout) as u64
}

pub unsafe fn unpack_and_own_from_wasm<'d, T: serde::Deserialize<'d>>(ptr: u64, len: u64) -> Result<T, Box<bincode::ErrorKind>> {
    let bytes = std::slice::from_raw_parts(ptr as *mut u8, len as usize);
    bincode::deserialize(bytes)
}

pub unsafe fn serialize_and_leak_into_wasm<T: serde::Serialize>(val: &T) -> Result<(u64, u64), Box<bincode::ErrorKind>> {
    let bytes = bincode::serialize(val)?;
    Ok(pack_and_leak_into_wasm(bytes))
}

pub unsafe fn pack_and_leak_into_wasm(mut bytes: Vec<u8>) -> (u64, u64) {
    bytes.shrink_to(0); // This only shrinks capacity, and doesn't remove values.
    let len = bytes.len();
    assert_eq!(len, bytes.capacity(), "Despite being shrunk, the packed vector capacity and size differed");
    let ptr = bytes.as_mut_ptr();
    
    (ptr as u64, len as u64)
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