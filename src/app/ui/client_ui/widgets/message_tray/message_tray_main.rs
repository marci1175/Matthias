use crate::app::backend::{ClientMessage, TemplateApp};
use crate::app::ui::client_ui::client_actions::audio_recording::audio_recroding;
use chrono::Utc;
use egui::{
    vec2, Align, Align2, Area, Button, Color32, FontFamily, FontId, Key, Layout, Modifiers,
    RichText, Rounding, Stroke,
};
use rand::Rng;
use rfd::FileDialog;
use std::fs::{self};
use std::sync::mpsc;

impl TemplateApp {
    pub fn message_tray(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
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

        let text_widget = egui::TextEdit::multiline(&mut self.client_ui.usr_msg)
            .font(FontId {
                size: self.font_size,
                family: FontFamily::default(),
            })
            .hint_text(format!("Message to: {}", self.client_ui.send_on_ip))
            .desired_width(ui.available_width() - self.client_ui.text_widget_offset * 1.3)
            .desired_rows(0)
            .frame(false);

        let msg_scroll = egui::ScrollArea::vertical()
            .id_source("usr_input")
            .stick_to_bottom(true)
            .auto_shrink([false, true])
            .min_scrolled_height(self.font_size * 2.)
            .show(&mut frame_ui, |ui| ui.add(text_widget));

        self.client_ui.scroll_widget_rect = msg_scroll.inner_rect;

        ui.allocate_space(vec2(
            ui.available_width(),
            msg_scroll.inner.rect.height() + 15.,
        ));

        Area::new("msg_action_tray".into())
            .anchor(
                Align2::RIGHT_BOTTOM,
                vec2(-30., -msg_scroll.content_size.y / 2. - 4.),
            )
            .show(ctx, |ui| {
                //We should also pass in whether it should be enabled
                self.buttons(ui, ctx, self.client_connection.client.is_some());
            })
    }

    fn buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, enabled: bool) {
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            ui.allocate_ui(vec2(ui.available_width(), self.font_size * 1.5), |ui| {
                //Display buttons, check if they should be enabled or nah
                ui.add_enabled_ui(enabled, |ui| {
                    if ui
                        .add(egui::widgets::ImageButton::new(egui::include_image!(
                            "../../../../../../icons/send_msg.png"
                        )))
                        .clicked()
                        || ctx.input(|reader| reader.key_pressed(Key::Enter))
                            && !(ctx.input_mut(|reader| {
                                reader.consume_key(Modifiers::SHIFT, Key::Enter)
                            }))
                    {
                        if !(self.client_ui.usr_msg.trim().is_empty()
                            || self.client_ui.usr_msg.trim_end_matches('\n').is_empty())
                        {
                            self.send_msg(ClientMessage::construct_normal_msg(
                                &self.client_ui.usr_msg,
                                self.client_ui
                                    .req_passw
                                    .then_some((|| &self.client_ui.client_password)()),
                                &self.login_username,
                                self.client_ui.replying_to,
                            ))
                        }

                        for file_path in self.client_ui.files_to_send.clone() {
                            //Check for no user fuckery
                            if file_path.exists() {
                                self.send_msg(ClientMessage::construct_file_msg(
                                    file_path,
                                    self.client_ui
                                        .req_passw
                                        .then_some((|| &self.client_ui.client_password)()),
                                    &self.login_username,
                                    self.client_ui.replying_to,
                                ));
                            }
                        }

                        //clear vectors
                        self.client_ui.files_to_send.clear();
                        self.client_ui.replying_to = None;
                        self.client_ui.usr_msg.clear();
                    }

                    //add file button
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
                            self.client_ui.files_to_send.push(file);
                        }
                    }

                    //Emoji button
                    let button = ui.add(Button::new(
                        RichText::from(&self.client_ui.random_emoji).size(self.font_size * 1.2),
                    ));

                    if button.clicked() {
                        self.main.emoji_mode = !self.main.emoji_mode;
                    };
                    
                    if button.hovered() {
                        if !self.client_ui.random_generated {
                            let random_number = self
                                .client_ui
                                .rand_eng
                                .gen_range(0..=self.client_ui.emoji.len() - 1);
                            self.client_ui
                                .random_emoji
                                .clone_from(&self.client_ui.emoji[random_number]);
                            self.client_ui.random_generated = true;
                        }
                    } else {
                        //check if button has been unhovered, reset variable
                        self.client_ui.random_generated = false;
                    }

                    //Record button
                    if let Some(atx) = self.atx.clone() {
                        ui.horizontal_centered(|ui| {
                            ui.label(
                                RichText::from("Recording")
                                    .size(self.font_size)
                                    .color(Color32::RED),
                            );
                            //Display lenght
                            ui.label(
                                RichText::from(format!(
                                    "{}s",
                                    Utc::now()
                                        .signed_duration_since(
                                            self.client_ui.voice_recording_start.unwrap()
                                        )
                                        .num_seconds()
                                ))
                                .size(self.font_size),
                            );
                            if ui
                                .add(
                                    egui::ImageButton::new(egui::include_image!(
                                        "../../../../../../icons/record.png"
                                    ))
                                    .tint(Color32::RED),
                                )
                                .clicked()
                            {
                                //Just send something, it doesnt really matter
                                atx.send(false).unwrap();

                                //Path to voice recording created by audio_recording.rs, Arc mutex to avoid data races
                                match self.audio_file.clone().try_lock() {
                                    Ok(ok) => {
                                        self.send_msg(ClientMessage::construct_file_msg(
                                            ok.to_path_buf().clone(),
                                            self.client_ui
                                                .req_passw
                                                .then_some((|| &self.client_ui.client_password)()),
                                            &self.login_username,
                                            self.client_ui.replying_to,
                                        ));

                                        let _ = fs::remove_file(ok.to_path_buf());
                                    }
                                    Err(error) => println!("{error}"),
                                };

                                //Destroy state
                                self.atx = None;
                            }
                        });
                    } else if ui
                        .add(egui::ImageButton::new(egui::include_image!(
                            "../../../../../../icons/record.png"
                        )))
                        .clicked()
                    {
                        let (tx, rx) = mpsc::channel::<bool>();

                        self.atx = Some(tx);

                        //Set audio recording start
                        self.client_ui.voice_recording_start = Some(Utc::now());

                        audio_recroding(rx, self.audio_file.clone());
                    }
                });
            });
        });
    }
}
