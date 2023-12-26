use device_query::Keycode;
use egui::{vec2, Align, Align2, Area, Color32, FontFamily, FontId, Id, Layout, Pos2, Stroke, LayerId};
use image::Frame;
use windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW;
use windows_sys::w;

use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::Duration;

use crate::app::account_manager::{write_audio, write_file, write_image};

//use crate::app::account_manager::write_file;
use crate::app::backend::{
    ClientMessage, ServerAudioReply, ServerFileReply, ServerImageReply, ServerMaster, TemplateApp,
};
use crate::app::client::{self};

impl TemplateApp {
    pub fn state_client(
        &mut self,
        _frame: &mut eframe::Frame,
        ctx: &egui::Context,
        input_keys: Vec<Keycode>,
    ) {
        //set multiline mode
        //IMPORTANT: unused
        if self.usr_msg.trim().lines().count() > 1
        || /*try to detect a new line caused by char lenght */
            (self.usr_msg.len() as f32 * (self.font_size / 2.)) > _frame.info().window_info.size[0] / 1.4
        {
            self.multiline_mode = true;
        } else {
            self.multiline_mode = false;
        }

        let should_be_running = self.autosync_should_run.clone();
        let rx = self.autosync_sender.get_or_insert_with(|| {
            let (tx, rx) = mpsc::channel::<String>();

            let message = ClientMessage::construct_sync_msg(
                self.send_on_ip.clone(),
                self.client_password.clone(),
                self.login_username.clone(),
            );

            tokio::spawn(async move {
                while should_be_running.load(Ordering::Relaxed) {
                    tokio::time::sleep(Duration::from_secs_f32(2.)).await;
                    //This is where the messages get recieved
                    match client::send_msg(message.clone()).await {
                        Ok(ok) => {
                            match tx.send(ok) {
                                Ok(_) => {}
                                Err(err) => {
                                    println!("{} ln 57", err);
                                    break;
                                }
                            };
                        }
                        Err(_err) => {
                            //println!("ln 197 {:?}", err.source());
                            break;
                        }
                    };
                }
            });
            rx
        });

        //Get sent to the channel to be displayed
        match rx.try_recv() {
            Ok(msg) => {
                //show messages
                ctx.request_repaint();
                let incoming_struct: Result<ServerMaster, serde_json::Error> =
                    serde_json::from_str(&msg);
                if let Ok(ok) = incoming_struct {
                    self.incoming_msg = ok;
                }
            }
            Err(_err) => {
                //println!("{}", _err)
            }
        }

        egui::TopBottomPanel::new(egui::panel::TopBottomSide::Top, "setting_area").show(
            ctx,
            |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.allocate_ui(vec2(300., 40.), |ui| {
                        if ui
                            .add(egui::widgets::ImageButton::new(egui::include_image!(
                                "../../../icons/logout.png"
                            )))
                            .clicked()
                        {
                            //shut down sync service
                            self.autosync_should_run.store(false, Ordering::Relaxed);
                            self.autosync_sender = None;

                            self.client_mode = false;
                        };
                    })
                    .response
                    .on_hover_text("Logout");
                    ui.allocate_ui(vec2(300., 40.), |ui| {
                        if ui
                            .add(egui::widgets::ImageButton::new(egui::include_image!(
                                "../../../icons/settings.png"
                            )))
                            .clicked()
                        {
                            self.settings_window = !self.settings_window;
                        };
                    });
                });

                ui.allocate_space(vec2(ui.available_width(), 5.));
            },
        );

        //msg_area
        egui::CentralPanel::default().show(ctx, |ui| {
            //Drop file warning
            self.drop_file_animation =
                    ui.input(|input| !input.raw.clone().hovered_files.is_empty());
                if self.how_on >= 0. {
                    let window_size = ui.input(|reader| reader.screen_rect().max).to_vec2();
                    let font_id = FontId {
                        family: FontFamily::default(),
                        size: self.font_size,
                    };

                    ui.painter().rect_filled(
                        egui::Rect::EVERYTHING,
                        0.,
                        Color32::from_rgba_premultiplied(0, 0, 0, (self.how_on / 3.) as u8),
                    );

                    Area::new("warning_overlay").show(ctx, |ui| {
                        ui.painter().rect(
                            egui::Rect {
                                min: Pos2::new(
                                    window_size[0] / 3.,
                                    window_size[0] / 5. + self.how_on / 50.,
                                ),
                                max: Pos2::new(
                                    window_size[0] / 1.5,
                                    window_size[0] / 3. + self.how_on / 50.,
                                ),
                            },
                            5.0,
                            Color32::from_rgba_unmultiplied(0, 0, 0, self.how_on as u8 / 8),
                            Stroke::default(),
                        );
                        ui.painter().text(
                            Pos2::new(window_size[0] / 2., window_size[0] / 4. + self.how_on / 50.),
                            Align2([Align::Center, Align::Center]),
                            "Drop to upload",
                            font_id,
                            Color32::from_rgba_unmultiplied(255, 255, 255, self.how_on as u8),
                        );
                    });
                }
                self.how_on = ctx.animate_value_with_time(
                    Id::from("warning_overlay"),
                    match self.drop_file_animation {
                        true => 255.,
                        false => 0.,
                    },
                    0.4,
                );

                let dropped_files = ui.input(|reader| reader.raw.clone().dropped_files);
                if !dropped_files.is_empty() {
                    let dropped_file_path = dropped_files[0].path.clone().unwrap_or_default();

                    self.files_to_send.push(dropped_file_path);
                }
                
            //Messages go here
            self.client_ui_message_main(ui, ctx);
        });

        //usr_input
        let usr_panel = egui::TopBottomPanel::bottom("usr_input")
            .max_height(_frame.info().window_info.size[1] / 2.)
            .show_animated(ctx, self.usr_msg_expanded, |ui| {
                let msg_tray = self.message_tray(ui, ctx, input_keys);

                self.text_widget_offset = msg_tray.response.rect.width();

                ui.allocate_space(vec2(ui.available_width(), 5.));
            });

        self.file_tray(ctx);

        let panel_height = match usr_panel {
            Some(panel) => panel.response.rect.size()[1],
            None => 0.,
        };

        Area::new("usr_msg_expand")
            .anchor(
                Align2::RIGHT_BOTTOM,
                match self.usr_msg_expanded {
                    true => vec2(-41.0, (-panel_height - 10.) / 14.),
                    false => vec2(-41.0, -10.),
                },
            )
            .show(ctx, |ui| {
                ui.allocate_ui(vec2(25., 25.), |ui| {
                    if ui
                        .add(egui::ImageButton::new(egui::include_image!(
                            "../../../icons/cross.png"
                        )))
                        .clicked()
                    {
                        self.usr_msg_expanded = !self.usr_msg_expanded;
                    };
                });
            });

        //Recivers
        match self.rx.try_recv() {
            Ok(msg) => {
                let incoming_struct: Result<ServerMaster, serde_json::Error> =
                    serde_json::from_str(&msg);
                match incoming_struct {
                    Ok(ok) => {
                        self.invalid_password = false;
                        self.incoming_msg = ok;
                    }
                    Err(_error) => {
                        //Funny server response check, this must match what server replies when inv passw
                        if msg == "Invalid Password!" {
                            //Reset messages
                            self.incoming_msg = ServerMaster::default();

                            //Set bools etc.
                            self.invalid_password = true;
                            self.settings_window = true;
                        }
                    }
                }
            }
            Err(_err) => {
                //println!("ln 332 {}", err);
            }
        }

        match self.frx.try_recv() {
            Ok(msg) => {
                let file_serve: Result<ServerFileReply, serde_json::Error> =
                    serde_json::from_str(&msg);
                let _ = write_file(file_serve.unwrap());
            }
            Err(_err) => {}
        }

        match self.irx.try_recv() {
            Ok(msg) => {
                let file_serve: Result<ServerImageReply, serde_json::Error> =
                    serde_json::from_str(&msg);

                let _ = write_image(file_serve.as_ref().unwrap(), self.send_on_ip.clone());

                //The default uri is => "bytes://{index}", we need to forget said image to clear it from cache, therefor load the corrected file. becuase it has cached the placeholder 
                ctx.forget_image(&format!("bytes://{}", file_serve.unwrap().index));
            }
            Err(_err) => {}
        }

        match self.audio_save_rx.try_recv() {
            Ok(msg) => {
                let file_serve: Result<ServerAudioReply, serde_json::Error> =
                    serde_json::from_str(&msg);
                let _ = write_audio(file_serve.unwrap(), self.send_on_ip.clone());
            }
            Err(_err) => {}
        }
    }
}
