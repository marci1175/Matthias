use device_query::Keycode;
use egui::{vec2, Align, Align2, Area, Button, FontFamily, FontId, Layout, RichText};

use rand::Rng;
use regex::Regex;
use rfd::FileDialog;
use std::fs::{self};
use std::sync::atomic::Ordering;
use std::time::Duration;
use windows_sys::w;
use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONEXCLAMATION, MB_ICONSTOP};

use std::sync::mpsc;

use crate::app::account_manager::write_file;
use crate::app::backend::TemplateApp;
use crate::app::client::{self, request_file, send_file};

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
            let passw = self.client_password.clone();
            let ip = self.send_on_ip.clone();

            tokio::spawn(async move {
                while should_be_running.load(Ordering::Relaxed) {
                    tokio::time::sleep(Duration::from_secs_f32(1.5)).await;
                    println!("requested sync!");
                    dbg!(ip.clone());
                    match client::sync_msg(passw.clone(), ip.clone()).await {
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
                self.incoming_msg = msg;
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
                        .show(ui, |ui| {
                            ui.separator();

                            ui.allocate_ui(ui.available_size(), |ui| {
                                for i in self.incoming_msg.clone().lines() {
                                    //md style
                                    if i.contains("file_upload") {
                                        //use a character before file upload which cannot be set as a file name
                                        let sort_msg: Vec<&str> = i.split('>').collect();

                                        let re = Regex::new(r#"'(.*?[^\\])'"#).unwrap();

                                        let mut results: Vec<String> = Vec::new();

                                        for captured in re.captures_iter(sort_msg[1]) {
                                            if let Some(inner_text) = captured.get(1) {
                                                results.push(inner_text.as_str().to_string());
                                            }
                                        }
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::from(sort_msg[0]).size(self.font_size),
                                            );
                                            if ui
                                                .button(
                                                    RichText::from(format!(
                                                        "Download {}",
                                                        results[0]
                                                    ))
                                                    .strong()
                                                    .size(self.font_size),
                                                )
                                                .clicked()
                                            {
                                                let ip = self.send_on_ip.clone();
                                                tokio::spawn(async move {
                                                    match request_file(
                                                        results[1].parse::<i32>().unwrap(),
                                                        ip,
                                                    )
                                                    .await
                                                    {
                                                        Ok(file_reponse) => {
                                                            if let Err(err) =
                                                                write_file(file_reponse)
                                                            {
                                                                println!("{err}")
                                                            };
                                                        }
                                                        Err(err) => println!("{err}"),
                                                    };
                                                });
                                            };
                                        });
                                    } else if (i.contains('[') && i.contains(']'))
                                        && (i.contains('(') && i.contains(')'))
                                    {
                                        let re = Regex::new(
                                            r"\[(?P<link_text>[^\]]*)\]\((?P<link_target>[^)]+)\)",
                                        )
                                        .unwrap();
                                        let mut captures: Vec<String> = Vec::new();
                                        for capture in re.captures_iter(i) {
                                            for i in 1..capture.len() {
                                                captures.push(capture[i].to_string());
                                            }
                                        }
                                        if captures.is_empty() {
                                            ui.label(RichText::from(i).size(self.font_size));
                                        } else {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::from(re.replace_all::<&str>(i, ""))
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
                                }
                            });

                            ui.separator();
                            if !self.usr_msg_expanded {
                                ui.allocate_space(vec2(ui.available_width(), 25.));
                            }
                        });
                },
            );
        });

        Area::new("usr_msg_expand")
            .anchor(
                Align2::RIGHT_BOTTOM,
                match self.usr_msg_expanded {
                    true => vec2(-41.0, -183.8),
                    false => vec2(-_frame.info().window_info.size[1] / 19.5, -10.),
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

        //usr_input
        egui::TopBottomPanel::bottom("usr_input").show_animated(ctx, self.usr_msg_expanded, |ui| {
                ui.allocate_space(vec2(ui.available_width(), 5.));

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
                                                    .hint_text(RichText::from(format!("Message : {}", self.send_on_ip)).size(self.font_size))
                                                    .font(FontId::new(self.font_size, FontFamily::default()))
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
                                .clicked() || input_keys.contains(&Keycode::Enter) && !(input_keys.contains(&Keycode::LShift)  || input_keys.contains(&Keycode::RShift)) && !(self.usr_msg.trim().is_empty() || self.usr_msg.trim_end_matches('\n').is_empty())
                            {
                                if self.usr_msg.contains("file_upload") {
                                    std::thread::spawn(|| unsafe {
                                        MessageBoxW(
                                            0,
                                            w!("You can not send server messages!"),
                                            w!("Error"),
                                            MB_ICONSTOP,
                                        );
                                    });
                                    return;
                                }

                                let temp_msg = self.usr_msg.clone();
                                let tx = self.tx.clone();
                                let username = self.login_username.clone();
                                //disable pass if its not ticked
                                let passw = match self.req_passw {
                                    true => self.client_password.clone(),
                                    false => "".into(),
                                };
                                let ok = self.send_on_ip.clone();
                                tokio::spawn(async move {
                                    match client::send_msg(username, temp_msg, passw, ok)
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
                                self.usr_msg.clear();
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
                                    match fs::read(file.clone()) {
                                        Ok(file_bytes) => {
                                            let passw = self.client_password.clone();
                                            let ip =  self.send_on_ip.clone();
                                            let author = self.login_username.clone();
                                            tokio::spawn(async move {
                                                match send_file(passw, ip, file_bytes, file, author).await {
                                                    Ok(_ok) => {
                                                        //server errors listing here

                                                        /*
                                                        
                                                        error -> 0 success
                                                        error -> 1 Server : failed to get APPDATA arg
                                                        error -> 2 Server : failed to create file
                                                        
                                                        */
                                                        match _ok {
                                                            -2 => {
                                                                std::thread::spawn(|| unsafe {
                                                                    MessageBoxW(
                                                                        0,
                                                                        w!("Invalid password"),
                                                                        w!("Error"),
                                                                        MB_ICONEXCLAMATION,
                                                                    );
                                                                });
                                                            }
                                                            -1 => {println!("File too large!")}
                                                            0 => {println!("File Sent successfully")}
                                                            1 => {println!("[Server : failed to get APPDATA arg] Error : {_ok}")}
                                                            2 => {println!("[Server : failed to create file] Error : {_ok}")}
                                                            _ => {println!("Unknown error : {_ok}")}
                                                        }
                                                    },
                                                    Err(err) => {
                                                        println!("{err}");
                                                    },
                                                };
                                            });
                                        },
                                        Err(err) => {
                                            println!("{err}")
                                        },
                                    }
                                }
                            }
                        });
                        ui.allocate_ui(vec2(37., 37.), |ui|{
                            let button = ui.add(
                                Button::new(RichText::from(&self.random_emoji).size(45.))
                            );

                            if button.clicked() {
                                self.emoji_mode = !self.emoji_mode;
                            };
                            
                            if button.hovered() {
                                if !self.random_generated {
                                    let random_number = self.rand_eng.gen_range(0..=self.emoji.len() - 1);
                                    self.random_emoji = self.emoji[random_number].clone();
                                    self.random_generated = true;
                                }
                            }
                            else {
                                //check if button has been unhovered, reset variable
                                self.random_generated = false;
                            }
                        });
                    });
                });
                //receive server answer unconditionally
                match self.rx.try_recv() {
                    Ok(ok) => self.incoming_msg = ok,
                    Err(_err) => {
                        //println!("ln 332 {}", err);
                    }
                };

                ui.allocate_space(vec2(ui.available_width(), 5.));
            });
    }
}
