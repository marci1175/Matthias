use chrono::Utc;
use device_query::Keycode;
use egui::epaint::RectShape;
use egui::{
    vec2, Align, Align2, Area, Button, Color32, FontFamily, FontId, Id, ImageButton, Layout, Pos2,
    RichText, Stroke, TextBuffer, Ui, Response,
};

use rand::Rng;
use regex::Regex;
use rfd::FileDialog;
use std::f32::consts::E;
use std::ffi::OsStr;
use std::fs::{self};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::Duration;
use windows_sys::w;
use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONSTOP};

use std::sync::mpsc;

use crate::app::account_manager::write_file;
//use crate::app::account_manager::write_file;
use crate::app::backend::{
    FileRequest, FileServe, Message, ServerMaster, ServerMessageType, TemplateApp,
};
use crate::app::client::{self};

impl TemplateApp {
    pub fn state_client(
        &mut self,
        _frame: &mut eframe::Frame,
        ctx: &egui::Context,
        input_keys: Vec<Keycode>,
    ) {
        let should_be_running = self.autosync_should_run.clone();
        let rx = self.autosync_sender.get_or_insert_with(|| {
            let (tx, rx) = mpsc::channel::<String>();

            let message = Message::construct_sync_msg(
                self.send_on_ip.clone(),
                self.client_password.clone(),
                self.login_username.clone(),
                None,
            );

            tokio::spawn(async move {
                while should_be_running.load(Ordering::Relaxed) {
                    tokio::time::sleep(Duration::from_secs_f32(2.)).await;
                    println!("requested sync!");
                    match client::send_msg(message.clone()).await {
                        Ok(ok) => {
                            match tx.send(ok) {
                                Ok(_) => {}
                                Err(err) => {
                                    println!("{}", err);
                                }
                            };
                        }
                        Err(err) => {
                            println!("ln 197 {:?}", err.source());
                            break;
                        }
                    };
                }
            });
            rx
        });

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

            if ui.input(|input| !input.raw.clone().hovered_files.is_empty() ) {
                self.drop_file_animation = true;

            }
            else {

                self.drop_file_animation = false;

            }
            if self.how_on >= 0. {
                let window_size = ui.input(|reader| {reader.screen_rect().max}).to_vec2();
                let font_id = FontId {
                    family: FontFamily::default(),
                    size: self.font_size,
                };

                Area::new("drop_warning").show(ctx, |ui|{
                    ui.painter()
                        .rect(egui::Rect { min: Pos2::new(window_size[0] / 3., window_size[0] / 5. + self.how_on / 50.), max: Pos2::new(window_size[0] / 1.5, window_size[0] / 3. + self.how_on / 50.) }, 5.0, Color32::from_rgba_unmultiplied(0, 0, 0, self.how_on as u8 / 8), Stroke::default());
                    ui.painter()
                        .text(Pos2::new(window_size[0] / 2., window_size[0] / 4. + self.how_on / 50.), Align2([Align::Center, Align::Center]), "Drop to upload", font_id, Color32::from_rgba_unmultiplied(255, 255, 255, self.how_on as u8));
                });
            }
            self.how_on = ctx.animate_value_with_time(Id::from("drop_warning"), match self.drop_file_animation {
                true => 255.,
                false => 0.
            }, 0.4);

            let dropped_files = ui.input(|reader| {reader.raw.clone().dropped_files});
            if !dropped_files.is_empty() {
                let dropped_file_path = dropped_files[0].path.clone().unwrap_or_default();

                self.files_to_send.push(dropped_file_path);

            }

            //Messages go here
            ui.allocate_ui(
                match self.usr_msg_expanded {
                    true => vec2(
                        ui.available_width(),
                        ui.available_height() - (_frame.info().window_info.size[1] / 5. + 10.),
                    ),
                    false => vec2(ui.available_width(), ui.available_height()),
                },
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_source("msg_area")
                        .stick_to_bottom(true)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.allocate_ui(ui.available_size(), |ui| {
                                if self.send_on_ip.is_empty() {
                                    ui.with_layout(Layout::centered_and_justified(egui::Direction::TopDown), |ui|{
                                        ui.label(RichText::from("To start chatting go to settings and set the IP to the server you want to connect to!").size(self.font_size).color(Color32::LIGHT_BLUE));
                                    });
                                }
                                let mut test: Vec<Response> = Vec::new();
                                let mut has_been_reply_clicked = (false, 0);

                                for (index, item) in self.incoming_msg.clone().struct_list.iter().enumerate() {
                                    let mut i: &String = &Default::default();
                                    if let ServerMessageType::Normal(item) = &item.MessageType {
                                        i = &item.message;
                                    }
                                    let fasz = ui.group(|ui|
                                    {
                                        if let Some(replied_to) = item.replying_to {
                                            if ui.add(egui::widgets::Button::new(RichText::from(format!("Replying to: {}: {}",
                                                self.incoming_msg.struct_list[replied_to].Author,
                                                match &self.incoming_msg.struct_list[replied_to].MessageType {
                                                    ServerMessageType::Image(_img) => format!("Image"),
                                                    ServerMessageType::Upload(upload) => format!("Upload {}", upload.file_name),
                                                    ServerMessageType::Normal(msg) => {
                                                        let mut message_clone = msg.message.clone();
                                                        if message_clone.clone().len() > 20 {
                                                            message_clone.truncate(20);
                                                            message_clone.push_str(" ...");
                                                        }
                                                        format!("{}", message_clone)
                                                },
                                            })
                                            ).size(self.font_size / 1.5))
                                                .frame(false))
                                                    .clicked() {
                                                        //implement scrolling to message
                                                        has_been_reply_clicked = (true, replied_to);
                                                    }
                                        }
                                        ui.label(RichText::from(format!("{}", item.Author)).size(self.font_size / 1.3).color(Color32::WHITE));
                                            if (i.contains('[') && i.contains(']'))
                                            && (i.contains('(') && i.contains(')'))
                                        {
                                            let re = Regex::new(
                                                r"\[(?P<link_text>[^\]]*)\]\((?P<link_target>[^)]+)\)",
                                            )
                                            .unwrap();
                                            let mut captures: Vec<String> = Vec::new();
                                            for capture in re.captures_iter(&i) {
                                                for i in 1..capture.len() {
                                                    captures.push(capture[i].to_string());
                                                }
                                            }
                                            if captures.is_empty() {
                                                ui.label(RichText::from(i).size(self.font_size));
                                            } else {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        RichText::from(re.replace_all::<&str>(&i, ""))
                                                            .size(self.font_size),
                                                    );
                                                    for i in (0..captures.len()).step_by(2) {
                                                        ui.add(egui::Hyperlink::from_label_and_url(
                                                            RichText::from(&captures[i])
                                                                .size(self.font_size),
                                                            &captures[i + 1],
                                                        ));
                                                    }
                                                });
                                            }
                                        } else if i.contains('#') && i.rmatches('#').count() <= 5 {
                                            let split_lines = i.rsplit_once('#').unwrap();
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::from(split_lines.0.replace('#', ""))
                                                        .size(self.font_size),
                                                );
                                                ui.label(
                                                    RichText::from(split_lines.1).strong().size(
                                                        self.font_size
                                                            * match i
                                                                .rmatches('#')
                                                                .collect::<Vec<&str>>()
                                                                .len()
                                                            {
                                                                1 => 2.0,
                                                                2 => 1.8,
                                                                3 => 1.6,
                                                                4 => 1.4,
                                                                5 => 1.2,
                                                                _ => 1.,
                                                            }
                                                                as f32,
                                                    ),
                                                );
                                            });
                                        } else {
                                            ui.label(RichText::from(i).size(self.font_size));
                                        }
                                    if let ServerMessageType::Upload(file) = &item.MessageType {
                                        if ui.button(RichText::from(format!("{}", file.file_name)).size(self.font_size)).clicked() {
                                            //let rx = self.autosync_sender.get_or_insert_with(|| {
                                            let passw = self.client_password.clone();
                                            let author = self.login_username.clone();
                                            let send_on_ip = self.send_on_ip.clone();
                                            let sender = self.ftx.clone();
                                            let replying_to = self.replying_to.clone();

                                            let message = Message::construct_file_request_msg(file.index, passw, author, send_on_ip, replying_to);

                                            tokio::spawn(async move {
                                                match client::send_msg(message).await {
                                                    Ok(ok) => {
                                                        match sender.send(ok) {
                                                            Ok(_) => {}
                                                            Err(err) => {
                                                                println!("{}", err);
                                                            }
                                                        };
                                                    },
                                                    Err(err) => {
                                                        println!("{err} ln 264")
                                                    }
                                                }
                                            });
                                        }
                                    }
                                    ui.label(RichText::from(format!("{}", item.MessageDate)).size(self.font_size / 1.5).color(Color32::DARK_GRAY));
                                }
                                ).response.context_menu(|ui|{
                                    if ui.button("Reply").clicked() {
                                        self.replying_to = Some(index);
                                    }
                                    if ui.button("Copy text").clicked() {
                                        ctx.copy_text(i.clone());
                                    };
                                });

                                test.push(fasz);
                                if has_been_reply_clicked.0 {
                                    test[has_been_reply_clicked.1].scroll_to_me(Some(Align::Center));
                                }
                                };
                            });
                            
                            if !self.usr_msg_expanded {
                                ui.allocate_space(vec2(ui.available_width(), 25.));
                            }
                        });
                },
            );
        });

        //usr_input
        let usr_panel = egui::TopBottomPanel::bottom("usr_input").show_animated(ctx, self.usr_msg_expanded, |ui| {
            ui.allocate_space(vec2(ui.available_width(), 5.));
            if !self.files_to_send.is_empty() {
                egui::ScrollArea::horizontal()
                    .id_source("file_to_send")
                    .stick_to_right(true)
                    .show(ui, |ui|{
                        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                            for (index, item) in self.files_to_send.clone().iter().enumerate() {
                                ui.group(|ui| {
                                    ui.allocate_ui(vec2(200., 100.), |ui| {
                                        ui.with_layout(Layout::left_to_right(Align::Center), |ui|{
                                            ui.with_layout(Layout::top_down(Align::Center), |ui| {

                                                //file icon
                                                ui.allocate_ui(vec2(75., 75.), |ui|{
                                                    match item.extension().unwrap().to_string_lossy().to_ascii_lowercase().as_str() {
                                                        //file extenisons
                                                        "exe" | "msi" | "cmd" | "com" | "inf" | "bat" | "ipa" | "osx" | "pif" => {
                                                            ui.add(egui::widgets::Image::new(egui::include_image!("../../../icons/file_types/exe_icon.png")));
                                                        }
                                                        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "z" | "tgz" | "tbz2" | "txz" | "sit" | "tar.gz" | "tar.bz2" | "tar.xz" | "zipp" => {
                                                            ui.add(egui::widgets::Image::new(egui::include_image!("../../../icons/file_types/zip_icon.png")));
                                                        }
                                                        "jpeg" | "jpg" | "png" | "gif" | "bmp" | "tiff" | "webp" | "svg" | "ico" | "raw" | "heif" | "pdf" | "eps" | "ai" | "psd" => {
                                                            ui.add(egui::widgets::Image::new(egui::include_image!("../../../icons/file_types/picture_icon.png")));
                                                        }
                                                        "wav" | "mp3" | "ogg" | "flac" | "aac" | "midi" | "wma" | "aiff" | "ape" | "alac" | "amr" | "caf" | "au" | "ra" | "m4a" | "ac3" | "dts" => {
                                                            ui.add(egui::widgets::Image::new(egui::include_image!("../../../icons/file_types/sound_icon.png")));
                                                        }
                                                        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "3gp" | "mpeg" | "mpg" | "rm" | "swf" | "vob" | "ts" | "m2ts" | "mts" | "divx" => {
                                                            ui.add(egui::widgets::Image::new(egui::include_image!("../../../icons/file_types/video_icon.png")));
                                                        }

                                                        // :)
                                                        "rs" => {
                                                            ui.add(egui::widgets::Image::new(egui::include_image!("../../../icons/file_types/rust_lang_icon.png")));
                                                        }

                                                        _ => {
                                                            ui.add(egui::widgets::Image::new(egui::include_image!("../../../icons/file_types/general_icon.png")));
                                                        }
                                                    }
                                                });

                                                //selected file widget part
                                                ui.label(
                                                    RichText::from(
                                                        item.file_name()
                                                            .unwrap_or_default()
                                                            .to_string_lossy(),
                                                    )
                                                    .size(self.font_size),
                                                );
                                            });
                                            ui.separator();

                                            //bin icon
                                            ui.allocate_ui(vec2(30., 30.), |ui|{
                                                if ui.add(
                                                    ImageButton::new(
                                                        egui::include_image!("../../../icons/bin.png")
                                                    )
                                                ).clicked() {
                                                    self.files_to_send.remove(index);
                                                };
                                            });
                                        });
                                    });
                                });
                            }
                        });
                    });
                ui.separator();
            }
            if let Some(replying_to) = self.replying_to.clone() {
                ui.horizontal(|ui| {
                    ui.group(|ui|{
                        ui.allocate_ui(vec2(ui.available_width(), self.font_size), |ui|{
                            //place them in one line
                            ui.horizontal(|ui| {
                                ui.label(RichText::from("Replying to:").size(self.font_size).weak());
                                ui.label(RichText::from(match &self.incoming_msg.struct_list[replying_to].MessageType {
                                    ServerMessageType::Image(_img) => format!("Image"),
                                    ServerMessageType::Upload(upload) => format!("Upload {}", upload.file_name),

                                    ServerMessageType::Normal(msg) => msg.message.clone(),

                                }).size(self.font_size).strong());
                            });
                        });
                    });
                    if ui.button(RichText::from("X").size(self.font_size * 1.5).color(Color32::RED)).clicked() {
                        self.replying_to = None;
                    };
                });
                ui.allocate_space(vec2(ui.available_width(), 10.));
            }
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                ui.allocate_ui(
                    vec2(
                        ui.available_width() - 100.,
                        _frame.info().window_info.size[1] / 5.,
                    ),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .id_source("usr_input")
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                ui.with_layout(
                                    egui::Layout::top_down_justified(Align::Center),
                                    |ui| {
                                        ui.add_sized(
                                            ui.available_size(),
                                            egui::TextEdit::multiline(&mut self.usr_msg)
                                                .hint_text(
                                                    RichText::from(format!(
                                                        "Message : {}",
                                                        self.send_on_ip
                                                    ))
                                                    .size(self.font_size),
                                                )
                                                .font(FontId::new(
                                                    self.font_size,
                                                    FontFamily::default(),
                                                )),
                                        );
                                    },
                                );
                            });
                    },
                );

                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.allocate_ui(vec2(50., 50.), |ui| {
                        if ui
                            .add(egui::widgets::ImageButton::new(egui::include_image!(
                                "../../../icons/send_msg.png"
                            )))
                            .clicked()
                            || input_keys.contains(&Keycode::Enter) && !(input_keys.contains(&Keycode::LShift) || input_keys.contains(&Keycode::RShift))
                        {
                            if !(self.usr_msg.trim().is_empty() || self.usr_msg.trim_end_matches('\n').is_empty()) {
                                let temp_msg = self.usr_msg.clone();
                                let tx = self.tx.clone();
                                let username = self.login_username.clone();
                                //disable pass if its not ticked
                                let passw = match self.req_passw {
                                    true => self.client_password.clone(),
                                    false => "".into(),
                                };
                                let temp_ip = self.send_on_ip.clone();
                                let replying_to = self.replying_to.clone();

                                tokio::spawn(async move {
                                    match client::send_msg(Message::construct_normal_msg(
                                        &temp_msg, temp_ip, passw, username, replying_to,
                                    ))
                                    .await
                                    {
                                        Ok(ok) => {
                                            match tx.send(ok) {
                                                Ok(_) => {}
                                                Err(err) => {
                                                    println!("{}", err);
                                                }
                                            };
                                        }
                                        Err(err) => {
                                            println!("ln 321 {:?}", err.source());
                                        }
                                    };
                                });
                            }

                            self.replying_to = None;
                            self.usr_msg.clear();

                            for file_path in self.files_to_send.clone() {
                                self.send_file(file_path);
                            }

                            //clear vectors
                            self.files_to_send.clear();
                        }
                    });
                    ui.allocate_ui(vec2(50., 50.), |ui| {
                        if ui
                            .add(egui::widgets::ImageButton::new(egui::include_image!(
                                "../../../icons/add_file.png"
                            )))
                            .on_hover_text("Send files")
                            .clicked()
                        {
                            let files = FileDialog::new()
                                .set_title("Pick a file")
                                .set_directory("/")
                                .pick_file();

                            if let Some(file) = files {
                                //send file
                                self.files_to_send.push(file);
                            }
                        }
                    });
                    ui.allocate_ui(vec2(37., 37.), |ui| {
                        let button =
                            ui.add(Button::new(RichText::from(&self.random_emoji).size(45.)));

                        if button.clicked() {
                            self.emoji_mode = !self.emoji_mode;
                        };

                        if button.hovered() {
                            if !self.random_generated {
                                let random_number =
                                    self.rand_eng.gen_range(0..=self.emoji.len() - 1);
                                self.random_emoji = self.emoji[random_number].clone();
                                self.random_generated = true;
                            }
                        } else {
                            //check if button has been unhovered, reset variable
                            self.random_generated = false;
                        }
                    });
                });
            });

            //receive server answer unconditionally
            match self.rx.try_recv() {
                Ok(msg) => {
                    let incoming_struct: Result<ServerMaster, serde_json::Error> =
                        serde_json::from_str(&msg);
                    if let Ok(ok) = incoming_struct {
                        self.incoming_msg = ok;
                    }
                }
                Err(_err) => {
                    //println!("ln 332 {}", err);
                }
            };
            match self.frx.try_recv() {
                Ok(msg) => {
                    let file_serve: Result<FileServe, serde_json::Error> = serde_json::from_str(&msg);
                    let _ = write_file(file_serve.unwrap());
                },
                Err(err) => {}
            }
            ui.allocate_space(vec2(ui.available_width(), 5.));
        });

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
    }

    fn send_file(&mut self, file: std::path::PathBuf) {
        let passw = self.client_password.clone();
        let ip = self.send_on_ip.clone();
        let author = self.login_username.clone();
        let replying_to = self.replying_to.clone();

        let message = Message::construct_file_msg(file, ip, passw, author, replying_to);

        tokio::spawn(async move {
            let _ = client::send_msg(message).await;
        });
    }
}
