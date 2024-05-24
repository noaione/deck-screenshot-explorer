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
        let home = std::env::var("DECKY_HOME")
            .unwrap_or_else(|_| std::env::var("HOME").expect("HOME env var is not set"));
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

pub fn steamid64_to_steamid(steamid64: u64) -> u64 {
    let acct = steamid64 - ID64_IDENT;
    acct / 2
}

pub fn steamid64_to_usteamid(steamid64: u64) -> u64 {
    steamid64 - ID64_IDENT
}
