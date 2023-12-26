use base64::engine::general_purpose;
use base64::Engine;

use egui::{vec2, Color32, Area, Align2, Context, LayerId, Id, RichText, Sense};

use std::fs::{self};

use std::path::PathBuf;

use crate::app::account_manager::write_file;

//use crate::app::account_manager::write_file;
use crate::app::backend::{ClientMessage, ServerFileReply, ServerMessageType, TemplateApp, ServerImageUpload};
use crate::app::client;

impl TemplateApp {
    pub fn image_overlay_draw(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &Context,
        image_bytes: Vec<u8>,
        picture: &ServerImageUpload
    ) {
        //Image overlay
        ui.painter().rect_filled(
            egui::Rect::EVERYTHING,
            0.,
            Color32::from_rgba_premultiplied(0, 0, 0, 180),
        );

        Area::new("image_overlay").movable(false).anchor(Align2::CENTER_CENTER, vec2(0., 0.)).show(&ctx, |ui|{
            ui.allocate_ui(vec2(ui.available_width() / 1.3, ui.available_height() / 1.3), |ui|{
                ui.add(egui::widgets::Image::from_bytes(
                    format!("bytes://{}", picture.index),
                    image_bytes.clone(),
                ))/*Add the same context menu as before*/.context_menu(|ui| {
                    if ui.button("Save").clicked() {
                        //always name the file ".png"
                        let image_save = ServerFileReply {
                            bytes: image_bytes.clone(),
                            file_name: PathBuf::from(".png"),
                        };
                        let _ = write_file(image_save);
                    }
                });
            });
        });
        Area::new("image_overlay_exit")
            .movable(false)
            .anchor(Align2::LEFT_TOP, vec2(30., 50.))
            .show(ctx, |ui|{
                if ui.add(egui::ImageButton::new(egui::include_image!("../../../../../icons/cross.png"))).clicked() {
                    self.image_overlay = false;
                }
            });
        
    }
    pub fn image_message_instance(
        &mut self,
        item: &crate::app::backend::ServerOutput,
        ui: &mut egui::Ui,
        ctx: &Context
    ) {
        if let ServerMessageType::Image(picture) = &item.MessageType {
            let path = PathBuf::from(format!(
                "{}\\szeChat\\Client\\{}\\Images\\{}",
                env!("APPDATA"),
                self.send_on_ip_base64_encoded,
                picture.index
            ));
            ui.allocate_ui(vec2(300., 300.), |ui| {
                match fs::read(&path) {
                    Ok(image_bytes) => {
                        //display picture from bytes
                        let image_widget = ui.add(egui::widgets::Image::from_bytes(
                            format!("bytes://{}", picture.index),
                            image_bytes.clone(),
                        ));

                        if image_widget.interact(Sense::click()).clicked() {
                            self.image_overlay = true;
                        }

                        image_widget.context_menu(|ui| {
                            if ui.button("Save").clicked() {
                                //always name the file ".png", NOTE: USE WRITE FILE BECAUSE WRITE IMAGE IS AUTOMATIC WITHOUT ASKING THE USER
                                let image_save = ServerFileReply {
                                    bytes: image_bytes.clone(),
                                    file_name: PathBuf::from("image.png"),
                                };
                                let _ = write_file(image_save);
                            }
                        });

                        if self.image_overlay {
                            self.image_overlay_draw(ui, ctx, image_bytes, picture);
                        }
                    }
                    Err(_err) => {
                        //create decoy file, to manually create a race condition
                        let _ = fs::create_dir_all(PathBuf::from(format!(
                            "{}\\szeChat\\Client\\{}\\Images",
                            env!("APPDATA"),
                            self.send_on_ip_base64_encoded,
                        )));

                        if let Err(err) = std::fs::write(
                            path,
                            "This is a placeholder file, this will get overwritten (hopefully)",
                        ) {
                            println!("Error when creating a decoy: {err}");
                            return;
                        };
                        
                        //check if we are visible
                        if !ui.is_visible() {
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
                        
                        tokio::spawn(async move {
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
                        });
                    }
                };
            });
        }
    }
}
