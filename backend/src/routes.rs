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
use vdfr::Value;

use crate::{
    models::{AppInfo, Pagination, User},
    steam::{
        get_app_name, get_localized_app_name, get_steam_root_path, steamid64_to_steamid,
        steamid64_to_usteamid,
    },
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

fn transform_vdfr_to_app(app: &vdfr::App) -> AppInfo {
    let app_name = get_app_name(app);

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

    let localized_name = get_localized_app_name(app);
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

async fn try_check_path_dir(path: &PathBuf, folder_name: &str) -> Result<bool, String> {
    match tokio::fs::try_exists(path).await {
        // Pass the data
        Ok(exists) => Ok(exists),
        // Pass the error
        Err(io_error) => match io_error.kind() {
            std::io::ErrorKind::NotFound => Ok(false),
            other => {
                let error_message = format!("Error checking {}: {}", folder_name, other);
                tracing::error!("{}", &error_message);
                Err(error_message)
            }
        },
    }
}

fn make_error(error: &str) -> String {
    serde_json::to_string(&json!({
        "ok": false,
        "error": error,
    }))
    .unwrap()
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

    match try_check_path_dir(&user_folder, "User folder").await {
        Ok(false) => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                headers,
                serde_json::to_string(&json!({
                    "ok": false,
                    "error": "User folder not found",
                }))
                .unwrap(),
            )
        }
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                headers,
                make_error(&e),
            )
        }
        _ => (),
    }

    let screenshot_apps = user_folder.join("760/remote");

    match try_check_path_dir(&screenshot_apps, "Screenshot folder").await {
        Ok(false) => {
            return (
                axum::http::StatusCode::OK,
                headers,
                serde_json::to_string(&json!({
                    "ok": true,
                    "data": [],
                }))
                .unwrap(),
            )
        }
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                headers,
                make_error(&e),
            )
        }
        _ => (),
    }

    let shortcuts_data = state.users_shortcuts.get(&id3).unwrap();

    // get all folders in the remote folder
    let mut entries = match tokio::fs::read_dir(&screenshot_apps).await {
        Ok(entries) => entries,
        Err(io_error) => match io_error.kind() {
            std::io::ErrorKind::NotFound => {
                return (
                    axum::http::StatusCode::OK,
                    headers,
                    serde_json::to_string(&json!({
                        "ok": true,
                        "data": [],
                    }))
                    .unwrap(),
                )
            }
            other => {
                let error_msg = format!("Failed to read screenshot folder directory: {}", other);
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    headers,
                    serde_json::to_string(&json!({
                        "ok": false,
                        "error": error_msg,
                    }))
                    .unwrap(),
                );
            }
        },
    };

    let mut app_entries = Vec::new();

    loop {
        let entry = match entries.next_entry().await {
            Ok(Some(entry)) => entry,
            Ok(None) => break, // No more entries
            Err(io_error) => match io_error.kind() {
                std::io::ErrorKind::NotFound => {
                    return (
                        axum::http::StatusCode::OK,
                        headers,
                        serde_json::to_string(&json!({
                            "ok": true,
                            "data": [],
                        }))
                        .unwrap(),
                    )
                }
                other => {
                    let error_msg =
                        format!("Failed to get next entry for screenshot folder: {}", other);
                    return (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        headers,
                        serde_json::to_string(&json!({
                            "ok": false,
                            "error": error_msg,
                        }))
                        .unwrap(),
                    );
                }
            },
        };

        let file_type = match entry.file_type().await {
            Ok(ft) => ft,
            Err(_) => continue, // Ignore errors
        };

        if file_type.is_dir() {
            let filename = entry.file_name();
            let app_id = filename.to_string_lossy();
            let app_id = app_id.parse::<u32>();
            match app_id {
                Ok(app_id) => {
                    if app_id == 7 {
                        app_entries.push(AppInfo {
                            id: app_id,
                            name: "Steam".to_string(),
                            ..Default::default()
                        })
                    } else {
                        match state.app_info.apps.get(&app_id) {
                            Some(app) => {
                                app_entries.push(transform_vdfr_to_app(app));
                            }
                            None => match shortcuts_data.get(&app_id) {
                                Some(shortcut) => {
                                    app_entries.push(transform_shortcut_to_app(shortcut));
                                }
                                None => {
                                    app_entries.push(AppInfo {
                                        id: app_id,
                                        name: format!("Unknown App {}", app_id),
                                        ..Default::default()
                                    });
                                }
                            },
                        }
                    }
                }
                Err(_) => (), // ignore
            }
        }
    }

    let wrapped_json = json!({
        "ok": true,
        "data": app_entries,
    });

    (
        axum::http::StatusCode::OK,
        headers,
        serde_json::to_string(&wrapped_json).unwrap(),
    )
}

async fn get_screenshot_folders(id3: u64, appid: u32) -> anyhow::Result<PathBuf> {
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

    match tokio::fs::try_exists(&user_folder).await {
        Ok(exists) => {
            if !exists {
                anyhow::bail!("User folder not found");
            }
        }
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => {
                anyhow::bail!("User folder not found");
            }
            other => {
                anyhow::bail!("Failed to check if user folder exists: {}", other);
            }
        },
    }

    let base_folder = user_folder.join("760/remote");
    tracing::debug!("[get_screenshot_folders] base folder: {:?}", base_folder);

    match tokio::fs::try_exists(&base_folder).await {
        Ok(exists) => {
            if !exists {
                anyhow::bail!("Screenshot folder not found");
            }
        }
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => {
                anyhow::bail!("Screenshot folder not found");
            }
            other => {
                anyhow::bail!("Failed to check if screenshot folder exists: {}", other);
            }
        },
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

    // match tokio::fs::try_exists(&screenshots_folder).await {
    //     Ok(exists) => {
    //         if !exists {
    //             anyhow::bail!("App screenshot folder not found");
    //         }
    //     }
    //     Err(e) => match e.kind() {
    //         std::io::ErrorKind::NotFound => {
    //             anyhow::bail!("App screenshot folder not found");
    //         }
    //         other => {
    //             anyhow::bail!("Failed to check if app screenshot folder exists: {}", other);
    //         }
    //     },
    // }

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

    let screenshots_folder = match get_screenshot_folders(id3, appid).await {
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

    let default_err_data = json!({
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
    });

    match try_check_path_dir(&screenshots_folder, "App screenshot folder").await {
        Ok(false) => {
            return (
                axum::http::StatusCode::OK,
                headers,
                serde_json::to_string(&default_err_data).unwrap(),
            );
        }
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                headers,
                make_error(&e),
            );
        }
        _ => (),
    }

    // get all folders in the remote folder
    let mut entries = match tokio::fs::read_dir(&screenshots_folder).await {
        Ok(entries) => entries,
        Err(io_error) => match io_error.kind() {
            std::io::ErrorKind::NotFound => {
                return (
                    axum::http::StatusCode::OK,
                    headers,
                    serde_json::to_string(&default_err_data).unwrap(),
                )
            }
            other => {
                let error_msg = format!("Failed to read screenshot folder directory: {}", other);
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    headers,
                    serde_json::to_string(&json!({
                        "ok": false,
                        "error": error_msg,
                    }))
                    .unwrap(),
                );
            }
        },
    };

    let mut screenshot_data: Vec<PathBuf> = Vec::new();

    loop {
        let entry = match entries.next_entry().await {
            Ok(Some(entry)) => entry,
            Ok(None) => break, // No more entries
            Err(io_error) => match io_error.kind() {
                std::io::ErrorKind::NotFound => {
                    return (
                        axum::http::StatusCode::OK,
                        headers,
                        serde_json::to_string(&default_err_data).unwrap(),
                    )
                }
                other => {
                    let error_msg = format!(
                        "Failed to get next entry for app screenshot folder: {}",
                        other
                    );
                    return (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        headers,
                        serde_json::to_string(&json!({
                            "ok": false,
                            "error": error_msg,
                        }))
                        .unwrap(),
                    );
                }
            },
        };

        let file_type = match entry.file_type().await {
            Ok(ft) => ft,
            Err(_) => continue, // Ignore errors
        };

        if file_type.is_file() {
            match entry.path().extension() {
                Some(file_ext) => {
                    let ext_clean = file_ext.to_string_lossy().to_string();
                    if ["jpg", "png", "webp"].contains(&ext_clean.as_str()) {
                        screenshot_data.push(entry.path());
                    }
                }
                _ => (),
            }
        }
    }

    // sort by filename
    screenshot_data.sort_by(|a, b| a.file_stem().cmp(&b.file_stem()));
    let total_ss = screenshot_data.len();
    // take only the required page
    let screenshot_files: Vec<String> = screenshot_data
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

    let screenshots_folder = match get_screenshot_folders(id3, appid).await {
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

    let mimetype = mime_guess::from_path(&file_path)
        .first_or_octet_stream()
        .to_string();

    let file_fs = match tokio::fs::File::open(file_path).await {
        Ok(file) => file,
        Err(error) => {
            let mut text_headers = HeaderMap::new();
            text_headers.insert("Content-Type", "text/plain".parse().unwrap());
            match error.kind() {
                std::io::ErrorKind::NotFound => {
                    return (
                        axum::http::StatusCode::NOT_FOUND,
                        text_headers,
                        "File not found".to_string(),
                    )
                        .into_response();
                }
                _ => {
                    return (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        text_headers,
                        format!("Failed to open file: {}", error),
                    )
                        .into_response();
                }
            }
        }
    };
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

    let screenshots_folder = match get_screenshot_folders(id3, appid).await {
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

    let file_fs = match tokio::fs::File::open(file_path).await {
        Ok(file) => file,
        Err(error) => {
            let mut text_headers = HeaderMap::new();
            text_headers.insert("Content-Type", "text/plain".parse().unwrap());
            match error.kind() {
                std::io::ErrorKind::NotFound => {
                    return (
                        axum::http::StatusCode::NOT_FOUND,
                        text_headers,
                        "Thumbnail not found".to_string(),
                    )
                        .into_response();
                }
                _ => {
                    return (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        text_headers,
                        format!("Failed to open thumbnail: {}", error),
                    )
                        .into_response();
                }
            }
        }
    };
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
        .route("/users/{id3}", get(get_screenshot_apps))
        .route("/users/{id3}/{appid}", get(get_screenshot_app))
        .route("/users/{id3}/{appid}/{filename}", get(get_screenshot_file))
        .route(
            "/users/{id3}/{appid}/t/{filename}",
            get(get_screenshot_file_thumbnail),
        )
        .with_state(state)
}
