//! Valve Data Format (also known as Key-Values) Reader
//!
//! This is heavily modified version from https://github.com/drguildo/vdfr
//! Originally written using byteorder, this implementation use nom for parsing.

use std::collections::HashMap;

use nom::{
    bytes::complete::{take, take_until},
    multi::{count, many0},
    number::complete::{be_u16, le_f32, le_i32, le_i64, le_u16, le_u32, le_u64, le_u8},
    sequence::tuple,
    IResult,
};

const BIN_NONE: u8 = b'\x00';
const BIN_STRING: u8 = b'\x01';
const BIN_INT32: u8 = b'\x02';
const BIN_FLOAT32: u8 = b'\x03';
const BIN_POINTER: u8 = b'\x04';
const BIN_WIDESTRING: u8 = b'\x05';
const BIN_COLOR: u8 = b'\x06';
const BIN_UINT64: u8 = b'\x07';
const BIN_END: u8 = b'\x08';
const BIN_INT64: u8 = b'\x0A';
const BIN_END_ALT: u8 = b'\x0B';

// Before Dec 2022
const MAGIC_27: u32 = 0x07_56_44_27;
// Before June 2024, added checksum_bin
const MAGIC_28: u32 = 0x07_56_44_28;
// Latest, storage optimization with string pools
const MAGIC_29: u32 = 0x07_56_44_29;

#[derive(Debug)]
pub enum VdfrError {
    InvalidType(u8),
    ReadError(std::io::Error),
    UnknownMagic(u32),
    NomError(String),
    InvalidStringIndex(usize, usize),
}

impl std::error::Error for VdfrError {}

impl std::fmt::Display for VdfrError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            VdfrError::InvalidType(t) => write!(f, "Invalid type {:#x}", t),
            VdfrError::UnknownMagic(v) => write!(f, "Unknown magic {:#x}", v),
            VdfrError::InvalidStringIndex(c, t) => {
                write!(f, "Invalid string index {} (total {})", c, t)
            }
            VdfrError::ReadError(e) => e.fmt(f),
            VdfrError::NomError(e) => write!(f, "Nom error: {}", e),
        }
    }
}

impl From<std::io::Error> for VdfrError {
    fn from(e: std::io::Error) -> Self {
        VdfrError::ReadError(e)
    }
}

pub enum Value {
    StringType(String),
    WideStringType(String),
    Int32Type(i32),
    PointerType(i32),
    ColorType(i32),
    UInt64Type(u64),
    Int64Type(i64),
    Float32Type(f32),
    KeyValueType(KeyValue),
}

fn fmt_string(s: &str) -> String {
    // escape quotes and backslashes
    let mut escaped = String::new();
    for c in s.chars() {
        match c {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            _ => escaped.push(c),
        }
    }
    escaped
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::StringType(s) => write!(f, "\"{}\"", fmt_string(s)),
            Value::WideStringType(s) => write!(f, "W\"{}\"", fmt_string(s)),
            Value::Int32Type(i) => write!(f, "{}", i),
            Value::PointerType(i) => write!(f, "\"*{}\"", i),
            Value::ColorType(i) => write!(f, "{}", i),
            Value::UInt64Type(i) => write!(f, "{}", i),
            Value::Int64Type(i) => write!(f, "{}", i),
            Value::Float32Type(i) => write!(f, "{}", i),
            Value::KeyValueType(kv) => write!(f, "{:?}", kv),
        }
    }
}

fn throw_error(error: nom::Err<nom::error::Error<&[u8]>>) -> VdfrError {
    // clone the error to avoid lifetime issues
    match error {
        nom::Err::Error(e) | nom::Err::Failure(e) => {
            // get like 64 bytes of data to show in the error message
            let data = e.input;
            let data = if data.len() > 64 { &data[..64] } else { data };

            VdfrError::NomError(format!("Error: {:?}, data: {:?}", e.code, data))
        }
        nom::Err::Incomplete(e) => VdfrError::NomError(format!("Incomplete data, need: {:?}", e)),
    }
}

type KeyValue = HashMap<String, Value>;

/// Options for reading key-value data.
#[derive(Debug, Clone, Default)]
pub struct KeyValueOptions {
    pub string_pool: Vec<String>,
    pub alt_format: bool,
}

// Recursively search for the specified sequence of keys in the key-value data.
// The order of the keys dictates the hierarchy, with all except the last having
// to be a Value::KeyValueType.
fn find_keys<'a>(kv: &'a KeyValue, keys: &[&str]) -> Option<&'a Value> {
    if keys.len() == 0 {
        return None;
    }

    let key = keys.first().unwrap();
    let value = kv.get(&key.to_string());
    if keys.len() == 1 {
        value
    } else {
        if let Some(Value::KeyValueType(kv)) = value {
            find_keys(&kv, &keys[1..])
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct App {
    pub id: u32,
    pub size: u32,
    pub state: u32,
    pub last_update: u32,
    pub access_token: u64,
    pub checksum_txt: [u8; 20],
    pub checksum_bin: Option<[u8; 20]>,
    pub change_number: u32,
    pub key_values: KeyValue,
}

#[derive(Debug)]
pub struct AppInfo {
    pub version: u32,
    pub universe: u32,
    pub apps: HashMap<u32, App>,
}

impl AppInfo {
    pub fn load(data: &[u8]) -> Result<AppInfo, VdfrError> {
        let (data, (version, universe)) = tuple((le_u32, le_u32))(data).map_err(throw_error)?;

        let (payloads, options) = match version {
            MAGIC_27 | MAGIC_28 => (data, KeyValueOptions::default()),
            MAGIC_29 => {
                let (data, offset) = le_i64(data).map_err(throw_error)?;

                // Use nom to jump to offset_table and read the string pool
                // data is the remaining data after reading version, universe, and offset.
                // to ensure we actually jump to the offset, we need to subtract the amount of data read so far.
                let read_amount = 4usize + 4 + 8;
                let offset_actual = (offset as usize) - read_amount;
                // Left side, is the remainder which is the string pools, while payload is the actual data.
                let (string_pools, payload) = take(offset_actual)(data).map_err(throw_error)?;
                let (string_pools, count) = le_u32(string_pools).map_err(throw_error)?;

                let (_, string_pool) =
                    read_string_pools(string_pools, count as usize).map_err(throw_error)?;

                (
                    payload,
                    KeyValueOptions {
                        string_pool,
                        alt_format: false,
                    },
                )
            }
            _ => return Err(VdfrError::UnknownMagic(version)),
        };

        let (_, mut apps) = parse_apps(payloads, &options, version).map_err(throw_error)?;

        // Pop app 0
        apps.remove(&0);

        Ok(AppInfo {
            version,
            universe,
            apps,
        })
    }
}

impl App {
    pub fn get(&self, keys: &[&str]) -> Option<&Value> {
        find_keys(&self.key_values, keys)
    }

    /// Get the name of the app.
    pub fn app_name(&self) -> Option<String> {
        let name = self.get(&["appinfo", "common", "name"]);
        match name {
            Some(Value::StringType(name)) => Some(name.clone()),
            Some(Value::WideStringType(name)) => Some(name.clone()),
            _ => None,
        }
    }

    /// Get localized name
    pub fn localized_name(&self) -> HashMap<String, String> {
        let mut names = HashMap::new();
        let localized = self.get(&["appinfo", "common", "name_localized"]);
        if let Some(Value::KeyValueType(kv)) = localized {
            for (k, v) in kv.iter() {
                match v {
                    Value::StringType(v) => {
                        names.insert(k.clone(), v.clone());
                    }
                    Value::WideStringType(v) => {
                        names.insert(k.clone(), v.clone());
                    }
                    _ => {}
                }
            }
        }
        names
    }
}

fn parse_apps<'a>(
    data: &'a [u8],
    options: &'a KeyValueOptions,
    version: u32,
) -> IResult<&'a [u8], HashMap<u32, App>> {
    let (rest, apps) = many0(|d| parse_app(d, options, version))(data)?;

    let hash_apps: HashMap<u32, App> = apps.into_iter().map(|app| (app.id, app)).collect();

    Ok((rest, hash_apps))
}

fn parse_app<'a>(
    data: &'a [u8],
    options: &'a KeyValueOptions,
    version: u32,
) -> IResult<&'a [u8], App> {
    let (data, app_id) = le_u32(data)?;

    if app_id == 0 {
        // End of apps, return empty app
        Ok((
            data,
            App {
                id: 0,
                size: 0,
                state: 0,
                last_update: 0,
                access_token: 0,
                checksum_txt: [0; 20],
                checksum_bin: Some([0; 20]),
                change_number: 0,
                key_values: HashMap::new(),
            },
        ))
    } else {
        let (data, (size, state, last_update, access_token)) =
            tuple((le_u32, le_u32, le_u32, le_u64))(data)?;

        let (data, checksum_txt) = take(20usize)(data)?;
        let (data, change_number) = le_u32(data)?;
        let (data, checksum_bin) = match version {
            MAGIC_27 => {
                // we skip checksum_bin
                (data, None)
            }
            _ => {
                let (data, checksum_bin) = take(20usize)(data)?;
                (data, Some(checksum_bin.try_into().unwrap()))
            }
        };

        let (data, key_values) = parse_bytes_kv(data, options)?;

        Ok((
            data,
            App {
                id: app_id,
                size,
                state,
                last_update,
                access_token,
                checksum_txt: checksum_txt.try_into().unwrap(),
                checksum_bin,
                change_number,
                key_values,
            },
        ))
    }
}

pub fn parse_keyvalues(data: &[u8]) -> Result<KeyValue, VdfrError> {
    let (_, key_values) = parse_bytes_kv(data, &KeyValueOptions::default()).map_err(throw_error)?;
    Ok(key_values)
}

fn parse_bytes_kv<'a>(data: &'a [u8], options: &'a KeyValueOptions) -> IResult<&'a [u8], KeyValue> {
    let bin_end = if options.alt_format {
        BIN_END_ALT
    } else {
        BIN_END
    };

    let mut node = KeyValue::new();

    let mut data = data;
    loop {
        let (res, bin) = le_u8(data)?;

        if bin == bin_end {
            return Ok((res, node));
        }

        let (res, key) = if options.string_pool.is_empty() {
            parse_utf8(res)?
        } else {
            let (res, index) = le_u32(res)?;
            let index = index as usize;
            if index >= options.string_pool.len() {
                return Err(nom::Err::Error(nom::error::Error::new(
                    res,
                    nom::error::ErrorKind::Eof,
                )));
            }
            (res, options.string_pool[index].clone())
        };

        let (res, value) = match bin {
            BIN_NONE => {
                let (res, subnode) = parse_bytes_kv(res, options)?;
                (res, Value::KeyValueType(subnode))
            }
            BIN_STRING => {
                let (res, value) = parse_utf8(res)?;
                (res, Value::StringType(value))
            }
            BIN_WIDESTRING => {
                let (res, value) = parse_utf16(res)?;
                (res, Value::WideStringType(value))
            }
            BIN_INT32 | BIN_POINTER | BIN_COLOR => {
                let (res, value) = le_i32(res)?;
                let value = match bin {
                    BIN_INT32 => Value::Int32Type(value),
                    BIN_POINTER => Value::PointerType(value),
                    BIN_COLOR => Value::ColorType(value),
                    _ => unreachable!(),
                };
                (res, value)
            }
            BIN_UINT64 => {
                let (res, value) = le_u64(res)?;
                (res, Value::UInt64Type(value))
            }
            BIN_INT64 => {
                let (res, value) = le_i64(res)?;
                (res, Value::Int64Type(value))
            }
            BIN_FLOAT32 => {
                let (res, value) = le_f32(res)?;
                (res, Value::Float32Type(value))
            }
            _ => {
                return Err(nom::Err::Error(nom::error::Error::new(
                    res,
                    nom::error::ErrorKind::Char,
                )));
            }
        };

        node.insert(key, value);
        data = res;
    }
}

fn read_string_pools(data: &[u8], amount: usize) -> IResult<&[u8], Vec<String>> {
    count(parse_utf8, amount)(data)
}

fn parse_utf8(input: &[u8]) -> IResult<&[u8], String> {
    // Parse until NULL byte
    let (rest, buf) = take_until("\0")(input)?;
    let (rest, _) = le_u8(rest)?; // Skip NULL byte
    let s = std::str::from_utf8(buf)
        .map_err(|_| nom::Err::Error(nom::error::Error::new(rest, nom::error::ErrorKind::Char)))?;
    Ok((rest, s.to_string()))
}

enum Endian {
    Be,
    Le,
}

fn parse_utf16(input: &[u8]) -> IResult<&[u8], String> {
    // Parse until NULL byte
    let (rest, buf) = take_until("\0\0")(input)?;
    // Check if BOM is preset, if not assume BE
    let (buf, bom) = if buf.len() >= 2 {
        // Has BOM, check if LE or BE
        let big_endian = buf[0] == 0xFE && buf[1] == 0xFF;
        let little_endian = buf[0] == 0xFF && buf[1] == 0xFE;

        match (big_endian, little_endian) {
            // If BE/LE, skip BOM bytes and set endianness
            (true, false) => (&buf[2..], Endian::Be),
            (false, true) => (&buf[2..], Endian::Le),
            _ => (buf, Endian::Be),
        }
    } else {
        // No BOM, assume BE
        (buf, Endian::Be)
    };

    // Consume NULL byte
    let (rest, _) = match bom {
        Endian::Be => be_u16(rest)?,
        Endian::Le => le_u16(rest)?,
    };

    let mut v: Vec<u16> = vec![];
    for i in 0..buf.len() / 2 {
        let temp_buf = [buf[i * 2], buf[i * 2 + 1]];
        let c = match bom {
            Endian::Be => u16::from_be_bytes(temp_buf),
            Endian::Le => u16::from_le_bytes(temp_buf),
        };
        v.push(c);
    }
    v.push(0); // Add NULL terminator
    let s = std::string::String::from_utf16_lossy(&v);
    Ok((rest, s))
}
