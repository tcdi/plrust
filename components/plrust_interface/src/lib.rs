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

pub unsafe fn serialize_pack_and_leak<T: serde::Serialize>(val: &T) -> Result<u64, Box<bincode::ErrorKind>> {
    let retval = leak_into_wasm(
        pack_with_len(
            serialize(val)?
        )
    );
    Ok(retval)
}

pub unsafe fn own_unpack_and_deserialize<T: serde::de::DeserializeOwned>(ptr: *mut u8) -> Result<T, Box<bincode::ErrorKind>> {
    let (_unpacked_len, unpacked_bytes) = unpack(ptr).unwrap();
    let retval = deserialize(&unpacked_bytes).unwrap();
    Ok(retval)
}

pub unsafe fn get_packed_len(ptr: *mut u8) -> Result<u64, std::array::TryFromSliceError> {
    let packed_len_bytes = std::slice::from_raw_parts(ptr, std::mem::size_of::<u64>());
    let len = u64::from_le_bytes(packed_len_bytes.try_into()?);
    Ok(len)
}

pub unsafe fn unpack(ptr: *mut u8) -> Result<(u64, Vec<u8>), Box<dyn std::error::Error>> {
    let packed_len = get_packed_len(ptr)?;
    let packed_len_with_packing = packed_len as usize + std::mem::size_of::<u64>();
    let mut buf = Vec::from_raw_parts(ptr, packed_len_with_packing, packed_len_with_packing);
    let bytes = buf.split_off(std::mem::size_of::<u64>());
    Ok((packed_len, bytes))
}

pub fn deserialize<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, Box<bincode::ErrorKind>> {
    bincode::deserialize(bytes)
}

pub fn serialize<T: serde::Serialize>(val: &T) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
    bincode::serialize(val)
}

pub fn pack_with_len(mut bytes: Vec<u8>) -> Vec<u8> {
    let bytes_len = bytes.len() as u64; // This cast is extremely load bearing, don't trim it.
    let mut packed_bytes = bytes_len.to_le_bytes().to_vec();
    packed_bytes.append(&mut bytes);
    assert_eq!(
        u64::from_le_bytes(packed_bytes[0..std::mem::size_of::<u64>()].try_into().unwrap()),
        bytes_len as u64,
        "Packed len and real len mismatch",
    );
    packed_bytes
}

pub unsafe fn leak_into_wasm(mut bytes: Vec<u8>) -> u64 {
    bytes.shrink_to(0); // This only shrinks capacity, and doesn't remove values.
    assert_eq!(
        bytes.len(),
        bytes.capacity(),
        "Despite being shrunk, the packed vector capacity and size differed",
    );
    
    let ptr = bytes.as_mut_ptr();
    std::mem::forget(bytes);
    
    ptr as u64
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

#[test]
fn round_trip_string() -> Result<(), Box<dyn std::error::Error>> {
    let data = String::from("Nami");
    let leaked_ptr = unsafe { serialize_pack_and_leak(&data).unwrap() };
    let reowned: String = unsafe { own_unpack_and_deserialize(leaked_ptr as *mut u8).unwrap() };
    assert_eq!(data, reowned);
    Ok(())
}

#[test]
fn pack_and_unpack_1() -> Result<(), Box<dyn std::error::Error>> {
    let val = vec![1];
    let mut packed = pack_with_len(val.clone());
    assert_eq!(packed.len(), 9);
    let packed_ptr = packed.as_mut_ptr();
    std::mem::forget(packed);
    let packed_len = unsafe { get_packed_len(packed_ptr)? };
    assert_eq!(packed_len, 1);
    let (unpacked_len, unpacked) = unsafe { unpack(packed_ptr)? };
    assert_eq!(unpacked_len, 1);
    assert_eq!(unpacked, val);

    Ok(())
}

#[test]
fn pack_and_unpack_4() -> Result<(), Box<dyn std::error::Error>> {
    let val = vec![1, 2, 3, 4];
    let mut packed = pack_with_len(val.clone());
    assert_eq!(packed.len(), 12);
    let packed_ptr = packed.as_mut_ptr();
    std::mem::forget(packed);
    let packed_len = unsafe { get_packed_len(packed_ptr)? };
    assert_eq!(packed_len, 4);
    let (unpacked_len, unpacked) = unsafe { unpack(packed_ptr)? };
    assert_eq!(unpacked_len, 4);
    assert_eq!(unpacked, val);

    Ok(())
}

#[test]
fn pack_and_unpack_string() -> Result<(), Box<dyn std::error::Error>> {
    let val = String::from("Nami");
    let val_encoded = serialize(&val)?;

    let mut packed = pack_with_len(val_encoded.clone());
    assert_eq!(packed.len(), 20);
    let packed_ptr = packed.as_mut_ptr();
    std::mem::forget(packed);
    let packed_len = unsafe { get_packed_len(packed_ptr)? };
    assert_eq!(packed_len, 12);
    let (unpacked_len, unpacked) = unsafe { unpack(packed_ptr)? };
    assert_eq!(unpacked_len, 12);
    assert_eq!(unpacked, val_encoded);

    let decoded: String = deserialize(&unpacked)?;
    assert_eq!(decoded, val);

    Ok(())
}

#[test]
fn headers() -> Result<(), Box<dyn std::error::Error>> {
    let s = String::from("Nami");

    let s_bytes = serialize(&s)?;
    assert_eq!(
        s_bytes.as_slice(),
        &[
            4, 0, 0, 0, 0, 0, 0, 0, // The bincode header
            78, 97, 109, 105
        ]
    );

    let packed = pack_with_len(s_bytes.clone());
    assert_eq!(
        packed.as_slice(),
        &[
            12, 0, 0, 0, 0, 0, 0, 0, // Our packed length
            4, 0, 0, 0, 0, 0, 0, 0, // The bincode header
            78, 97, 109, 105
        ]
    );

    Ok(())
}