pub mod commands;
pub mod indexer;
pub mod markdown;
pub mod models;
pub mod paths;
pub mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::pages::set_vault,
            commands::pages::current_vault,
            commands::pages::read_page_tree,
            commands::pages::read_page,
            commands::pages::write_page,
            commands::pages::create_page,
            commands::pages::delete_page,
            commands::pages::rename_page,
            commands::pages::move_page,
            commands::pages::reorder_siblings,
            commands::pages::render_markdown,
            commands::ink::read_ink,
            commands::ink::write_ink,
            commands::search::search,
            commands::search::rebuild_index,
            commands::assets::import_asset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
