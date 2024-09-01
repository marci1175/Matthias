/// Define the maximum amount of entries in the lua output vector
const LUA_OUTPUT_BUFFER_SIZE: usize = 100;

/// Define custom url of this application
/// The registry keys for opening this url should already be added by the installer
const CUSTOM_URL: &str = "matthias://";

/// Define the url to the deocumentation
const DOCUMENTATION_URL: &str = "https://matthias.gitbook.io/matthiasdocu";

use anyhow::Error;
use base64::{engine::general_purpose, Engine};
use egui::{
    vec2, Align, Color32, KeyboardShortcut, Layout, Modifiers, RichText, ScrollArea, Slider,
    Stroke, TextEdit,
};
use egui_extras::{Column, TableBuilder};
use egui_notify::Toast;
use std::fs::{self};
use tap::TapFallible;
use tokio_util::sync::CancellationToken;

pub mod backend;

mod client;
mod lua;
mod server;
mod ui;

use self::backend::{display_error_message, ClientMessage, UserInformation};

use self::backend::{ClientConnection, ConnectionState, ServerMaster};

impl eframe::App for backend::Application
{
    fn save(&mut self, storage: &mut dyn eframe::Storage)
    {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>)
    {
        //try to close connection if there is one
        let username = self.login_username.clone();
        let mut connection = self.client_connection.clone();
        let password = self.client_connection.password.clone();
        let uuid = self.opened_user_information.uuid.clone();
        let toasts = self.toasts.clone();

        //Disconnect from server
        if let ConnectionState::Connected(_) = self.client_connection.state {
            tokio::task::spawn(async move {
                match ClientConnection::disconnect(&mut connection, username, password, uuid).await
                {
                    Ok(_) => {},
                    Err(err) => {
                        //Avoid panicking when trying to display a Notification
                        //This is very rare but can still happen
                        display_error_message(err, toasts);
                    },
                };
            });
        }

        //clean up after server and client
        match std::env::var("APPDATA") {
            Ok(app_data) => {
                if let Err(_err) = fs::remove_dir_all(format!("{}\\Matthias\\Server", app_data)) {
                    // println!("{_err}");
                };
                if let Err(_err) = fs::remove_dir_all(format!("{}\\Matthias\\Client", app_data)) {
                    // println!("{_err}");
                };
            },
            Err(err) => println!("{err}"),
        }

        //Shut down the server
        self.server_shutdown_token.cancel();
        self.autosync_shutdown_token.cancel();
        self.voip_shutdown_token.cancel();
        self.voip_video_shutdown_token.cancel();

        //Signal the voice recorder function to stop
        let _ = self.record_audio_interrupter.send(());
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame)
    {
        /* TODOS:
            TODO: Migrate to latest egui https://github.com/emilk/egui/issues/4306
            TODO: Restructure files
            TODO: Create sound effects

            TODO: Add AI (Neural network) powered hand drawing recognition
            TODO: Improve video chat
            TODO: Fix disconnecting from a call
        */

        if self.main.register_mode {
            self.state_register(_frame, ctx);
            return;
        }

        //Login Page
        if !self.main.client_mode {
            self.state_login(_frame, ctx);
        }

        //Client page
        if self.main.client_mode {
            self.state_client(_frame, ctx);
        }

        //Create value
        let mut settings_window = self.settings_window && self.main.client_mode;

        //Settings window
        egui::Window::new("Settings")
            .open(&mut settings_window)
            .show(ctx, |ui| {
                //show client mode settings
                if self.main.client_mode {
                    self.client_settings_ui(ui, ctx);

                    self.server_setup_ui(ui, ctx);
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
                    &fs::read_to_string(self.opened_user_information.path.clone()).unwrap(),
                    self.opened_user_information.password.clone(),
                ) {
                    Ok(mut user_info) => {
                        if ui.button("Save ip address").clicked() {
                            user_info.add_bookmark_entry(self.client_ui.send_on_ip.clone());

                            let _ = user_info
                                .write_file(self.opened_user_information.path.clone())
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
                                                                self.opened_user_information
                                                                    .path
                                                                    .clone(),
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
                            }
                            else {
                                ui.label(RichText::from("Add your favorite servers!").strong());
                            }
                        });
                    },
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

                    //Modify the base64 encoded string of send on ip, so it can be used in different places without having to re-encode every frame
                    self.client_ui.send_on_ip_base64_encoded =
                        general_purpose::URL_SAFE_NO_PAD.encode(self.client_ui.send_on_ip.clone());

                    if let Ok(incoming_message) = incoming_sync_message {
                        self.client_ui.incoming_messages = incoming_message;
                    }
                    else {
                        eprintln!("Failed to convert {} to ServerMaster", connection.1)
                    }
                }
                else {
                    // A race condition will occur if we connected succesfully after getting a connection error (request timed out)
                    // So we check if we have already made the connection before actually modifying the value based on the timed out request
                    if !matches!(self.client_connection.state, ConnectionState::Connected(_)) {
                        //If we recived a None it means we have an error
                        self.client_connection.state = ConnectionState::Error;
                    }
                }
            },
            Err(_err) => {
                // tracing::error!("{}", _err.to_string());
            },
        }

        //Voip instance listener
        // match self.voip_connection_reciver.try_recv() {
        //     Ok(voip) => {
        //         self.client_ui.voip = Some(voip.clone());

        //         self.send_msg(ClientMessage::construct_voip_connect(
        //             &self.opened_user_information.uuid,
        //             voip.socket.local_addr().unwrap().port(),
        //         ))
        //     },
        //     Err(_err) => {},
        // }
    }
}

impl backend::Application
{
    /// This function spawn an async tokio thread, to send the message passed in as the argument, this function does not await a response from the server
    pub fn send_msg(&self, message: ClientMessage)
    {
        let connection = self.client_connection.clone();

        tokio::spawn(async move {
            match connection.send_message(message).await {
                //We dont need the server's reply since we dont handle it here
                Ok(_server_reply) => {},
                Err(err) => {
                    tracing::error!("{:?}", err.source());
                    tracing::error!("{}", err);
                },
            };
        });
    }

    /// This function resets clientconnection and all of its other attributes (self.client_ui.incoming_msg, self.autosync_should_run)
    fn reset_client_connection(&mut self)
    {
        self.client_ui.incoming_messages = ServerMaster::default();

        self.autosync_shutdown_token.cancel();

        self.client_connection.state = ConnectionState::Disconnected;
    }

    fn client_settings_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context)
    {
        ui.collapsing("Client", |ui| {
            ui.label("Connect to an ip address");

            let compare_ip = self.client_ui.send_on_ip.clone();

            ui.allocate_ui(vec2(ui.available_width(), 25.), |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.add_enabled_ui(
                        matches!(self.client_connection.state, ConnectionState::Disconnected)
                            || matches!(self.client_connection.state, ConnectionState::Error),
                        |ui| {
                            ui.add(
                                TextEdit::singleline(&mut self.client_ui.send_on_ip)
                                    .hint_text("Address"),
                            )
                            .on_hover_text(
                                "Formatting: [FFFF:FFFF:FFFF:FFFF:FFFF:FFFF:FFFF:FFFF]:PORT",
                            );
                        },
                    );

                    match &self.client_connection.state {
                        ConnectionState::Connected(_) => {
                            if ui
                                .button(RichText::from("Disconnect").color(Color32::RED))
                                .clicked()
                            {
                                self.disconnect_from_server();
                            }
                        },
                        ConnectionState::Connecting => {
                            if ui
                                .button(
                                    RichText::from("Cancel connection").color(Color32::LIGHT_GRAY),
                                )
                                .clicked()
                            {
                                //Reset client
                                self.reset_client_connection();
                            }
                        },
                        _ => {
                            if ui.button("Connect").clicked() {
                                self.connect_to_server(
                                    ctx,
                                    self.client_ui.send_on_ip.clone(),
                                    self.client_ui
                                        .req_passw
                                        .then_some(&self.client_ui.client_password)
                                        .cloned(),
                                );
                            }
                        },
                    }

                    ui.label(match self.client_connection.state {
                        ConnectionState::Connected(_) => {
                            RichText::from("Connected").color(Color32::GREEN)
                        },
                        ConnectionState::Disconnected => {
                            RichText::from("Disconnected").color(Color32::LIGHT_RED)
                        },
                        ConnectionState::Connecting => {
                            RichText::from("Connecting").color(Color32::LIGHT_GREEN)
                        },
                        ConnectionState::Error => {
                            RichText::from("Error when trying to connect").color(Color32::RED)
                        },
                    });

                    ui.allocate_ui(vec2(25., 25.), |ui| {
                        if ui
                            .add(egui::widgets::ImageButton::new(egui::include_image!(
                                "../../assets/icons/bookmark.png"
                            )))
                            .clicked()
                        {
                            self.main.bookmark_mode = !self.main.bookmark_mode;
                        };
                    });
                });
            });

            let compare_passwords = self.client_ui.client_password.clone();

            ui.add_enabled(
                matches!(self.client_connection.state, ConnectionState::Disconnected)
                    || matches!(self.client_connection.state, ConnectionState::Error),
                |ui: &mut egui::Ui| {
                    ui.add(
                        TextEdit::singleline(&mut self.client_ui.client_password)
                            .hint_text("Password (Optional)"),
                    )
                },
            );

            if compare_passwords != self.client_ui.client_password
                || self.client_ui.send_on_ip != compare_ip
            {
                self.server_sender_thread = None;
                self.client_ui.incoming_messages = ServerMaster::default();
            }

            ui.horizontal(|ui| {
                ui.label("Microphone volume precentage");
                ui.add(Slider::new(
                    &mut *self.client_ui.microphone_volume.lock().unwrap(),
                    50.0..=500.0,
                ));
            });
        });
    }

    pub fn connect_to_server(
        &mut self,
        ctx: &egui::Context,
        address: String,
        password: Option<String>,
    )
    {
        let username = self.login_username.clone();

        let sender = self.connection_sender.clone();

        //Reset shutdown token
        self.autosync_shutdown_token = CancellationToken::new();

        //Clone ctx so we can call request repaint from another thread
        let ctx = ctx.clone();

        let user_information = self.opened_user_information.clone();

        //Reset all messages and everything else
        self.client_ui.incoming_messages = ServerMaster::default();

        //Forget all imaes so the cahced imges will be deleted
        ctx.forget_all_images();

        let toasts = self.toasts.clone();

        tokio::task::spawn(async move {
            match ClientConnection::connect_to_server(
                address,
                username,
                password,
                &user_information.uuid,
                user_information.profile,
            )
            .await
            {
                Ok(ok) => {
                    ctx.request_repaint();
                    if let Err(err) = sender.send(Some(ok)) {
                        tracing::error!("{}", err);
                    };
                },
                Err(err) => {
                    //Avoid panicking when trying to display a Notification
                    //This is very rare but can still happen
                    display_error_message(err, toasts);

                    if let Err(err) = sender.send(None) {
                        tracing::error!("{}", err);
                    };
                },
            };
        });

        //reset autosync
        self.server_sender_thread = None;

        self.client_connection.state = ConnectionState::Connecting;
    }

    fn disconnect_from_server(&mut self)
    {
        let username = self.login_username.clone();

        let mut connection = self.client_connection.clone();

        let password = self.client_connection.password.clone();

        let uuid = self.opened_user_information.uuid.clone();

        let toasts = self.toasts.clone();

        //Shut down threadsa nad reset state
        self.reset_client_connection();

        //Disconnect from server
        tokio::task::spawn(async move {
            match connection.disconnect(username, password, uuid).await {
                Ok(_) => {},
                Err(err) => {
                    //Avoid panicking when trying to display a Notification
                    //This is very rare but can still happen
                    display_error_message(err, toasts);
                },
            };
        });

        //Reset client, as we are already disconnecting above
        self.client_connection.reset_state();
    }
}