#![warn(clippy::all, rust_2018_idioms)]
#![allow(non_snake_case)]
#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

pub mod app;
use std::env::args;

use app::backend::Application;
use eframe::NativeOptions;
use egui::{Style, ViewportBuilder, Visuals};

#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[tokio::main]
#[cfg(target_os = "android")]
#[no_mangle]
async fn android_main(app: AndroidApp)
{
    use winit::platform::android::EventLoopBuilderExtAndroid;

    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Debug),
    );

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder {
            ..Default::default()
        },
        event_loop_builder: Some(Box::new(move |builder| {
            builder.with_android_app(app);
        })),
        ..Default::default()
    };

    application_main(native_options, args).unwrap_or_else(|err| {
        log::error!("Failure while running EFrame application: {err:?}");
    });
}

pub fn application_main(native_options: NativeOptions, args: Vec<String>) -> Result<(), eframe::Error> {
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

            let mut application = Application::new(cc);

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

#[cfg(not(target_os = "android"))]
fn main() -> Result<(), eframe::Error> {
    //Get args
    let args: Vec<String> = args().collect();

    env_logger::builder()
        .filter_level(log::LevelFilter::Warn) // Default Log Level
        .parse_default_env()
        .init();

    #[cfg(debug_assertions)]
    std::env::set_var("RUST_BACKTRACE", "1");

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder {
            ..Default::default()
        },
        ..Default::default()
    };

    application_main(native_options, args)
}


/*  Gulyásleves recept

    Heat the oil or lard in a large pot (preferably a Dutch oven). Add the onions along with a few spoonfuls of water (so they don’t brown) and a pinch of the salt. Cook slowly over very low heat for about 15 to 20 minutes, or until the onions are clear and glassy.
    Remove from the heat and add the paprika, pepper, and caraway seeds. Stir quickly to combine and add a tiny bit of water (to prevent the paprika from burning).
    Add the meat and garlic and cook over high heat, stirring, until the meat is slightly browned (about ten minutes). Turn the heat down to low, add a few spoonfuls of water, and cook for about 15 more minutes, until the meat is nearly cooked through.
    Add the rest of the water and keep cooking, over low heat, for at least an hour, or until the meat is cooked and nearly tender enough to serve. This could take hours, depending on the cut of beef you used.
    When the meat is nearly done, add the tomatoes, carrots, parsnips, and potatoes and cook for about 15 more minutes, or until they are tender (being careful not to overcook them). Taste the soup and add more salt and pepper, if needed.
    If you are using csipetke or another kind of small pasta, add it to the soup before serving. You can serve this soup with hot pepper or hot pepper paste.

*/
