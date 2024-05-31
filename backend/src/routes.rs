use std::{collections::HashMap, path::PathBuf};

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
    routing::get,
    Router,
};
use serde_json::json;

use crate::{
    models::{AppInfo, Pagination, User},
    steam::{get_steam_root_path, steamid64_to_steamid, steamid64_to_usteamid},
    vendor::vdfr::Value,
    SharedAppState,
};

pub async fn get_users(State(state): State<SharedAppState>) -> impl IntoResponse {
    let users = state
        .steam_users
        .keys()
        .map(|user_id| {
            let user = state.steam_users.get(user_id).unwrap();
            User {
                id: steamid64_to_steamid(*user_id),
                id3: steamid64_to_usteamid(*user_id),
                id64: *user_id,
                username: user.username.clone(),
                display_name: user.display_name.clone(),
                timestamp: user.timestamp,
            }
        })
        .collect::<Vec<User>>();

    let wrapped_json = json!({
        "ok": true,
        "data": users,
    });

    axum::Json(wrapped_json)
}

fn transform_vdfr_to_app(app: &crate::vendor::vdfr::App) -> AppInfo {
    let app_name = app.app_name().unwrap();

    let mut developers = Vec::new();
    let mut publishers = Vec::new();
    if let Some(Value::KeyValueType(associations)) = app.get(&["appinfo", "common", "associations"])
    {
        for value in associations.values() {
            if let Value::KeyValueType(kv) = value {
                if let Some(Value::StringType(name)) = kv.get("name") {
                    if let Some(Value::StringType(kind_type)) = kv.get("type") {
                        match kind_type.as_str() {
                            "developer" => {
                                developers.push(name.clone());
                            }
                            "publisher" => {
                                publishers.push(name.clone());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    let localized_name = app.localized_name();
    // get "english" name or fallback to app name
    let english_name = localized_name.get("english").unwrap_or(&app_name);

    AppInfo {
        id: app.id,
        name: english_name.clone(),
        localized_name: localized_name.clone(),
        developers: developers.clone(),
        publishers: publishers.clone(),
        non_steam: false,
    }
}

fn transform_shortcut_to_app(shortcut: &crate::steam::SteamShortcut) -> AppInfo {
    AppInfo {
        id: shortcut.id,
        name: shortcut.name.clone(),
        localized_name: HashMap::new(),
        developers: Vec::new(),
        publishers: Vec::new(),
        non_steam: true,
    }
}

pub async fn get_screenshot_apps(
    Path(id3): Path<u64>,
    State(state): State<SharedAppState>,
) -> impl IntoResponse {
    let steam_folder = dunce::canonicalize(get_steam_root_path()).unwrap();
    let user_folder = dunce::canonicalize(steam_folder.join(format!("userdata/{}", id3))).unwrap();

    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());

    tracing::debug!("[get_screenshot_apps] user ID3: {}", id3);
    tracing::debug!("[get_screenshot_apps] steam folder: {:?}", steam_folder);
    tracing::debug!("[get_screenshot_apps] user folder: {:?}", user_folder);

    // check if user_folder starts with steam_folder
    if !user_folder.starts_with(&steam_folder) {
        return (
            axum::http::StatusCode::FORBIDDEN,
            headers,
            serde_json::to_string(&json!({
                "ok": false,
                "error": "Invalid user id3 provided",
            }))
            .unwrap(),
        );
    }

    if !user_folder.exists() {
        return (
            axum::http::StatusCode::NOT_FOUND,
            headers,
            serde_json::to_string(&json!({
                "ok": false,
                "error": "User folder not found",
            }))
            .unwrap(),
        );
    }

    let screenshot_apps = user_folder.join("760/remote");
    if !screenshot_apps.exists() {
        return (
            axum::http::StatusCode::OK,
            headers,
            serde_json::to_string(&json!({
                "ok": true,
                "data": [],
            }))
            .unwrap(),
        );
    }

    let shortcuts_data = state.users_shortcuts.get(&id3).unwrap();

    // get all folders in the remote folder
    let apps = screenshot_apps
        .read_dir()
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                Some(entry.file_name())
            } else {
                None
            }
        })
        .filter_map(|app_id| {
            let app_id = app_id.to_string_lossy();
            let app_id = app_id.parse::<u32>();

            // fail to parse?
            if app_id.is_err() {
                return None;
            }

            let app_id = app_id.unwrap();

            if app_id == 7 {
                return Some(AppInfo {
                    id: app_id,
                    name: "Steam".to_string(),
                    ..Default::default()
                });
            }

            let app = state.app_info.apps.get(&app_id);

            if app.is_none() {
                let shortcut = shortcuts_data.get(&app_id);
                if let Some(shortcut) = shortcut {
                    return Some(transform_shortcut_to_app(shortcut));
                }
                return Some(AppInfo {
                    id: app_id,
                    name: format!("Unknown App {}", app_id),
                    ..Default::default()
                });
            }

            let app = app.unwrap();

            Some(transform_vdfr_to_app(app))
        })
        .collect::<Vec<AppInfo>>();

    let wrapped_json = json!({
        "ok": true,
        "data": apps,
    });

    (
        axum::http::StatusCode::OK,
        headers,
        serde_json::to_string(&wrapped_json).unwrap(),
    )
}

fn get_screenshot_folders(id3: u64, appid: u32) -> anyhow::Result<PathBuf> {
    let steam_folder = dunce::canonicalize(get_steam_root_path()).unwrap();
    let user_folder = dunce::canonicalize(steam_folder.join(format!("userdata/{}", id3)))?;

    tracing::debug!("[get_screenshot_folders] user ID3: {}", id3);
    tracing::debug!("[get_screenshot_folders] app ID: {}", appid);
    tracing::debug!("[get_screenshot_folders] steam folder: {:?}", steam_folder);
    tracing::debug!("[get_screenshot_folders] user folder: {:?}", user_folder);

    // check if user_folder starts with steam_folder
    if !user_folder.starts_with(&steam_folder) {
        anyhow::bail!("Invalid user id3 provided");
    }

    if !user_folder.exists() {
        anyhow::bail!("User folder not found");
    }

    let base_folder = user_folder.join("760/remote");
    tracing::debug!("[get_screenshot_folders] base folder: {:?}", base_folder);

    if !base_folder.exists() {
        anyhow::bail!("Screenshot folder not found");
    }

    let screenshots_folder =
        dunce::canonicalize(base_folder.join(format!("{}/screenshots", appid)))?;

    tracing::debug!(
        "[get_screenshot_folders] screenshots folder: {:?}",
        screenshots_folder
    );

    // check if screenshots_folder starts with steam_folder
    if !screenshots_folder.starts_with(&steam_folder) {
        anyhow::bail!("Invalid app ID provided");
    }

    Ok(screenshots_folder)
}

pub async fn get_screenshot_app(
    Path((id3, appid)): Path<(u64, u32)>,
    Query(pagination): Query<Pagination>,
    State(state): State<SharedAppState>,
) -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());

    let page = pagination.page.unwrap_or(0);
    let per_page = pagination.per_page.unwrap_or(10);

    // check if per_page is not 10, 20, 50, 100
    if ![10, 20, 50, 100].contains(&per_page) {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            headers,
            serde_json::to_string(&json!({
                "ok": false,
                "error": "Invalid per_page value (must be 10, 20, 50, or 100)",
            }))
            .unwrap(),
        );
    }

    let screenshots_folder = match get_screenshot_folders(id3, appid) {
        Ok(folder) => folder,
        Err(e) => {
            return (
                axum::http::StatusCode::FORBIDDEN,
                headers,
                serde_json::to_string(&json!({
                    "ok": false,
                    "error": e.to_string(),
                }))
                .unwrap(),
            );
        }
    };

    let shortcuts_data = state.users_shortcuts.get(&id3).unwrap();

    let app_info = match state.app_info.apps.get(&appid) {
        Some(app) => {
            if app.id == 7 {
                AppInfo {
                    id: app.id,
                    name: "Steam".to_string(),
                    ..Default::default()
                }
            } else {
                transform_vdfr_to_app(app)
            }
        }
        None => {
            if appid == 7 {
                AppInfo {
                    id: appid,
                    name: "Steam".to_string(),
                    ..Default::default()
                }
            } else {
                let shortcut = shortcuts_data.get(&appid);
                if let Some(shortcut) = shortcut {
                    transform_shortcut_to_app(shortcut)
                } else {
                    AppInfo {
                        id: appid,
                        name: format!("Unknown App {}", appid),
                        ..Default::default()
                    }
                }
            }
        }
    };

    if !screenshots_folder.exists() {
        return (
            axum::http::StatusCode::OK,
            headers,
            serde_json::to_string(&json!({
                "ok": true,
                "data": {
                    "app": app_info,
                    "screenshots": [],
                    "pagination": {
                        "total": 0,
                        "page": page,
                        "per_page": per_page,
                    }
                },
            }))
            .unwrap(),
        );
    }

    let mut screenshots: Vec<PathBuf> = screenshots_folder
        .read_dir()
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                // check if ext is not jpg, png, or webp
                let ext = entry
                    .path()
                    .extension()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                if !["jpg", "png", "webp"].contains(&ext.as_str()) {
                    return None;
                }
                Some(entry.path())
            } else {
                None
            }
        })
        .collect();

    // sort by filename
    screenshots.sort_by(|a, b| a.file_stem().cmp(&b.file_stem()));
    let total_ss = screenshots.len();
    // take only the required page
    let screenshot_files: Vec<String> = screenshots
        .into_iter()
        .skip(page * per_page)
        .take(per_page)
        .map(|path| path.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    (
        axum::http::StatusCode::OK,
        headers,
        serde_json::to_string(&json!({
            "ok": true,
            "data": {
                "app": app_info,
                "screenshots": screenshot_files,
                "pagination": {
                    "total": total_ss,
                    "page": page,
                    "per_page": per_page,
                }
            },
        }))
        .unwrap(),
    )
}

pub async fn get_screenshot_file(
    Path((id3, appid, filename)): Path<(u64, u32, String)>,
) -> axum::response::Response {
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());

    let steam_folders = dunce::canonicalize(get_steam_root_path()).unwrap();

    let screenshots_folder = match get_screenshot_folders(id3, appid) {
        Ok(folder) => folder,
        Err(e) => {
            return (
                axum::http::StatusCode::FORBIDDEN,
                headers,
                serde_json::to_string(&json!({
                    "ok": false,
                    "error": e.to_string(),
                }))
                .unwrap(),
            )
                .into_response();
        }
    };

    // get file
    let file_path = dunce::canonicalize(screenshots_folder.join(filename.clone())).unwrap();
    if !file_path.starts_with(&steam_folders) {
        return (
            axum::http::StatusCode::FORBIDDEN,
            headers,
            serde_json::to_string(&json!({
                "ok": false,
                "error": "Invalid filename",
            }))
            .unwrap(),
        )
            .into_response();
    }

    if !file_path.exists() {
        let mut text_headers = HeaderMap::new();
        text_headers.insert("Content-Type", "text/plain".parse().unwrap());
        return (
            axum::http::StatusCode::NOT_FOUND,
            text_headers,
            "File not found".to_string(),
        )
            .into_response();
    }

    let mimetype = mime_guess::from_path(&file_path)
        .first_or_octet_stream()
        .to_string();

    let file_fs = tokio::fs::File::open(file_path).await.unwrap();
    let stream = tokio_util::io::ReaderStream::new(file_fs);
    let body = Body::from_stream(stream);

    let file_headers = {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", mimetype.parse().unwrap());
        headers.insert(
            "Content-Disposition",
            format!("inline; filename={}", filename).parse().unwrap(),
        );
        headers
    };

    (axum::http::StatusCode::OK, file_headers, body).into_response()
}

pub async fn get_screenshot_file_thumbnail(
    Path((id3, appid, filename)): Path<(u64, u32, String)>,
) -> axum::response::Response {
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());

    let steam_folders = dunce::canonicalize(get_steam_root_path()).unwrap();

    let screenshots_folder = match get_screenshot_folders(id3, appid) {
        Ok(folder) => folder,
        Err(e) => {
            return (
                axum::http::StatusCode::FORBIDDEN,
                headers,
                serde_json::to_string(&json!({
                    "ok": false,
                    "error": e.to_string(),
                }))
                .unwrap(),
            )
                .into_response();
        }
    };

    // get file and change to jpg
    let file_path =
        dunce::canonicalize(screenshots_folder.join(format!("thumbnails/{}", filename)))
            .unwrap()
            .with_extension("jpg");
    if !file_path.starts_with(&steam_folders) {
        return (
            axum::http::StatusCode::FORBIDDEN,
            headers,
            serde_json::to_string(&json!({
                "ok": false,
                "error": "Invalid filename",
            }))
            .unwrap(),
        )
            .into_response();
    }

    if !file_path.exists() {
        let mut text_headers = HeaderMap::new();
        text_headers.insert("Content-Type", "text/plain".parse().unwrap());
        return (
            axum::http::StatusCode::NOT_FOUND,
            text_headers,
            "File not found".to_string(),
        )
            .into_response();
    }

    let file_fs = tokio::fs::File::open(file_path).await.unwrap();
    let stream = tokio_util::io::ReaderStream::new(file_fs);
    let body = Body::from_stream(stream);

    let file_headers = {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "image/jpeg".parse().unwrap());
        headers
    };

    (axum::http::StatusCode::OK, file_headers, body).into_response()
}

pub fn api_routes(state: SharedAppState) -> Router<SharedAppState> {
    Router::new()
        .route("/users", get(get_users))
        .route("/users/:id3", get(get_screenshot_apps))
        .route("/users/:id3/:appid", get(get_screenshot_app))
        .route("/users/:id3/:appid/:filename", get(get_screenshot_file))
        .route(
            "/users/:id3/:appid/t/:filename",
            get(get_screenshot_file_thumbnail),
        )
        .with_state(state)
}
