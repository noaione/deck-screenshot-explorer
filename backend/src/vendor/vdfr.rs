//! Valve Data Format (also known as Key-Values) Reader
//!
//! This is vendored from https://github.com/drguildo/vdfr
//! with some modifications.

use std::{collections::HashMap, io::Error};

use byteorder::{LittleEndian, ReadBytesExt};

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
const MAGIC_28: u32 = 0x07_56_44_28;
const MAGIC_29: u32 = 0x07_56_44_29;

#[derive(Debug)]
pub enum VdfrError {
    InvalidType(u8),
    ReadError(std::io::Error),
    UnknownMagic(u32),
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
    pub checksum_bin: [u8; 20],
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
    pub fn load<R: std::io::Read + std::io::Seek>(reader: &mut R) -> Result<AppInfo, VdfrError> {
        let version = reader.read_u32::<LittleEndian>()?;

        if version != MAGIC_28 && version != MAGIC_29 {
            return Err(VdfrError::UnknownMagic(version));
        }
        let universe = reader.read_u32::<LittleEndian>()?;

        let mut options = KeyValueOptions::default();

        if version == MAGIC_29 {
            let offset_table = reader.read_i64::<LittleEndian>()?;
            let old_offset = reader.stream_position()?.clone();
            reader.seek(std::io::SeekFrom::Start(offset_table as u64))?;
            let string_count = reader.read_u32::<LittleEndian>()?;
            options.string_pool = (0..string_count)
                .map(|_| read_string(reader, false).unwrap())
                .collect();
            reader.seek(std::io::SeekFrom::Start(old_offset))?;
        }

        let mut appinfo = AppInfo {
            universe,
            version,
            apps: HashMap::new(),
        };

        loop {
            let app_id = reader.read_u32::<LittleEndian>()?;
            if app_id == 0 {
                break;
            }

            let size = reader.read_u32::<LittleEndian>()?;
            let state = reader.read_u32::<LittleEndian>()?;
            let last_update = reader.read_u32::<LittleEndian>()?;
            let access_token = reader.read_u64::<LittleEndian>()?;

            let mut checksum_txt: [u8; 20] = [0; 20];
            reader.read_exact(&mut checksum_txt)?;

            let change_number = reader.read_u32::<LittleEndian>()?;

            let mut checksum_bin: [u8; 20] = [0; 20];
            reader.read_exact(&mut checksum_bin)?;

            let key_values = read_kv(reader, options.clone())?;

            let app = App {
                id: app_id,
                size,
                state,
                last_update,
                access_token,
                checksum_txt,
                checksum_bin,
                change_number,
                key_values,
            };
            appinfo.apps.insert(app_id, app);
        }

        Ok(appinfo)
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

/// Read a "binary" key-value data structure from the reader.
pub fn read_kv<R: std::io::Read>(
    reader: &mut R,
    options: KeyValueOptions,
) -> Result<KeyValue, VdfrError> {
    let current_bin_end = if options.alt_format {
        BIN_END_ALT
    } else {
        BIN_END
    };

    let mut node = KeyValue::new();

    loop {
        let t = reader.read_u8()?;
        if t == current_bin_end {
            return Ok(node);
        }

        let key = if options.string_pool.is_empty() {
            read_string(reader, false)?
        } else {
            let idx = reader.read_u32::<LittleEndian>()? as usize;
            options
                .string_pool
                .get(idx)
                .ok_or(VdfrError::InvalidStringIndex(
                    idx,
                    options.string_pool.len(),
                ))?
                .clone()
        };

        if t == BIN_NONE {
            let subnode = read_kv(reader, options.clone())?;
            node.insert(key, Value::KeyValueType(subnode));
        } else if t == BIN_STRING {
            let s = read_string(reader, false)?;
            node.insert(key, Value::StringType(s));
        } else if t == BIN_WIDESTRING {
            let s = read_string(reader, true)?;
            node.insert(key, Value::WideStringType(s));
        } else if [BIN_INT32, BIN_POINTER, BIN_COLOR].contains(&t) {
            let val = reader.read_i32::<LittleEndian>()?;
            if t == BIN_INT32 {
                node.insert(key, Value::Int32Type(val));
            } else if t == BIN_POINTER {
                node.insert(key, Value::PointerType(val));
            } else if t == BIN_COLOR {
                node.insert(key, Value::ColorType(val));
            }
        } else if t == BIN_UINT64 {
            let val = reader.read_u64::<LittleEndian>()?;
            node.insert(key, Value::UInt64Type(val));
        } else if t == BIN_INT64 {
            let val = reader.read_i64::<LittleEndian>()?;
            node.insert(key, Value::Int64Type(val));
        } else if t == BIN_FLOAT32 {
            let val = reader.read_f32::<LittleEndian>()?;
            node.insert(key, Value::Float32Type(val));
        } else {
            return Err(VdfrError::InvalidType(t));
        }
    }
}

fn read_string<R: std::io::Read>(reader: &mut R, wide: bool) -> Result<String, Error> {
    if wide {
        let mut buf: Vec<u16> = vec![];
        loop {
            // Maybe this should be big-endian?
            let c = reader.read_u16::<LittleEndian>()?;
            if c == 0 {
                break;
            }
            buf.push(c);
        }
        return Ok(std::string::String::from_utf16_lossy(&buf).to_string());
    } else {
        let mut buf: Vec<u8> = vec![];
        loop {
            let c = reader.read_u8()?;
            if c == 0 {
                break;
            }
            buf.push(c);
        }
        return Ok(std::string::String::from_utf8_lossy(&buf).to_string());
    }
}
