use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug)]
pub struct User {
    pub id: u64,
    pub id3: u64,
    pub id64: u64,
    pub username: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub timestamp: u64,
}

#[derive(Serialize, Debug, Default)]
pub struct AppInfo {
    pub id: u32,
    pub name: String,
    pub localized_name: HashMap<String, String>,
    pub developers: Vec<String>,
    pub publishers: Vec<String>,
    pub non_steam: bool,
}

#[derive(Deserialize)]
pub struct Pagination {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}
