use crate::app::backend::{display_error_message, TemplateApp};
use crate::app::backend::{ipv4_get, ipv6_get};
use crate::app::server;
use egui::{vec2, Align, Layout, RichText};
use tap::Tap;
use tokio::sync;
use tokio_util::sync::CancellationToken;

impl TemplateApp {
    pub fn server_setup_ui(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Host a server!", |ui| {
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                if !self.server_has_started {
                    ui.label(RichText::from("Server setup"));

                    ui.allocate_ui(vec2(100., 30.), |ui| {
                        ui.horizontal_centered(|ui| {
                            ui.label(RichText::from("Port"));
                            ui.text_edit_singleline(&mut self.open_on_port);
                        });
                    });

                    let temp_open_on_port = &self.open_on_port;

                    if ui.button("Start server").clicked() {
                        let server_pw = match self.server_req_password {
                            true => self.server_password.clone(),
                            false => "".to_string(),
                        };

                        //Overwrite the channel we have in the TemplateApp struct
                        self.server_shutdown_token = CancellationToken::new();

                        let token = self.server_shutdown_token.child_token();

                        self.server_has_started = match temp_open_on_port.parse::<i32>() {
                            Ok(port) => {
                                tokio::spawn(async move {
                                    match server::server_main(port.to_string(), server_pw, token)
                                        .await
                                    {
                                        Ok(_temp_stuff) => {}
                                        Err(err) => {
                                            println!("ln 208 {:?}", err);
                                        }
                                    };
                                });
                                true
                            }
                            Err(err) => {
                                display_error_message(err);

                                false
                            }
                        };
                    }

                    ui.checkbox(&mut self.server_req_password, "Server requires password");

                    if self.server_req_password {
                        ui.text_edit_singleline(&mut self.server_password);
                    }
                } else {
                    if ui.button("Shutdown").clicked() {
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
                                self.public_ip = ok.clone();
                                //Set the ip we are connecting to so we dont need to paste it
                                let pub_ip: Vec<&str> = self.public_ip.rsplit(';').collect();

                                self.client_ui.send_on_ip =
                                    format!("[{}]:{}", pub_ip[0], self.open_on_port);
                            }
                            Err(err) => {
                                eprintln!("{}", err)
                            }
                        }
                    }

                    let pub_ip: Vec<&str> = self.public_ip.rsplit(';').collect();

                    ui.horizontal(|ui| {
                        ui.label("Server address (Public ipv6 address)");
                        ui.text_edit_singleline(&mut format!(
                            "[{}]:{}",
                            pub_ip[0], self.open_on_port
                        ));
                    });

                    if self.server_req_password && !self.server_password.is_empty() {
                        ui.label(RichText::from(format!(
                            "Password: {}",
                            self.server_password
                        )));
                    }
                }
            });
        });
    }
}
