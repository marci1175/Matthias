use base64::engine::general_purpose;
use base64::Engine;
use device_query::Keycode;
use egui::{
    vec2, Align, Align2, Area, Button, Color32, FontFamily, FontId, FontSelection, Id, ImageButton,
    Layout, Pos2, Response, RichText, Rounding, Stroke, TextBuffer,
};
use rand::Rng;
use regex::Regex;
use rfd::FileDialog;
use std::f32::consts::E;
use std::fs::{self};
use std::sync::atomic::Ordering;
use std::time::Duration;

use std::sync::{mpsc, Arc};

use crate::app::account_manager::{write_file, write_image};
//use crate::app::account_manager::write_file;
use crate::app::backend::{
    ClientMessage, ServerFileReply, ServerImageReply, ServerMaster, ServerMessageType, TemplateApp,
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
                println!("{}", _err)
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

                ui.painter().rect_filled(egui::Rect::EVERYTHING, 0., Color32::from_rgba_premultiplied(0, 0, 0, (self.how_on / 3.) as u8));
                
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
                                let mut reply_to_got_to = (false, 0);

                                for (index, item) in self.incoming_msg.clone().struct_list.iter().enumerate() {
                                    
                                    let mut i: &String = &Default::default();

                                    if let ServerMessageType::Normal(item) = &item.MessageType {
                                        i = &item.message;
                                    }

                                    let message_group = ui.group(|ui|
                                    {
                                        ui.allocate_ui(vec2(ui.available_width(), self.font_size), |ui|{
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
                                                            reply_to_got_to = (true, replied_to);
                                                            
                                                        }
                                            }
                                        });
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

                                                let message = ClientMessage::construct_file_request_msg(file.index, passw, author, send_on_ip);

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

                                        if let ServerMessageType::Image(picture) = &item.MessageType {
                                        ui.allocate_ui(vec2(300., 300.), |ui|{

                                            match fs::read(format!("{}\\szeChat\\Client\\{}\\{}", env!("APPDATA"), general_purpose::URL_SAFE_NO_PAD.encode(self.send_on_ip.clone()), picture.index)){
                                                Ok(image_bytes) => {
                                                    
                                                    //display picture from bytes
                                                    ui.add(egui::widgets::Image::from_bytes(format!("bytes://{}", picture.index), image_bytes));
                                                
                                                },
                                                Err(_err) => {

                                                    //check if we are visible
                                                    if !ui.is_visible() {
                                                        return;
                                                    }

                                                    //We dont have file on our local system so we have to ask the server to provide it
                                                    let passw = self.client_password.clone();
                                                    let author = self.login_username.clone();
                                                    let send_on_ip = self.send_on_ip.clone();
                                                    let sender = self.itx.clone();
                                                    

                                                    let message = ClientMessage::construct_image_request_msg(picture.index, passw, author, send_on_ip);

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

                                                },
                                            };
                                            
                                        });
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

                                    //this functions for the reply autoscroll
                                    test.push(message_group);
                                    if reply_to_got_to.0 {
                                        test[reply_to_got_to.1].scroll_to_me(Some(Align::Center));
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
        let usr_panel = egui::TopBottomPanel::bottom("usr_input")
            .max_height(_frame.info().window_info.size[1] / 2.)
            .show_animated(ctx, self.usr_msg_expanded, |ui| {
                ui.allocate_space(vec2(ui.available_width(), 5.));

                let frame_rect = ui.max_rect().shrink(5.0);
                let code_rect = frame_rect.shrink(10.);

                ui.painter().rect(
                    frame_rect,
                    Rounding::same(5.0),
                    Color32::BLACK,
                    Stroke::NONE,
                );

                let mut frame_ui = ui.child_ui(code_rect, Layout::default());

                let text_widget = egui::TextEdit::multiline(&mut self.usr_msg)
                    .font(FontId {
                        size: self.font_size,
                        family: FontFamily::default(),
                    })
                    .hint_text(format!("Message to: {}", self.send_on_ip))
                    .desired_width(ui.available_width() - self.text_widget_offset * 1.3)
                    .desired_rows(0)
                    .frame(false);

                let msg_scroll = egui::ScrollArea::vertical()
                    .id_source("usr_input")
                    .stick_to_bottom(true)
                    .auto_shrink([false, true])
                    .min_scrolled_height(self.font_size * 2.)
                    .show(&mut frame_ui, |ui| ui.add(text_widget));

                ui.allocate_space(vec2(
                    ui.available_width(),
                    msg_scroll.inner.rect.height() + 15.,
                ));

                let msg_tray = Area::new("msg_action_tray")
                    .anchor(
                        Align2::RIGHT_BOTTOM,
                        vec2(-30., -msg_scroll.content_size.y / 2. - 2.),
                    )
                    .show(ctx, |ui| {
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.allocate_ui(
                                vec2(self.font_size * 1.5, self.font_size * 1.5),
                                |ui| {
                                    if ui
                                        .add(egui::widgets::ImageButton::new(egui::include_image!(
                                            "../../../icons/send_msg.png"
                                        )))
                                        .clicked()
                                        || input_keys.contains(&Keycode::Enter)
                                            && !(input_keys.contains(&Keycode::LShift)
                                                || input_keys.contains(&Keycode::RShift))
                                    {
                                        if !(self.usr_msg.trim().is_empty()
                                            || self.usr_msg.trim_end_matches('\n').is_empty())
                                        {
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
                                                match client::send_msg(
                                                    ClientMessage::construct_normal_msg(
                                                        &temp_msg,
                                                        temp_ip,
                                                        passw,
                                                        username,
                                                        replying_to,
                                                    ),
                                                )
                                                .await
                                                {
                                                    Ok(ok) => {
                                                        match tx.send(ok) {
                                                            Ok(_) => {}
                                                            Err(err) => {
                                                                println!("{} ln 554", err);
                                                            }
                                                        };
                                                    }
                                                    Err(err) => {
                                                        println!("ln 321 {:?}", err.source());
                                                    }
                                                };
                                            });
                                        }
                                        for file_path in self.files_to_send.clone() {
                                            match file_path
                                                .extension()
                                                .unwrap()
                                                .to_string_lossy()
                                                .as_str()
                                            {
                                                "png" | "jpeg" | "bmp" | "tiff" | "webp" => {
                                                    self.send_picture(file_path);
                                                }
                                                _ => {
                                                    self.send_file(file_path);
                                                }
                                            }
                                        }
                                        //clear vectors
                                        self.files_to_send.clear();
                                        self.replying_to = None;
                                        self.usr_msg.clear();
                                    }
                                },
                            );
                            ui.allocate_ui(
                                vec2(self.font_size * 1.5, self.font_size * 1.5),
                                |ui| {
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
                                },
                            );
                            ui.allocate_ui(
                                vec2(self.font_size * 1.5, self.font_size * 1.5),
                                |ui| {
                                    let button = ui.add(Button::new(
                                        RichText::from(&self.random_emoji)
                                            .size(self.font_size * 1.2),
                                    ));
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
                                },
                            );
                        });
                    });

                self.text_widget_offset = msg_tray.response.rect.width();

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
                        let file_serve: Result<ServerFileReply, serde_json::Error> =
                            serde_json::from_str(&msg);
                        let _ = write_file(file_serve.unwrap());
                    }
                    Err(_err) => {}
                }
                ui.allocate_space(vec2(ui.available_width(), 5.));

                match self.irx.try_recv() {
                    Ok(msg) => {
                        let file_serve: Result<ServerImageReply, serde_json::Error> =
                            serde_json::from_str(&msg);
                        let _ = write_image(file_serve.unwrap(), self.send_on_ip.clone());
                    }
                    Err(_err) => {}
                }
            });
        egui::TopBottomPanel::bottom("file_tray").show_animated(ctx, !self.files_to_send.is_empty(), |ui|{
            ui.allocate_space(vec2(ui.available_width(), 10.));
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
                    
                }  
                ui.allocate_space(vec2(ui.available_width(), 10.));   
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

        let message = ClientMessage::construct_file_msg(file, ip, passw, author, replying_to);

        tokio::spawn(async move {
            let _ = client::send_msg(message).await;
        });
    }

    fn send_picture(&mut self, file: std::path::PathBuf) {
        let passw = self.client_password.clone();
        let ip = self.send_on_ip.clone();
        let author = self.login_username.clone();
        let replying_to = self.replying_to.clone();

        dbg!(replying_to);

        let message = ClientMessage::construct_image_msg(file, ip, passw, author, replying_to);

        tokio::spawn(async move {
            let _ = client::send_msg(message).await;
        });
    }
}
