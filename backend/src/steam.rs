// get steam root path
// on steam deck and other linux, the root path is ~/.steam/root
// on windows, the root path is C:\Program Files (x86)\Steam

use std::{collections::HashMap, path::PathBuf};

use serde::Deserialize;

const ID64_IDENT: u64 = 76561197960265728;

pub fn get_steam_root_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        PathBuf::from("C:\\Program Files (x86)\\Steam")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").expect("HOME env var is not set");
        PathBuf::from(format!("{}/.steam/root", home))
    }
}

/// A minimal representation of a Steam user.
#[derive(Clone, Debug, Deserialize)]
pub struct LoginUser {
    #[serde(rename = "AccountName")]
    pub username: String,
    #[serde(rename = "PersonaName")]
    pub display_name: String,
    #[serde(rename = "Timestamp")]
    pub timestamp: u64,
}

pub fn get_steam_users(root_path: PathBuf) -> HashMap<u64, LoginUser> {
    let login_users_path = root_path.join("config/loginusers.vdf");

    if !login_users_path.exists() {
        // return an empty hashmap if the file doesn't exist
        return HashMap::new();
    }

    let mut login_users_reader = std::fs::File::open(login_users_path).unwrap();
    let login_users: HashMap<String, LoginUser> =
        keyvalues_serde::from_reader(&mut login_users_reader).unwrap();

    let transformed_users = login_users
        .into_iter()
        .map(|(k, v)| (k.parse().unwrap(), v))
        .collect();

    transformed_users
}

/// A minimal representation of a Steam shortcut.
#[derive(Clone, Debug)]
pub struct SteamShortcut {
    pub id: u32,
    pub name: String,
}

pub fn load_users_shortcuts(user_id: u64) -> HashMap<u32, SteamShortcut> {
    let shortcuts_path =
        get_steam_root_path().join(format!("userdata/{}/config/shortcuts.vdf", user_id));

    tracing::info!("Loading shortcuts from {:?}", shortcuts_path);

    if !shortcuts_path.exists() {
        return HashMap::new();
    }

    let mut shortcuts_reader = std::fs::read(shortcuts_path).unwrap();

    match vdfr::parser::parse_keyvalues(&mut shortcuts_reader) {
        Ok(kv) => {
            let shortcuts = kv.get("shortcuts");

            match shortcuts {
                Some(vdfr::Value::KeyValueType(shortcuts)) => {
                    let mapped: HashMap<u32, SteamShortcut> = shortcuts
                        .values()
                        .filter_map(|shortcut| {
                            if let vdfr::Value::KeyValueType(shortcut) = shortcut {
                                let id = shortcut.get("appid");
                                if let Some(vdfr::Value::Int32Type(id)) = id {
                                    let name = shortcut.get("AppName");
                                    let actual_id = clamp_i32_to_u24(*id);
                                    match name {
                                        Some(vdfr::Value::StringType(name)) => Some((
                                            actual_id,
                                            SteamShortcut {
                                                id: actual_id,
                                                name: name.clone(),
                                            },
                                        )),
                                        Some(vdfr::Value::WideStringType(name)) => Some((
                                            actual_id,
                                            SteamShortcut {
                                                id: actual_id,
                                                name: name.clone(),
                                            },
                                        )),
                                        _ => None,
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .collect();

                    mapped
                }
                _ => HashMap::new(),
            }
        }
        Err(_) => HashMap::new(),
    }
}

pub fn get_app_name(app: &vdfr::App) -> String {
    let name = app.get(&["appinfo", "common", "name"]);
    match name {
        Some(vdfr::Value::StringType(name)) => name.clone(),
        Some(vdfr::Value::WideStringType(name)) => name.clone(),
        _ => format!("Steam App {}", app.id),
    }
}

pub fn get_localized_app_name(app: &vdfr::App) -> HashMap<String, String> {
    let mut names = HashMap::new();
    let localized = app.get(&["appinfo", "common", "name_localized"]);
    if let Some(vdfr::Value::KeyValueType(kv)) = localized {
        for (k, v) in kv.iter() {
            match v {
                vdfr::Value::StringType(v) => {
                    names.insert(k.clone(), v.clone());
                }
                vdfr::Value::WideStringType(v) => {
                    names.insert(k.clone(), v.clone());
                }
                _ => {}
            }
        }
    }
    names
}

pub fn steamid64_to_steamid(steamid64: u64) -> u64 {
    let acct = steamid64 - ID64_IDENT;
    acct / 2
}

pub fn steamid64_to_usteamid(steamid64: u64) -> u64 {
    steamid64 - ID64_IDENT
}

pub fn clamp_i32_to_u24(value: i32) -> u32 {
    (value as u32) & 0x00FF_FFFF
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_clamp_works() {
        assert_eq!(super::clamp_i32_to_u24(-1195449660), 12509892);
    }
}
