pub mod commands;
pub mod export;
pub mod git_context;
pub mod model;
pub mod notifier;
pub mod storage;
pub mod watcher;

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const INPUT_WINDOW_LABEL: &str = "input";
const LIST_WINDOW_LABEL: &str = "list";

/// Managed flag that is set when the user chooses "Beenden" from the tray menu,
/// so that `ExitRequested` can tell a real quit apart from the last window
/// being closed.
pub struct QuitRequested(pub AtomicBool);

fn show_window(app: &AppHandle, label: &str, url: &str, title: &str, width: f64, height: f64) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.show();
        let _ = window.set_focus();
    } else {
        let _ = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
            .title(title)
            .inner_size(width, height)
            .build();
    }
}

fn show_input_window(app: &AppHandle) {
    show_window(
        app,
        INPUT_WINDOW_LABEL,
        "input.html",
        "ParkPlatzCoder — Eingabe",
        460.0,
        180.0,
    );
}

fn show_list_window(app: &AppHandle) {
    show_window(
        app,
        LIST_WINDOW_LABEL,
        "list.html",
        "ParkPlatzCoder — Zettel",
        680.0,
        520.0,
    );
}

fn export_and_notify(app: &AppHandle) {
    match export::export_all_notes() {
        Ok(path) => {
            let _ = notifier::notify_export_done(app, &path);
        }
        Err(e) => {
            // Never log paths or note contents — the error here carries neither.
            log::warn!("export failed: {}", e);
        }
    }
}

pub fn run() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .manage(QuitRequested(AtomicBool::new(false)))
        .invoke_handler(tauri::generate_handler![
            commands::park_note,
            commands::get_notes,
            commands::search_notes,
            commands::toggle_done,
            commands::remove_note,
            commands::export_json,
        ])
        .setup(|app| {
            let icon = Image::from_bytes(include_bytes!("../icons/icon.png"))?;

            let open_input =
                MenuItem::with_id(app, "open_input", "Eingabe öffnen", true, None::<&str>)?;
            let open_list =
                MenuItem::with_id(app, "open_list", "Liste öffnen", true, None::<&str>)?;
            let export_item = MenuItem::with_id(app, "export", "Exportieren", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Beenden", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_input, &open_list, &export_item, &quit])?;

            TrayIconBuilder::new()
                .icon(icon)
                .tooltip("ParkPlatzCoder")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open_input" => show_input_window(app),
                    "open_list" => show_list_window(app),
                    "export" => export_and_notify(app),
                    "quit" => {
                        if let Some(state) = app.try_state::<QuitRequested>() {
                            state.0.store(true, Ordering::SeqCst);
                        }
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            app.global_shortcut()
                .on_shortcut("Ctrl+Alt+P", |app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        show_input_window(app);
                    }
                })?;

            let app_handle = app.handle().clone();
            if let Err(e) = watcher::start_watcher(app_handle) {
                // The watcher is a stub in this ticket; log the failure without
                // leaking any repo path or branch name.
                log::warn!("watcher failed to start: {}", e);
            }

            log::info!("ParkPlatzCoder started");
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            let quitting = app_handle
                .try_state::<QuitRequested>()
                .map(|s| s.0.load(Ordering::SeqCst))
                .unwrap_or(false);
            if !quitting {
                api.prevent_exit();
            }
        }
    });
}
