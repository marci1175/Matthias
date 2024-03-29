use egui::{vec2, Align, Color32, Layout, RichText};
use tap::TapFallible;
use std::fs::{self};
use windows_sys::w;
use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR};

pub mod backend;

mod client;
mod server;
mod ui;

use self::backend::{ClientMessage, UserInformation};

use self::backend::{ClientConnection, ConnectionState, ServerMaster};

impl eframe::App for backend::TemplateApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        //clean up after server, client
        match std::env::var("APPDATA") {
            Ok(app_data) => {
                if let Err(err) = fs::remove_dir_all(format!("{}\\Matthias\\Server", app_data)) {
                    println!("{err}");
                };
                if let Err(err) = fs::remove_dir_all(format!("{}\\Matthias\\Client", app_data)) {
                    println!("{err}");
                };
            }
            Err(err) => println!("{err}"),
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        //dbg!(generate_uuid());

        /* NOTES:

            - file_tray_main.rs contains reply_tray

        */

        /*devlog:

            TODO: fix for audio playback

        */

        //For image loading
        egui_extras::install_image_loaders(ctx);

        //Login Page
        if !(self.main.mode_selector || self.main.server_mode || self.main.client_mode) {
            self.state_login(_frame, ctx);
        }

        //Main page
        if self.main.mode_selector && !(self.main.server_mode || self.main.client_mode) {
            self.state_mode_selection(_frame, ctx);
        }

        //Server page
        if self.main.server_mode {
            self.state_server(_frame, ctx);
        }

        //Client page
        if self.main.client_mode {
            self.state_client(_frame, ctx);
        }

        //character picker
        if self.main.emoji_mode && self.main.client_mode {
            self.window_emoji(ctx);
        }

        //children windows
        egui::Window::new("Settings")
            .open(&mut self.settings_window)
            .show(ctx, |ui| {
                //show client mode settings
                if self.main.client_mode {
                    ui.label("Message editor text size");
                    ui.add(egui::Slider::new(&mut self.font_size, 1.0..=100.0).text("Text size"));
                    ui.separator();

                    ui.label("Connect to an ip address");

                    let compare_ip = self.client_ui.send_on_ip.clone();

                    ui.allocate_ui(vec2(ui.available_width(), 25.), |ui| {
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            //Make connecting to an ip more user friendly
                            // ui.add(egui::TextEdit::singleline(&mut self.send_on_address).hint_text("Address"));
                            // ui.add(egui::TextEdit::singleline(&mut self.send_on_port).hint_text("Port"));

                            // //format two text inputs, so because im too lazy
                            // self.send_on_ip = format!("[{}]:{}", self.send_on_address, self.send_on_port);

                            //Check if there already is a connection
                            ui.add_enabled_ui(self.client_connection.client.is_none(), |ui| {
                                ui.text_edit_singleline(&mut self.client_ui.send_on_ip);
                            });

                            let username = self.login_username.clone();

                            let mut connection = self.client_connection.clone();

                            let password = self.client_ui.client_password.clone();

                            match self.client_connection.state {
                                ConnectionState::Connected => {
                                    if ui
                                        .button(RichText::from("Disconnect").color(Color32::RED))
                                        .clicked()
                                    {
                                        //Disconnect from server
                                        tokio::task::spawn(async move {
                                            match ClientConnection::disconnect(
                                                &mut connection,
                                                username,
                                                password,
                                            )
                                            .await
                                            {
                                                Ok(_) => {}
                                                Err(err) => {
                                                    std::thread::spawn(move || unsafe {
                                                        MessageBoxW(
                                                            0,
                                                            str::encode_utf16(
                                                                err.to_string().as_str(),
                                                            )
                                                            .chain(std::iter::once(0))
                                                            .collect::<Vec<_>>()
                                                            .as_ptr(),
                                                            w!("Error"),
                                                            MB_ICONERROR,
                                                        );
                                                    });
                                                }
                                            };
                                        });

                                        //Reset client
                                        self.client_connection.client = None;
                                        self.client_ui.incoming_msg = ServerMaster::default();
                                        self.autosync_should_run
                                            .store(false, std::sync::atomic::Ordering::Relaxed);

                                        self.client_connection.state =
                                            ConnectionState::Disconnected;
                                    }
                                }
                                ConnectionState::Disconnected => {
                                    if ui.button("Connect").clicked() {
                                        let ip = self.client_ui.send_on_ip.clone();

                                        let sender = self.connection_sender.clone();

                                        let username = self.login_username.clone();

                                        let password = self
                                            .client_ui
                                            .req_passw
                                            .then_some((|| &self.client_ui.client_password)())
                                            .cloned();

                                        tokio::task::spawn(async move {
                                            match ClientConnection::connect(
                                                format!("http://{}", ip),
                                                username,
                                                password,
                                            )
                                            .await
                                            {
                                                Ok(ok) => {
                                                    if let Err(err) = sender.send(Some(ok)) {
                                                        dbg!(err);
                                                    };
                                                }
                                                Err(err) => {
                                                    std::thread::spawn(move || unsafe {
                                                        MessageBoxW(
                                                            0,
                                                            str::encode_utf16(
                                                                err.to_string().as_str(),
                                                            )
                                                            .chain(std::iter::once(0))
                                                            .collect::<Vec<_>>()
                                                            .as_ptr(),
                                                            w!("Error"),
                                                            MB_ICONERROR,
                                                        );
                                                    });
                                                    if let Err(err) = sender.send(None) {
                                                        dbg!(err);
                                                    };
                                                }
                                            };
                                        });

                                        self.autosync_should_run
                                            .store(true, std::sync::atomic::Ordering::Relaxed);

                                        //reset autosync
                                        self.autosync_sender = None;

                                        self.client_connection.state = ConnectionState::Connecting;
                                    }
                                }
                                ConnectionState::Connecting => {
                                    if ui
                                        .button(
                                            RichText::from("Cancel connection")
                                                .color(Color32::LIGHT_GRAY),
                                        )
                                        .clicked()
                                    {
                                        //Reset client
                                        self.client_connection.client = None;
                                        self.client_ui.incoming_msg = ServerMaster::default();
                                        self.autosync_should_run
                                            .store(false, std::sync::atomic::Ordering::Relaxed);

                                        self.client_connection.state =
                                            ConnectionState::Disconnected;
                                    }
                                }
                                ConnectionState::Error => {
                                    if ui.button("Connect").clicked() {
                                        let ip = self.client_ui.send_on_ip.clone();

                                        let username = self.login_username.clone();
                                        let password = self
                                            .client_ui
                                            .req_passw
                                            .then_some((|| &self.client_ui.client_password)())
                                            .cloned();

                                        let sender = self.connection_sender.clone();

                                        tokio::task::spawn(async move {
                                            match ClientConnection::connect(
                                                format!("http://{}", ip),
                                                username,
                                                password,
                                            )
                                            .await
                                            {
                                                Ok(ok) => {
                                                    if let Err(err) = sender.send(Some(ok)) {
                                                        dbg!(err);
                                                    };
                                                }
                                                Err(err) => {
                                                    std::thread::spawn(move || unsafe {
                                                        MessageBoxW(
                                                            0,
                                                            str::encode_utf16(
                                                                err.to_string().as_str(),
                                                            )
                                                            .chain(std::iter::once(0))
                                                            .collect::<Vec<_>>()
                                                            .as_ptr(),
                                                            w!("Error"),
                                                            MB_ICONERROR,
                                                        );
                                                    });
                                                    if let Err(err) = sender.send(None) {
                                                        dbg!(err);
                                                    };
                                                }
                                            };
                                        });

                                        self.autosync_should_run
                                            .store(true, std::sync::atomic::Ordering::Relaxed);

                                        //reset autosync
                                        self.autosync_sender = None;

                                        self.client_connection.state = ConnectionState::Connecting;
                                    }
                                }
                            }

                            ui.label(match self.client_connection.state {
                                ConnectionState::Connected => {
                                    RichText::from("Connected").color(Color32::GREEN)
                                }
                                ConnectionState::Disconnected => {
                                    RichText::from("Disconnected").color(Color32::LIGHT_RED)
                                }
                                ConnectionState::Connecting => {
                                    RichText::from("Connecting").color(Color32::LIGHT_GREEN)
                                }
                                ConnectionState::Error => {
                                    RichText::from("Error when trying to connect")
                                        .color(Color32::RED)
                                }
                            });

                            ui.allocate_ui(vec2(25., 25.), |ui| {
                                if ui
                                    .add(egui::widgets::ImageButton::new(egui::include_image!(
                                        "../icons/bookmark.png"
                                    )))
                                    .clicked()
                                {
                                    self.main.bookmark_mode = !self.main.bookmark_mode;
                                };
                            });
                        });
                    });

                    ui.checkbox(&mut self.client_ui.req_passw, "Set password");
                    let compare_passwords = self.client_ui.client_password.clone();
                    if self.client_ui.req_passw {
                        ui.text_edit_singleline(&mut self.client_ui.client_password);
                    };
                    if compare_passwords != self.client_ui.client_password
                        || self.client_ui.send_on_ip != compare_ip
                    {
                        self.autosync_sender = None;
                        self.client_ui.incoming_msg = ServerMaster::default();
                    }
                    if self.client_ui.invalid_password {
                        ui.label(RichText::from("Invalid Password!").color(Color32::RED));
                    }
                } else if self.main.server_mode {
                }
            });

        egui::Window::new("Bookmarks")
            .open(&mut self.main.bookmark_mode)
            .show(ctx, |ui| {
                ui.label(RichText::from("Saved ip addresses"));
                match UserInformation::deserialize(
                    &fs::read_to_string(self.main.opened_account_path.clone()).unwrap(),
                ) {
                    Ok(mut user_info) => {
                        if ui.button("Save ip address").clicked() {
                            user_info.add_bookmark_entry(self.client_ui.send_on_ip.clone());

                            let _ = user_info
                                .write_file(self.main.opened_account_path.clone())
                                .tap_err_dbg(|err| tracing::error!("{err}"));
                        };

                        ui.separator();

                        let bookmark_entries = user_info.bookmarked_ips.clone();

                        ui.group(|ui| {
                            if !bookmark_entries.is_empty() {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    for (index, item) in bookmark_entries.iter().enumerate() {
                                        ui.horizontal(|ui| {
                                            if ui.button(RichText::from(item.clone())).clicked() {
                                                self.client_ui.send_on_ip.clone_from(item);
                                            }
                                            ui.with_layout(
                                                Layout::right_to_left(Align::Min),
                                                |ui| {
                                                    if ui
                                                        .button(RichText::from("-").strong())
                                                        .clicked()
                                                    {
                                                        user_info.delete_bookmark_entry(index);

                                                        //Dont check if user already exists because we overwrite the file which was already there
                                                        let _ = user_info
                                                            .write_file(
                                                                self.main
                                                                    .opened_account_path
                                                                    .clone(),
                                                            )
                                                            .tap_err_dbg(|err| tracing::error!("{err}"));
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

        //Connection reciver
        match self.connection_reciver.try_recv() {
            Ok(connection) => {
                if let Some(connection) = connection {
                    if connection.client.is_some() {
                        self.client_connection.state = ConnectionState::Connected;
                        self.client_connection = connection
                    } else {
                        self.client_connection.state = ConnectionState::Error;
                    }
                }
            }
            Err(_err) => {
                // dbg!(_err);
            }
        }
    }
}

impl backend::TemplateApp {
    pub fn send_msg(&self, message: ClientMessage) {
        let connection = self.client_connection.clone();
        let tx = self.tx.clone();

        tokio::spawn(async move {
            match client::send_msg(connection, message).await {
                Ok(ok) => {
                    match tx.send(ok) {
                        Ok(_) => {}
                        Err(err) => {
                            println!("{} ln 376", err);
                        }
                    };
                }
                Err(err) => {
                    dbg!(err.source());
                    dbg!(err);
                }
            };
        });
    }
}
