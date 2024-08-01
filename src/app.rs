//Define the maximum amount of entries in the lua output vector
const LUA_OUTPUT_BUFFER_SIZE: usize = 100;

use crate::app::lua::ExtensionProperties;
use anyhow::Error;
use base64::engine::general_purpose;
use base64::Engine;
use egui::{
    vec2, Align, Color32, Layout, Modifiers, RichText, ScrollArea, Slider, Stroke, TextEdit,
};
use egui_extras::{Column, TableBuilder};
use lua::{execute_code, load_code};
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

impl eframe::App for backend::Application {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
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
                    Ok(_) => {}
                    Err(err) => {
                        //Avoid panicking when trying to display a Notification
                        //This is very rare but can still happen
                        match toasts.lock() {
                            Ok(mut toasts) => {
                                display_error_message(err, &mut toasts);
                            }
                            Err(err) => {
                                dbg!(err);
                            }
                        }
                    }
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
            }
            Err(err) => println!("{err}"),
        }

        //Shut down the server
        self.server_shutdown_token.cancel();
        self.autosync_shutdown_token.cancel();
        self.voip_shutdown_token.cancel();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        /* TODOS:
            TODO: Migrate to latest egui https://github.com/emilk/egui/issues/4306
            TODO: Restructure files

            TODO: Add notifcations to lua API, callbacks
        */

        //Display notifications
        self.toasts.lock().unwrap().show(ctx);

        self.client_ui.extension.event_call_extensions(
            lua::EventCall::OnDraw,
            &self.lua,
            None,
        );

        //Truncate the vector from the other way around so the newest messages will stay
        //This will only start working if ```self.client_ui.extension.output.len() > LUA_OUTPUT_BUFFER_SIZE```
        match self.client_ui.extension.output.try_lock() {
            Ok(mut output) => {
                if let Some(desired_idx) = output.len().checked_sub(LUA_OUTPUT_BUFFER_SIZE) {
                    output.drain(0..desired_idx);
                }
            }
            Err(err) => {
                dbg!(err);
            }
        }

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
        let mut settings_window = self.settings_window;

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

                    //Modify the base64 encoded string of send on ip, so it can be used in different places without having to re-encode every frame
                    self.client_ui.send_on_ip_base64_encoded =
                        general_purpose::URL_SAFE_NO_PAD.encode(self.client_ui.send_on_ip.clone());

                    if let Ok(incoming_message) = incoming_sync_message {
                        self.client_ui.incoming_messages = incoming_message;

                        //Callback
                        self.client_ui.extension.event_call_extensions(crate::app::lua::EventCall::OnConnect, &self.lua, Some(self.client_ui.send_on_ip.clone()));
                    } else {
                        eprintln!("Failed to convert {} to ServerMaster", connection.1)
                    }
                } else {
                    // A race condition will occur if we connected succesfully after getting a connection error (request timed out)
                    // So we check if we have already made the connection before actually modifying the value based on the timed out request
                    if !matches!(self.client_connection.state, ConnectionState::Connected(_)) {
                        //If we recived a None it means we have an error
                        self.client_connection.state = ConnectionState::Error;
                    }
                }
            }
            Err(_err) => {
                // dbg!(_err);
            }
        }

        //Voip instance listener
        match self.voip_connection_reciver.try_recv() {
            Ok(voip) => {
                self.client_ui.voip = Some(voip.clone());

                self.send_msg(ClientMessage::construct_voip_connect(
                    &self.opened_user_information.uuid,
                    voip.socket.local_addr().unwrap().port(),
                ))
            }
            Err(_err) => {}
        }
    }
}

impl backend::Application {
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
        self.client_ui.incoming_messages = ServerMaster::default();

        self.autosync_shutdown_token.cancel();

        self.client_connection.state = ConnectionState::Disconnected;
    }

    fn client_settings_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
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

                                //Callback
                                self.client_ui.extension.event_call_extensions(lua::EventCall::OnDisconnect, &self.lua, None);
                            }
                        }
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
                        }
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
                                        ip,
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
                                                dbg!(err);
                                            };
                                        }
                                        Err(err) => {
                                            //Avoid panicking when trying to display a Notification
                                            //This is very rare but can still happen
                                            match toasts.lock() {
                                                Ok(mut toasts) => {
                                                    display_error_message(err, &mut toasts);
                                                }
                                                Err(err) => {
                                                    dbg!(err);
                                                }
                                            }
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
                            RichText::from("Error when trying to connect").color(Color32::RED)
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

            let compare_passwords = self.client_ui.client_password.clone();

            ui.add_enabled(
                !matches!(self.client_connection.state, ConnectionState::Connected(_)),
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

            //Draw the extensions part of the ui
            ui.collapsing("Extensions", |ui| {
                self.client_extension(ui, ctx);
            });

            ui.horizontal(|ui| {
                ui.label("Microphone volume precentage");
                ui.add(Slider::new(
                    &mut *self.client_ui.microphone_volume.lock().unwrap(),
                    50.0..=500.0,
                ));
            });
        });
    }

    fn disconnect_from_server(&mut self) {
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
                Ok(_) => {}
                Err(err) => {
                    //Avoid panicking when trying to display a Notification
                    //This is very rare but can still happen
                    match toasts.lock() {
                        Ok(mut toasts) => {
                            display_error_message(err, &mut toasts);
                        }
                        Err(err) => {
                            dbg!(err);
                        }
                    }
                }
            };
        });

        //Reset client, as we are already disconnecting above
        self.client_connection.reset_state();
    }

    /// Draw the extension part of the ui in the settings
    fn client_extension(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.label("Extensions");

            //Refresh button for refreshing the extensions
            if ui.button("Refresh").clicked() {
                //Read the extensions
                match read_extensions_dir() {
                    Ok(extension_list) => {
                        self.client_ui.extension.extension_list = extension_list;
                    }
                    //If there was an error, print it out and create the extensions folder as this is the most likely thing to error
                    Err(err) => {
                        dbg!(err);
                        let _ =
                            fs::create_dir(format!("{}\\matthias\\extensions", env!("APPDATA")));
                    }
                }
            };

            //Documentation
            ui.hyperlink_to("Documentation", "https://matthias.gitbook.io/matthias")
                .on_hover_text("For more information read the documentation.");
        });

        ui.horizontal(|ui| {
            ui.columns(2, |columns| {
                self.draw_extension_table(columns, ctx);

                self.draw_extension_output(columns);
            });
        });
    }

    fn draw_extension_table(&mut self, columns: &mut [egui::Ui], ctx: &egui::Context) {
        let available_width = columns[0].available_width();

        TableBuilder::new(&mut columns[0])
            .resizable(true)
            .auto_shrink([true, false])
            .striped(true)
            .columns(
                Column::remainder().at_most(available_width),
                /*Columns should be: 1. Name 2. Start / Stop 3. Edit*/ 3,
            )
            .header(25., |mut rows| {
                //Name
                rows.col(|ui| {
                    ui.label("Name");
                });
                //Start / Stop
                rows.col(|ui| {
                    ui.label("State");
                });
                //Edit
                rows.col(|ui| {
                    ui.label("Edit");
                });
            })
            .body(|body| {
                //Iter over all the extensions
                body.rows(
                    30.,
                    self.client_ui.extension.extension_list.len(),
                    |mut row| {
                        let row_idx = row.index();

                        //Each ```extension_list``` entry is a row
                        let extension = &mut self.client_ui.extension.extension_list[row_idx];

                        //Name
                        row.col(|ui| {
                            ui.horizontal_centered(|ui| {
                                ui.label(&extension.name);
                            });
                        });

                        //Start / Stop
                        row.col(|ui: &mut egui::Ui| {
                            ui.horizontal_centered(|ui| {
                                //If extension is stopped
                                let has_been_clicked = if !extension.is_running {
                                    ui.button("Start")
                                } else {
                                    ui.button("Stop")
                                }
                                .clicked();

                                //Change value if it has been interacted with there are only two states so this is pretty straightforward
                                if has_been_clicked {
                                    extension.is_running = !extension.is_running;

                                    match self.client_ui.extension.output.try_lock() {
                                        Ok(mut output) => match extension.is_running {
                                            true => {
                                                output.push(lua::LuaOutput::Info(format!(
                                                    r#"Extension "{}" has been started."#,
                                                    extension.name
                                                )));
                                            }
                                            false => {
                                                output.push(lua::LuaOutput::Info(format!(
                                                    r#"Extension "{}" has been stopped."#,
                                                    extension.name
                                                )));
                                            }
                                        },
                                        Err(err) => {
                                            dbg!(err);
                                        }
                                    }
                                }
                            });
                        });
                        //Edit
                        row.col(|ui| {
                            ui.horizontal_centered(|ui| {
                                ui.menu_button("Edit", |ui| {
                                    ui.horizontal(|ui| {
                                        if ui.button("Save").clicked() {
                                            if let Err(err) = extension.write_change_to_file() {
                                                //Avoid panicking when trying to display a Notification
                                                //This is very rare but can still happen
                                                match self.toasts.lock() {
                                                    Ok(mut toasts) => {
                                                        display_error_message(err, &mut toasts);
                                                    }
                                                    Err(err) => {
                                                        dbg!(err);
                                                    }
                                                }
                                            };
                                        }

                                        ui.label("CTRL + S");

                                        if extension.contents != extension.text_edit_buffer {
                                            ui.label(RichText::new("Unsaved").color(Color32::RED));
                                        }
                                    });

                                    let theme =
                                        egui_extras::syntax_highlighting::CodeTheme::from_memory(
                                            ui.ctx(),
                                        );

                                    let mut layouter =
                                        |ui: &egui::Ui, string: &str, wrap_width: f32| {
                                            let mut layout_job =
                                                egui_extras::syntax_highlighting::highlight(
                                                    ui.ctx(),
                                                    &theme,
                                                    string,
                                                    "lua",
                                                );
                                            layout_job.wrap.max_width = wrap_width;
                                            ui.fonts(|f| f.layout_job(layout_job))
                                        };

                                    ui.add(
                                        TextEdit::multiline(&mut extension.text_edit_buffer)
                                            .code_editor()
                                            .layouter(&mut layouter),
                                    );
                                });

                                //Display unsaved state
                                if extension.contents != extension.text_edit_buffer {
                                    ui.label(RichText::new("Unsaved").color(Color32::RED));
                                }
                            });

                            //Catch ctrl + c shortcut for cooler text edit
                            ctx.input_mut(|writer| {
                                if writer.consume_key(Modifiers::CTRL, egui::Key::S) {
                                    if let Err(err) = extension.write_change_to_file() {
                                        //Avoid panicking when trying to display a Notification
                                        //This is very rare but can still happen
                                        match self.toasts.lock() {
                                            Ok(mut toasts) => {
                                                display_error_message(err, &mut toasts);
                                            }
                                            Err(err) => {
                                                dbg!(err);
                                            }
                                        }
                                    };
                                }
                            });
                        });
                    },
                )
            });
    }

    ///Draw the extension's output into the little "console"
    fn draw_extension_output(&mut self, columns: &mut [egui::Ui]) {
        columns[1].painter().rect_stroke(
            self.client_ui.extension.output_rect.expand(5.),
            5.,
            Stroke::new(2., Color32::GRAY),
        );

        columns[1]
            .painter()
            .rect_filled(self.client_ui.extension.output_rect, 5., Color32::BLACK);

        let scroll_area = ScrollArea::vertical()
            .stick_to_bottom(true)
            .id_source(columns[1].next_auto_id())
            .show(&mut columns[1], |ui| {
                for output in self
                    .client_ui
                    .extension
                    .output
                    .lock()
                    .as_deref()
                    .unwrap_or(&vec![])
                    .iter()
                {
                    match output {
                        lua::LuaOutput::Error(error) => {
                            ui.label(RichText::from(error).color(Color32::RED));
                        }
                        lua::LuaOutput::Standard(output) => {
                            ui.label(RichText::from(output).color(Color32::LIGHT_YELLOW));
                        }
                        lua::LuaOutput::Info(info) => {
                            ui.label(
                                RichText::from(format!("INFO: {info}")).color(Color32::LIGHT_BLUE),
                            );
                        }
                    }
                }
            });

        self.client_ui.extension.output_rect = scroll_area.inner_rect;
    }
}

///Read all the extensions from the folder
pub fn read_extensions_dir() -> anyhow::Result<Vec<ExtensionProperties>> {
    let mut extensions: Vec<ExtensionProperties> = Vec::new();

    for entry in fs::read_dir(format!("{}\\matthias\\extensions", env!("APPDATA")))? {
        let dir_entry = entry.map_err(|err| Error::msg(err.to_string()))?;

        //If the file doesnt have an extension, then we can ingore it
        if let Some(extension) = dir_entry.path().extension() {
            //If the file is a lua file
            if extension.to_string_lossy() == "lua" {
                //Get the path to the entry
                let path_to_entry = dir_entry.path();
                //Read the file so it can be run later
                let file_content = fs::read_to_string(&path_to_entry)?;

                //Push back the important info, so it can be returned later
                extensions.push(ExtensionProperties::new(
                    file_content,
                    path_to_entry.clone(),
                    path_to_entry
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                ));
            }
        }
    }

    Ok(extensions)
}
