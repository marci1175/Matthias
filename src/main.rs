#![allow(non_snake_case)]
#![warn(clippy::all, rust_2018_idioms)]
#![feature(path_file_prefix)]
#![feature(cursor_remaining)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
mod app;

use std::env::args;

use egui::ViewportBuilder;
use egui::{Style, Visuals};
use tokio::fs;
use windows_sys::{
    w,
    Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR},
};
#[tokio::main]
async fn main() -> eframe::Result<()> {
    //Get args
    let args: Vec<String> = args().collect();



    #[cfg(not(debug_assertions))]
    env_logger::init();

    #[cfg(debug_assertions)]
    console_subscriber::init();

    #[cfg(debug_assertions)]
    std::env::set_var("RUST_BACKTRACE", "1");

    //set custom panic hook
    #[cfg(not(debug_assertions))]
    std::panic::set_hook(Box::new(|info| {
        let appdata_path = std::env!("APPDATA");
        // Write error message
        std::fs::write(
            format!("{appdata_path}/matthias/error.log"),
            format!(
                "[DATE]\n{:?}\n[PANIC]\n{:?}\n[STACK_BACKTRACE]\n{}\n",
                chrono::Local::now(),
                info.to_string(),
                std::backtrace::Backtrace::force_capture().to_string()
            ),
        )
        .unwrap();

        //Display error message
        display_panic_message(format!("A panic! has occured the error is logged in %appdata%. Please send the generated file or this message to the developer!\nPanic: \n{:?}\nLocation: \n{:?}", {
            match info.payload().downcast_ref::<&str>() {
                Some(msg) => msg,
                None => {
                    match info.payload().downcast_ref::<String>() {
                        Some(msg) => msg,
                        None => "Failed to display panic message",
                    }
                },
            }
        }, info.location()));
    }));

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

    let _ = fs::create_dir(format!("{}\\matthias\\extensions", env!("APPDATA"))).await;

    eframe::run_native(
        "Matthias",
        native_options,
        Box::new(|cc| {
            //Set app style
            cc.egui_ctx.set_style(Style {
                visuals: Visuals::dark(),
                ..Default::default()
            });

            //Load image loaders
            egui_extras::install_image_loaders(&cc.egui_ctx);

            let mut application = matthias::app::backend::Application::new(cc);

            //Check if there are any custom startup args
            if args.len() > 1 {
                //Modify args
                application.startup_args = Some(args);
            }

            //Create application
            Ok(Box::new(application))
        }),
    )
}

pub fn display_panic_message<T>(display: T)
where
    T: ToString + std::marker::Send + 'static,
{
    unsafe {
        MessageBoxW(
            0,
            str::encode_utf16(display.to_string().as_str())
                .chain(std::iter::once(0))
                .collect::<Vec<_>>()
                .as_ptr(),
            w!("Panic!"),
            MB_ICONERROR,
        )
    };
}
/*  Gulyásleves recept

    Heat the oil or lard in a large pot (preferably a Dutch oven). Add the onions along with a few spoonfuls of water (so they don’t brown) and a pinch of the salt. Cook slowly over very low heat for about 15 to 20 minutes, or until the onions are clear and glassy.
    Remove from the heat and add the paprika, pepper, and caraway seeds. Stir quickly to combine and add a tiny bit of water (to prevent the paprika from burning).
    Add the meat and garlic and cook over high heat, stirring, until the meat is slightly browned (about ten minutes). Turn the heat down to low, add a few spoonfuls of water, and cook for about 15 more minutes, until the meat is nearly cooked through.
    Add the rest of the water and keep cooking, over low heat, for at least an hour, or until the meat is cooked and nearly tender enough to serve. This could take hours, depending on the cut of beef you used.
    When the meat is nearly done, add the tomatoes, carrots, parsnips, and potatoes and cook for about 15 more minutes, or until they are tender (being careful not to overcook them). Taste the soup and add more salt and pepper, if needed.
    If you are using csipetke or another kind of small pasta, add it to the soup before serving. You can serve this soup with hot pepper or hot pepper paste.

*/
