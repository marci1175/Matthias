use egui::{vec2, Align, Layout, RichText};

use std::fs::{self};

mod account_manager;
pub mod backend;

mod client;
mod input;
mod networking;
mod server;
mod ui;

use self::account_manager::{
    append_to_file, decrypt_lines_from_vec, delete_line_from_file, read_from_file,
};

use self::backend::ServerMaster;
use self::input::keymap;

impl eframe::App for backend::TemplateApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn on_close_event(&mut self) -> bool {
        //clean up after server, client
        match std::env::var("APPDATA") {
            Ok(app_data) => {
                if let Err(err) = fs::remove_dir_all(format!("{}\\szeChat\\Server", app_data)) {
                    println!("{err}");
                };
                if let Err(err) = fs::remove_dir_all(format!("{}\\szeChat\\Client", app_data)) {
                    println!("{err}");
                };
            }
            Err(err) => println!("{err}"),
        }

        true
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let input_keys = keymap(self.keymap.clone());

        /*

            :: custom font ::

        let mut custom_font = FontDefinitions::default();

        custom_font.font_data.insert("consola".to_owned(), FontData::from_static(include_bytes!("C:\\Windows\\fonts\\arial.ttf")));

        custom_font.families.get_mut(&FontFamily::Proportional).unwrap()
            .insert(0, "consola".to_owned());

        // Put my font as last fallback for monospace:
        custom_font.families.get_mut(&FontFamily::Monospace).unwrap()
            .push("consola".to_owned());

        ctx.set_fonts(custom_font);

        */

        /*devlog:

        TODO: MAKE MESSAGE WIDGET BETTER

        */

        //For image loading
        egui_extras::install_image_loaders(ctx);
        //Login Page
        if !(self.mode_selector || self.server_mode || self.client_mode) {
            self.state_login(_frame, ctx, &input_keys);
        }

        //Main page
        if self.mode_selector && !(self.server_mode || self.client_mode) {
            self.state_mode_selection(_frame, ctx);
        }

        //Server page
        if self.server_mode {
            self.state_server(_frame, ctx);
        }

        //Client page
        if self.client_mode {
            self.state_client(_frame, ctx, input_keys);
        }
        //character picker
        if self.emoji_mode && self.client_mode {
            self.window_emoji(ctx);
        }

        //children windows
        egui::Window::new("Settings")
            .open(&mut self.settings_window)
            .show(ctx, |ui| {
                //show client mode settings
                if self.client_mode {
                    ui.label("Message editor text size");
                    ui.add(egui::Slider::new(&mut self.font_size, 1.0..=100.0).text("Text size"));
                    ui.separator();

                    ui.label("Connect to an ip address");

                    let compare_ip = self.send_on_ip.clone();

                    ui.allocate_ui(vec2(ui.available_width(), 25.), |ui| {
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.text_edit_singleline(&mut self.send_on_ip);
                            ui.allocate_ui(vec2(25., 25.), |ui| {
                                if ui
                                    .add(egui::widgets::ImageButton::new(egui::include_image!(
                                        "../icons/bookmark.png"
                                    )))
                                    .clicked()
                                {
                                    self.bookmark_mode = !self.bookmark_mode;
                                };
                            });
                        });
                    });

                    ui.checkbox(&mut self.req_passw, "Set password");
                    let compare_passwords = self.client_password.clone();
                    if self.req_passw {
                        ui.text_edit_singleline(&mut self.client_password);
                    };
                    if compare_passwords != self.client_password || self.send_on_ip != compare_ip {
                        self.autosync_sender = None;
                        self.incoming_msg = ServerMaster::default();
                    }
                } else if self.server_mode {
                }
            });

        egui::Window::new("Bookmarks")
            .open(&mut self.bookmark_mode)
            .show(ctx, |ui| {
                ui.text_edit_singleline(&mut self.send_on_ip);

                if ui.button("Save ip address").clicked() {
                    match append_to_file(self.opened_account_path.clone(), self.send_on_ip.clone())
                    {
                        Ok(_ok) => {}
                        Err(err) => eprintln!("{err}"),
                    };
                };

                ui.separator();
                ui.label(RichText::from("Saved ip addresses"));
                match read_from_file(self.opened_account_path.clone()) {
                    Ok(mut ok) => {
                        //actual decryption happens here, overwrite ok
                        ok = decrypt_lines_from_vec(ok).unwrap();

                        ui.group(|ui| {
                            if !ok.is_empty() {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    for (counter, item) in ok.iter().enumerate() {
                                        ui.horizontal(|ui| {
                                            if ui.button(RichText::from(item.clone())).clicked() {
                                                self.send_on_ip = item.clone();
                                            }
                                            ui.with_layout(
                                                Layout::right_to_left(Align::Min),
                                                |ui| {
                                                    if ui
                                                        .button(RichText::from("-").strong())
                                                        .clicked()
                                                    {
                                                        if let Err(err) = delete_line_from_file(
                                                            counter + 2,
                                                            self.opened_account_path.clone(),
                                                        ) {
                                                            eprintln!("{err}")
                                                        };
                                                    }
                                                },
                                            );
                                        });
                                    }
                                });
                            } else {
                                ui.label(RichText::from("Add your favorite servers!").strong());
                            }
                        });
                    }
                    Err(err) => eprintln!("{err}"),
                };
            });
    }
}
