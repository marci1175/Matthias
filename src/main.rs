#![allow(non_snake_case)]
#![warn(clippy::all, rust_2018_idioms)]
#![feature(path_file_prefix)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
mod app;
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> eframe::Result<()> {
    //Ensure all temporary folders are deleted
    let _ = std::fs::remove_dir_all(format!("{}\\szeChat\\Client", env!("APPDATA")));
    let _ = std::fs::remove_dir_all(format!("{}\\szeChat\\Server", env!("APPDATA")));

    //Ensure main folders are existing
    let _ = std::fs::create_dir_all(format!("{}\\szeChat\\Client", env!("APPDATA")));
    let _ = std::fs::create_dir_all(format!("{}\\szeChat\\Server", env!("APPDATA")));

    use eframe::IconData;

    env_logger::init();

    let native_options = eframe::NativeOptions {
        initial_window_size: Some([400.0, 300.0].into()),
        min_window_size: Some([300.0, 220.0].into()),
        icon_data: Some(
            IconData::try_from_png_bytes(&include_bytes!("../icons/main.png")[..]).unwrap(),
        ),

        ..Default::default()
    };
    eframe::run_native(
        "széChat v3",
        native_options,
        Box::new(|cc| Box::new(szeChat::app::backend::TemplateApp::new(cc))),
    )
}
