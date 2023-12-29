use device_query::Keycode;
use egui::{
    vec2, Align, Align2, Area, Button, Color32, FontFamily, FontId, Key, Layout, RichText,
    Rounding, Stroke,
};
use rand::Rng;

use rfd::FileDialog;

use std::fs::{self};

use std::path::PathBuf;

use std::sync::mpsc;

//use crate::app::account_manager::write_file;
use crate::app::backend::{ClientMessage, TemplateApp};
use crate::app::client::{self};
use crate::app::ui::client_ui::client_actions::audio_recording::audio_recroding;

impl TemplateApp {
    pub fn message_tray(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        input_keys: Vec<Keycode>,
    ) -> egui::InnerResponse<()> {
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

        self.scroll_widget_rect = msg_scroll.inner_rect;

        ui.allocate_space(vec2(
            ui.available_width(),
            msg_scroll.inner.rect.height() + 15.,
        ));

        Area::new("msg_action_tray")
            .anchor(
                Align2::RIGHT_BOTTOM,
                vec2(-30., -msg_scroll.content_size.y / 2. - 4.),
            )
            .show(ctx, |ui| {
                self.buttons(ui, input_keys, ctx);
            })
    }

    fn buttons(&mut self, ui: &mut egui::Ui, input_keys: Vec<Keycode>, ctx: &egui::Context) {
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            ui.allocate_ui(vec2(self.font_size * 1.5, self.font_size * 1.5), |ui| {
                if ui
                    .add(egui::widgets::ImageButton::new(egui::include_image!(
                        "../../../../../../icons/send_msg.png"
                    )))
                    .clicked()
                    || ctx.input(|reader| reader.key_pressed(Key::Enter))
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
                        let replying_to = self.replying_to;
                        tokio::spawn(async move {
                            match client::send_msg(ClientMessage::construct_normal_msg(
                                &temp_msg,
                                temp_ip,
                                passw,
                                username,
                                replying_to,
                            ))
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
                        //Check for no user fuckery
                        if file_path.exists() {
                            self.send_file(file_path);
                        }
                    }

                    //clear temp files
                    let _ = fs::remove_file(concat!(
                        env!("APPDATA"),
                        "/szeChat/Client/voice_record.wav"
                    ));

                    //clear vectors
                    self.files_to_send.clear();
                    self.replying_to = None;
                    self.usr_msg.clear();
                }
            });
            ui.allocate_ui(vec2(self.font_size * 1.5, self.font_size * 1.5), |ui| {
                if ui
                    .add(egui::widgets::ImageButton::new(egui::include_image!(
                        "../../../../../../icons/add_file.png"
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
            ui.allocate_ui(vec2(self.font_size * 1.5, self.font_size * 1.5), |ui| {
                let button = ui.add(Button::new(
                    RichText::from(&self.random_emoji).size(self.font_size * 1.2),
                ));
                if button.clicked() {
                    self.emoji_mode = !self.emoji_mode;
                };
                if button.hovered() {
                    if !self.random_generated {
                        let random_number = self.rand_eng.gen_range(0..=self.emoji.len() - 1);
                        self.random_emoji = self.emoji[random_number].clone();
                        self.random_generated = true;
                    }
                } else {
                    //check if button has been unhovered, reset variable
                    self.random_generated = false;
                }
            });
            ui.allocate_ui(vec2(self.font_size * 1.5, self.font_size * 1.5), |ui| {
                if let Some(atx) = self.atx.clone() {
                    if ui
                        .add(
                            egui::ImageButton::new(egui::include_image!(
                                "../../../../../../icons/record.png"
                            ))
                            .tint(Color32::RED),
                        )
                        .clicked()
                    {
                        ui.label(RichText::from("Recording").size(self.font_size / 2.));
                        //Just send something, it doesnt really matter
                        atx.send(false).unwrap();

                        //Path to voice recording created by audio_recording.rs
                        let path = PathBuf::from(format!(
                            "{}\\szeChat\\Client\\voice_record.wav",
                            env!("APPDATA")
                        ));

                        if path.exists() {
                            self.files_to_send.push(path);
                        }

                        //Destroy state
                        self.atx = None;
                    }
                } else if ui
                    .add(egui::ImageButton::new(egui::include_image!(
                        "../../../../../../icons/record.png"
                    )))
                    .clicked()
                {
                    let (tx, rx) = mpsc::channel::<bool>();

                    self.atx = Some(tx);

                    audio_recroding(rx);
                }
            });
        });
    }
}
