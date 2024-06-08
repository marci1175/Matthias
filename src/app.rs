use egui::{vec2, Align, Color32, Layout, RichText};
use std::fs::{self};
use tap::TapFallible;

pub mod backend;

mod client;
mod server;
mod ui;

use self::backend::{display_error_message, ClientMessage, UserInformation};

use self::backend::{ClientConnection, ConnectionState, ServerMaster};

impl eframe::App for backend::TemplateApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        //try to close connection if there is one
        // &self.client_connection.

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
        /* NOTES:

            - file_tray_main.rs contains reply_tray
            - we use the self.client_ui.seen_list to get the list of all of the connected clients

        */

        /*devlog:
            TODO: fix audio playback
            TODO: add notfications
            TODO: Migrate to latest egui https://github.com/emilk/egui/issues/4306
            TODO: Discord like emoji :skull:
            TODO: make an installer for this app so there wouldnt be so many huge binary sizes
        */

        //For image loading
        egui_extras::install_image_loaders(ctx);

        //Login Page
        if !self.main.client_mode {
            self.state_login(_frame, ctx);
        }

        //Client page
        if self.main.client_mode {
            self.state_client(_frame, ctx);
        }

        //character picker
        if self.main.emoji_mode && self.main.client_mode {
            self.window_emoji(ctx);
        }

        //Create value
        let mut settings_window = self.settings_window;

        //Settings window
        egui::Window::new("Settings")
            .open(&mut settings_window)
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
                            ui.add_enabled_ui(
                                matches!(
                                    self.client_connection.state,
                                    ConnectionState::Disconnected
                                ),
                                |ui| {
                                    ui.text_edit_singleline(&mut self.client_ui.send_on_ip);
                                },
                            );

                            let username = self.login_username.clone();

                            let mut connection = self.client_connection.clone();

                            let password = self.client_connection.password.clone();

                            match &self.client_connection.state {
                                ConnectionState::Connected(_) => {
                                    if ui
                                        .button(RichText::from("Disconnect").color(Color32::RED))
                                        .clicked()
                                    {
                                        let uuid = self.opened_account.uuid.clone();
                                        //Disconnect from server
                                        tokio::task::spawn(async move {
                                            match ClientConnection::disconnect(
                                                &mut connection,
                                                username,
                                                password,
                                                uuid,
                                            )
                                            .await
                                            {
                                                Ok(_) => {}
                                                Err(err) => {
                                                    display_error_message(err);
                                                }
                                            };
                                        });

                                        self.reset_client_connection();
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
                                        self.reset_client_connection();
                                    }
                                }
                                //this is what will get displayed if there sint a connection or the last connection attempt failed
                                _ => {
                                    if ui.button("Connect").clicked() {
                                        let ip = self.client_ui.send_on_ip.clone();

                                        let username = self.login_username.clone();
                                        let password = self
                                            .client_ui
                                            .req_passw
                                            .then_some(&self.client_ui.client_password)
                                            .cloned();

                                        let sender = self.connection_sender.clone();
                                        let uuid = self.opened_account.uuid.clone();

                                        tokio::task::spawn(async move {
                                            match ClientConnection::connect(
                                                ip, username, password, &uuid,
                                            )
                                            .await
                                            {
                                                Ok(ok) => {
                                                    if let Err(err) = sender.send(Some(ok)) {
                                                        dbg!(err);
                                                    };
                                                }
                                                Err(err) => {
                                                    display_error_message(err);
                                                    if let Err(err) = sender.send(None) {
                                                        dbg!(err);
                                                    };
                                                }
                                            };
                                        });

                                        //reset autosync
                                        self.server_sender_thread = None;

                                        self.client_connection.state = ConnectionState::Connecting;
                                    }
                                }
                            }

                            ui.label(match self.client_connection.state {
                                ConnectionState::Connected(_) => {
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
                        self.server_sender_thread = None;
                        self.client_ui.incoming_msg = ServerMaster::default();
                    }

                    if self.client_ui.invalid_password {
                        ui.label(RichText::from("Invalid Password!").color(Color32::RED));
                    }

                    ui.separator();

                    self.server_setup_ui(ui);
                }
            });

        //Set value; Im terribly sorry I had to dodge this borrrow checker, LAWD HAVE MERCY
        self.settings_window = settings_window;

        //Bookmarks windows
        egui::Window::new("Bookmarks")
            .open(&mut self.main.bookmark_mode)
            .show(ctx, |ui| {
                ui.label(RichText::from("Saved ip addresses"));
                match UserInformation::deserialize(
                    &fs::read_to_string(self.opened_account.path.clone()).unwrap(),
                ) {
                    Ok(mut user_info) => {
                        if ui.button("Save ip address").clicked() {
                            user_info.add_bookmark_entry(self.client_ui.send_on_ip.clone());

                            let _ = user_info
                                .write_file(self.opened_account.path.clone())
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
                                                                self.opened_account.path.clone(),
                                                            )
                                                            .tap_err_dbg(|err| {
                                                                tracing::error!("{err}")
                                                            });
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
                    //Modify client_connection
                    self.client_connection = connection.0;

                    //Modify local message list
                    let incoming_sync_message: Result<ServerMaster, serde_json::Error> =
                        serde_json::from_str(&connection.1);

                    if let Ok(incoming_message) = incoming_sync_message {
                        self.client_ui.incoming_msg = incoming_message;
                    } else {
                        eprintln!("Failed to convert {} to ServerMaster", connection.1)
                    }
                } else {
                    //If we recived a None it means we have an error
                    self.client_connection.state = ConnectionState::Error;
                }
            }
            Err(_err) => {
                // dbg!(_err);
            }
        }
    }
}

impl backend::TemplateApp {
    /// This function spawn an async tokio thread, to send the message passed in as the argument, this function does not await a response from the server
    pub fn send_msg(&self, message: ClientMessage) {
        let connection = self.client_connection.clone();

        tokio::spawn(async move {
            match connection.send_message(message).await {
                //We dont need the server's reply since we dont handle it here
                Ok(_server_reply) => {}
                Err(err) => {
                    dbg!(err.source());
                    dbg!(err);
                }
            };
        });
    }

    /// This function resets clientconnection and all of its other attributes (self.client_ui.incoming_msg, self.autosync_should_run)
    fn reset_client_connection(&mut self) {
        //Reset client, as we are already disconnecting above
        self.client_connection.reset_state();

        self.client_ui.incoming_msg = ServerMaster::default();

        let _ = self.autosync_input_sender.send(());

        self.client_connection.state = ConnectionState::Disconnected;
    }
}
