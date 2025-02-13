use std::{thread::{self, sleep}, time::Duration};
use tauri::Emitter;

use serde::Serialize;
use tauri::App;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NewPicture {
    image_src: String,
    location: String,
    time_taken: String,
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn setup(app: &mut App) -> Result<(), Box<(dyn std::error::Error + 'static)>> {
    let app_handle = app.handle().clone();

    thread::spawn(move || loop {
        app_handle.emit("new-picture", NewPicture {
            image_src: "data:image/jpeg;base64,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".to_string(),
            location: "location".to_string(),
            time_taken: "time".to_string(),
        }).unwrap();
        sleep(Duration::from_secs(10));
    });
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(setup)
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
