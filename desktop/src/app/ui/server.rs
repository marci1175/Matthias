use crate::app::{
    backend::{display_error_message, ipv4_get, ipv6_get, Application, ClientProfile},
    server,
};
use dashmap::DashMap;
use egui::{vec2, Align, Color32, Context, Image, Layout, RichText};
use egui_extras::{Column, TableBuilder};
use tokio_util::sync::CancellationToken;

impl Application
{
    pub fn server_setup_ui(&mut self, ui: &mut egui::Ui, ctx: &Context)
    {
        ui.collapsing("Server", |ui| {
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                if !self.server_has_started {
                    ui.label("Start a server!")
                        .on_hover_text("Starting hosting a server with a click of a button!");

                    ui.allocate_ui(vec2(100., 30.), |ui| {
                        ui.horizontal_centered(|ui| {
                            ui.label(RichText::from("Port"));
                            ui.text_edit_singleline(&mut self.open_on_port);
                        });
                    });

                    let temp_open_on_port = &self.open_on_port;

                    if ui.button("Start").clicked() {
                        let server_pw = match self.server_req_password {
                            true => self.server_password.clone(),
                            false => "".to_string(),
                        };

                        //Overwrite the channel we have in the TemplateApp struct
                        self.server_shutdown_token = CancellationToken::new();

                        let token = self.server_shutdown_token.child_token();

                        //We pass in this dashmap to the server as way for the server to modify it, so that we can read it later from the ui
                        let connected_clients: std::sync::Arc<DashMap<String, ClientProfile>> =
                            self.server_connected_clients_profile.clone();

                        let shared_fields_clone = self.client_ui.shared_fields.clone();

                        //Move context so we can request_repaint
                        let ctx = ctx.clone();

                        self.server_has_started = match temp_open_on_port.parse::<i32>() {
                            Ok(port) => {
                                tokio::spawn(async move {
                                    match server::server_main(
                                        port.to_string(),
                                        server_pw,
                                        token,
                                        connected_clients,
                                        ctx,
                                    )
                                    .await
                                    {
                                        Ok(shared_fields) => {
                                            //Assign shared fields
                                            tokio::spawn(async move {
                                                *shared_fields_clone.lock().unwrap() =
                                                    shared_fields.lock().await.clone();
                                            });
                                        },
                                        Err(err) => {
                                            println!("ln 208 {:?}", err);
                                        },
                                    };
                                });
                                true
                            },
                            Err(err) => {
                                //Avoid panicking when trying to display a Notification
                                //This is very rare but can still happen
                                display_error_message(err, self.toasts.clone());

                                false
                            },
                        };
                    }

                    ui.checkbox(&mut self.server_req_password, "Set password for server");

                    if self.server_req_password {
                        ui.text_edit_singleline(&mut self.server_password);
                    }
                }
                else {
                    ui.label("Server settings");
                    if ui.button("Shutdown server").clicked() {
                        let token = self.server_shutdown_token.clone();

                        tokio::spawn(async move {
                            //Throw away error, because we already inspect it
                            token.cancel();
                        });

                        //Reset server state
                        self.server_has_started = false;
                    }

                    if self.public_ip.is_empty() {
                        let tx = self.dtx.clone();
                        std::thread::spawn(move || {
                            let combined_ips = ipv4_get()
                                .unwrap_or_else(|_| "Couldnt connect to the internet".to_string())
                                + ";"
                                + &ipv6_get().unwrap_or_else(|_| {
                                    "Couldnt connect to the internet".to_string()
                                });
                            tx.send(combined_ips.trim().to_owned())
                        });
                        match self.drx.recv() {
                            Ok(ok) => {
                                self.public_ip.clone_from(&ok);
                                //Set the ip we are connecting to so we dont need to paste it
                                let pub_ip: Vec<&str> = self.public_ip.rsplit(';').collect();

                                self.client_ui.send_on_ip =
                                    format!("[{}]:{}", pub_ip[0], self.open_on_port);
                            },
                            Err(err) => {
                                eprintln!("{}", err)
                            },
                        }
                    }

                    let pub_ip: Vec<&str> = self.public_ip.rsplit(';').collect();

                    let formatted_ip = format!("[{}]:{}", pub_ip[0], self.open_on_port);

                    ui.horizontal(|ui| {
                        ui.label("Server address (Public ipv6 address)");
                        ui.label(formatted_ip.clone());
                    });

                    if self.server_req_password && !self.server_password.is_empty() {
                        ui.label(RichText::from(format!(
                            "Password: {}",
                            self.server_password.clone()
                        )));
                    }

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label(RichText::from("Share").weak().size(self.font_size / 2.));
                        if ui.button("Copy connection link").clicked() {
                            ctx.copy_text(format!(
                                "matthias://{formatted_ip}&{}",
                                self.server_password
                            ));
                        }
                    });

                    ui.separator();

                    ui.label("Clients' profiles");
                    //Display connected users, with a Table
                    ui.allocate_ui(vec2(ui.available_width(), 200.), |ui| {
                        TableBuilder::new(ui)
                            .resizable(true)
                            .auto_shrink([false, false])
                            .striped(true)
                            .columns(Column::remainder().at_most(ctx.available_rect().width()), 5)
                            .header(25., |mut row| {
                                row.col(|ui| {
                                    ui.label("Username");
                                });
                                row.col(|ui| {
                                    ui.label("Uuid")
                                        .on_hover_text("Universally unique identifier");
                                });
                                row.col(|ui| {
                                    ui.label("Profile picture");
                                });
                                row.col(|ui| {
                                    ui.label("Actions");
                                });
                            })
                            .body(|mut body| {
                                for (key, value) in
                                    <DashMap<std::string::String, ClientProfile> as Clone>::clone(
                                        &self.server_connected_clients_profile,
                                    )
                                    .into_iter()
                                {
                                    body.row(25., |mut row| {
                                        //Username
                                        row.col(|ui| {
                                            ui.centered_and_justified(|ui| {
                                                ui.label(value.username.clone());
                                            });
                                        });
                                        //Uuid
                                        row.col(|ui| {
                                            ui.centered_and_justified(|ui| {
                                                ui.label(key.clone());
                                            });
                                        });
                                        //Profile picture
                                        row.col(|ui| {
                                            ui.centered_and_justified(|ui| {
                                                ui.add(Image::from_bytes(
                                                    format!(
                                                        "bytes://server_connect_preview_{}",
                                                        key.clone()
                                                    ),
                                                    value.small_profile_picture.clone(),
                                                ));
                                            });
                                        });
                                        //Ban button
                                        row.col(|ui| {
                                            ui.centered_and_justified(|ui| {
                                                if ui.button("Ban").clicked() {
                                                    let shared_files = self
                                                        .client_ui
                                                        .shared_fields
                                                        .lock()
                                                        .unwrap();

                                                    let mut banned_uuids = shared_files
                                                        .banned_uuids
                                                        .try_lock()
                                                        .unwrap();

                                                    if !banned_uuids
                                                        .clone()
                                                        .iter()
                                                        .any(|item| *item == key)
                                                    {
                                                        banned_uuids.push(key.clone());
                                                    };
                                                }
                                            });
                                        });
                                    });
                                }
                            });
                    });

                    ui.separator();

                    ui.label("Banneds uuids");

                    let shared_fields = self.client_ui.shared_fields.lock().unwrap();

                    match shared_fields.banned_uuids.try_lock() {
                        Ok(mut banned_uuids) => {
                            for (index, uuid) in banned_uuids.clone().iter().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.label(uuid);
                                    if ui
                                        .button(RichText::from("Unban").color(Color32::RED))
                                        .clicked()
                                    {
                                        banned_uuids.remove(index);
                                    }
                                });
                            }
                        },
                        Err(err) => {
                            tracing::error!("{}", err);
                        },
                    };
                }
            });
        });
    }
}
