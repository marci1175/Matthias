#![allow(non_snake_case)]
#![warn(clippy::all, rust_2018_idioms)]
#![feature(path_file_prefix)]
#![feature(cursor_remaining)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
mod app;
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> eframe::Result<()> {
    // use crate::app::backend::display_toast_notification;
    // display_toast_notification();

    //Ensure all temporary folders are deleted

    use egui::ViewportBuilder;

    let _ = std::fs::remove_dir_all(format!("{}\\Matthias\\Client", env!("APPDATA")));
    let _ = std::fs::remove_dir_all(format!("{}\\Matthias\\Server", env!("APPDATA")));

    //Ensure main folders are existing
    let _ = std::fs::create_dir_all(format!("{}\\Matthias\\Client", env!("APPDATA")));
    let _ = std::fs::create_dir_all(format!("{}\\Matthias\\Server", env!("APPDATA")));

    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder {
            icon: Some(std::sync::Arc::new(egui::IconData {
                rgba: image::load_from_memory(include_bytes!("../icons/main.png"))
                    .unwrap()
                    .to_rgba8()
                    .to_vec(),
                width: 1024,
                height: 1024,
            })),
            ..Default::default()
        },
        ..Default::default()
    };

    eframe::run_native(
        "Matthias",
        native_options,
        Box::new(|cc| Box::new(Matthias::app::backend::TemplateApp::new(cc))),
    )
}

/*  Gulyásleves recept

    Heat the oil or lard in a large pot (preferably a Dutch oven). Add the onions along with a few spoonfuls of water (so they don’t brown) and a pinch of the salt. Cook slowly over very low heat for about 15 to 20 minutes, or until the onions are clear and glassy.
    Remove from the heat and add the paprika, pepper, and caraway seeds. Stir quickly to combine and add a tiny bit of water (to prevent the paprika from burning).
    Add the meat and garlic and cook over high heat, stirring, until the meat is slightly browned (about ten minutes). Turn the heat down to low, add a few spoonfuls of water, and cook for about 15 more minutes, until the meat is nearly cooked through.
    Add the rest of the water and keep cooking, over low heat, for at least an hour, or until the meat is cooked and nearly tender enough to serve. This could take hours, depending on the cut of beef you used.
    When the meat is nearly done, add the tomatoes, carrots, parsnips, and potatoes and cook for about 15 more minutes, or until they are tender (being careful not to overcook them). Taste the soup and add more salt and pepper, if needed.
    If you are using csipetke or another kind of small pasta, add it to the soup before serving. You can serve this soup with hot pepper or hot pepper paste.

*/
