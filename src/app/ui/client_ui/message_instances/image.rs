use base64::engine::general_purpose;
use base64::Engine;

use egui::vec2;

use std::fs::{self};

use std::path::PathBuf;

use crate::app::account_manager::write_file;

//use crate::app::account_manager::write_file;
use crate::app::backend::{ClientMessage, ServerFileReply, ServerMessageType, TemplateApp};
use crate::app::client;

impl TemplateApp {
    pub fn image_message_instance(
        &mut self,
        item: &crate::app::backend::ServerOutput,
        ui: &mut egui::Ui,
    ) {
        if let ServerMessageType::Image(picture) = &item.MessageType {
            ui.allocate_ui(vec2(300., 300.), |ui| {
                match fs::read(format!(
                    "{}\\szeChat\\Client\\{}\\Images\\{}",
                    env!("APPDATA"),
                    self.send_on_ip_base64_encoded,
                    picture.index
                )) {
                    Ok(image_bytes) => {
                        //display picture from bytes
                        ui.add(egui::widgets::Image::from_bytes(
                            format!("bytes://{}", picture.index),
                            image_bytes.clone(),
                        ))
                        .context_menu(|ui| {
                            if ui.button("Save").clicked() {
                                //always name the file ".png"
                                let image_save = ServerFileReply {
                                    bytes: image_bytes,
                                    file_name: PathBuf::from(".png"),
                                };
                                let _ = write_file(image_save);
                            }
                        });
                    }
                    Err(_err) => {
                        //check if we are visible
                        if !ui.is_visible() || !self.requests.image {
                            return;
                        }

                        //We dont have file on our local system so we have to ask the server to provide it
                        let passw = self.client_password.clone();
                        let author = self.login_username.clone();
                        let send_on_ip = self.send_on_ip.clone();
                        let sender = self.itx.clone();

                        let message = ClientMessage::construct_image_request_msg(
                            picture.index,
                            passw,
                            author,
                            send_on_ip,
                        );

                        self.requests.image = tokio::spawn(async move {
                            match client::send_msg(message).await {
                                Ok(ok) => {
                                    match sender.send(ok) {
                                        Ok(_) => {}
                                        Err(err) => {
                                            println!("{}", err);
                                        }
                                    };
                                }
                                Err(err) => {
                                    println!("{err} ln 264")
                                }
                            }
                        })
                        .is_finished();
                    }
                };
            });
        }
    }
}
