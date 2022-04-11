#[cfg(feature = "host")]
use std::mem::size_of;

use serde::{Serialize, Deserialize};

#[cfg(feature = "host")]
pub fn create_linker_functions(linker: &mut wasmtime::Linker<wasmtime_wasi::WasiCtx>) -> Result<(), wasmtime_wasi::Error> {
    linker.func_wrap("plrust_interface", "unsafe_get_one_with_args", crate::host_get_one_with_args)?;
    Ok(())
}

pub trait WasmArgOrReturnId {
    const ID: u64;
}

impl WasmArgOrReturnId for String {
    const ID: u64 = 0;
}

#[derive(Serialize, Deserialize, Debug)]
pub enum WasmArgOrReturn {
     String(String),
     I32(i32)
}

impl From<WasmArgOrReturn> for String {
    fn from(val: WasmArgOrReturn) -> Self {
        match val {
            WasmArgOrReturn::String(s) => s,
            _ => todo!(),
        }
    }
}

impl From<String> for WasmArgOrReturn {
    fn from(val: String) -> Self {
        WasmArgOrReturn::String(val)
    }
}

impl<'a> From<&'a str> for WasmArgOrReturn {
    fn from(val: &'a str) -> Self {
        WasmArgOrReturn::String(val.to_string())
    }
}

impl From<i32> for WasmArgOrReturn {
    fn from(val: i32) -> Self {
        WasmArgOrReturn::I32(val)
    }
}

impl WasmArgOrReturn {
//     unsafe fn own_and_unpack(ptr: *mut u8) -> Result<Self, Box<bincode::ErrorKind>> {
//         let len_bytes = std::slice::from_raw_parts(ptr, std::mem::size_of::<u64>());
//         let len = u64::from_le_bytes(len_bytes.try_into().unwrap());

//         let buf_bytes = std::slice::from_raw_parts(ptr, len as usize + std::mem::size_of::<u64>());
//         Self::unpack(buf_bytes)
//     }

//     fn unpack(bytes: &[u8]) -> Result<Self, Box<bincode::ErrorKind>> {
//         let _unpacked_len = u64::from_le_bytes(bytes[0..size_of::<u64>()].try_into().unwrap());
        
//         let buf = &bytes[size_of::<u64>()..];

//         deserialize(buf)
//     }

//     unsafe fn pack_and_leak(&self) -> Result<*mut u8, Box<bincode::ErrorKind>> {
//         let packed = self.pack()?;
//         Ok(leak_into_wasm(packed))
//     }
    
//     fn pack(&self) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
//         let mut serialized = serialize(self)?;

//         let mut with_len = serialized.len().to_le_bytes().to_vec();
//         with_len.append(&mut serialized);

//         Ok(with_len)
//     }

    #[cfg(feature = "host")]
    fn to_oid_and_datum(self) -> (pgx::PgOid, Option<pgx::pg_sys::Datum>) {
        match self {
            WasmArgOrReturn::String(v) => (pgx::pg_sys::PgBuiltInOids::TEXTOID.oid(), pgx::IntoDatum::into_datum(v)),
            WasmArgOrReturn::I32(v) => (pgx::pg_sys::PgBuiltInOids::INT8OID.oid(), pgx::IntoDatum::into_datum(v)),
        }
    }
}

// start spi_exec_select_num

#[cfg(feature = "guest")]
#[link(wasm_import_module = "plrust_interface")]
extern "C" {
    fn unsafe_get_one_with_args(query_packed_ptr: i64, args_packed_ptr: i64, return_type_id: u64) -> u64;
}

// The function signature must match `unsafe_get_one_with_args`
#[cfg(feature = "host")]
pub fn host_get_one_with_args(mut caller: wasmtime::Caller<'_, wasmtime_wasi::WasiCtx>, query_packed_ptr: i64, args_packed_ptr: i64, return_type_id: u64) -> i64 {
    use pgx::{PgOid, pg_sys::{Oid, Datum}, FromDatum, IntoDatum};

    let store = caller.data_mut();

    let mem = match caller.get_export("memory").unwrap() {
        wasmtime::Extern::Memory(mem) => mem,
        _ => todo!(),
    };

    let mut query: String = {
        let mut query_len_bytes = [0_u8; 8];
        mem.read(&mut caller, query_packed_ptr as usize, &mut query_len_bytes).unwrap();
        let query_bytes_len = u64::from_le_bytes(query_len_bytes);
        
        let mut query_bytes = vec![0_u8; query_bytes_len as usize];
        mem.read(&mut caller, query_packed_ptr as usize + size_of::<u64>(), &mut query_bytes).unwrap();
        deserialize(&query_bytes).unwrap()
    };
    pgx::warning!("Got query: {:?}", query);

    let mut args: Vec<WasmArgOrReturn> = {
        let mut args_len_bytes = [0_u8; 8];
        mem.read(&mut caller, args_packed_ptr as usize, &mut args_len_bytes).unwrap();
        let args_bytes_len = u64::from_le_bytes(args_len_bytes);
        
        let mut args_bytes = vec![0_u8; args_bytes_len as usize];
        mem.read(&mut caller, args_packed_ptr as usize + size_of::<u64>(), &mut args_bytes).unwrap();
        deserialize(&args_bytes).unwrap()
    };
    pgx::warning!("Got args: {:?}", args);

    pgx::warning!("Expecting return type id of: {:?}", return_type_id);
    let serialized = match return_type_id {
        0 => {
            pgx::warning!("Returns: String");
            let retval: Option<String> = pgx::Spi::get_one_with_args(
                &query, 
                args.into_iter().map(|arg| arg.to_oid_and_datum()).collect(),
            );
        
            pgx::warning!("Serializing: {:?}", retval);
            serialize(&WasmArgOrReturn::String(retval.unwrap())).unwrap()
        },
        _ => todo!(),
    };

    let packed = pack_with_len(serialized);

    let wasm_alloc = match caller.get_export("guest_alloc").unwrap() {
        wasmtime::Extern::Func(func) => func.typed::<(u64, u64), u64, _>(&mut caller).unwrap(),
        _ => todo!(),
    };

    let guest_ptr = wasm_alloc.call(&mut caller, (packed.len() as u64, 8)).unwrap();

    mem.write(&mut caller, guest_ptr as usize, packed.as_slice()).unwrap();

    guest_ptr as i64
}

#[cfg(feature = "guest")]
pub fn get_one_with_args<R: From<WasmArgOrReturn> + WasmArgOrReturnId>(query: &str, args: Vec<WasmArgOrReturn>) -> R  {
    let query_packed = unsafe { serialize_pack_and_leak(&query).unwrap() };
    let args_packed = unsafe { serialize_pack_and_leak(&args).unwrap() };
    
    let retval_ptr = unsafe { unsafe_get_one_with_args(query_packed as i64, args_packed as i64, <R as WasmArgOrReturnId>::ID) as *mut u8 };

    let retval: WasmArgOrReturn = unsafe { own_unpack_and_deserialize(retval_ptr).unwrap() };
    R::from(retval)
}


// end spi_exec_select_num

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

#[cfg(feature = "host")]
fn serialize_by_type_id(datum: pgx::pg_sys::Datum, type_id: u64) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
    pub use pgx::{PgOid, pg_sys::{Oid, Datum}, FromDatum, IntoDatum};

    match type_id {
        0 => {
            let val = unsafe { String::from_datum(datum, false, pgx::pg_sys::PgBuiltInOids::TEXTOID.value()).unwrap() };
            pgx::warning!("Serializing: {:?}", val);
            serialize(&WasmArgOrReturn::String(val))
        },
        _ => todo!(),
    }
}

pub unsafe fn serialize_pack_and_leak<T: serde::Serialize>(val: &T) -> Result<*mut u8, Box<bincode::ErrorKind>> {
    let retval = leak_into_wasm(
        pack_with_len(
            serialize(val)?
        )
    );
    Ok(retval)
}

pub unsafe fn own_unpack_and_deserialize<T: serde::de::DeserializeOwned>(ptr: *mut u8) -> Result<T, Box<bincode::ErrorKind>> {
    let (_unpacked_len, unpacked_bytes) = unpack(ptr).unwrap();
    let retval = deserialize(&unpacked_bytes)?;
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

pub unsafe fn leak_into_wasm(mut bytes: Vec<u8>) -> *mut u8 {
    bytes.shrink_to(0); // This only shrinks capacity, and doesn't remove values.
    assert_eq!(
        bytes.len(),
        bytes.capacity(),
        "Despite being shrunk, the packed vector capacity and size differed",
    );
    
    let ptr = bytes.as_mut_ptr();
    std::mem::forget(bytes);
    
    ptr
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