use crate::app::backend::{ClientMessage, ConnectionState, TemplateApp};
use crate::app::ui::client_ui::client_actions::audio_recording::audio_recroding;
use chrono::Utc;
use egui::epaint::text::cursor::Cursor;
use egui::text::{CCursor, CursorRange};
use egui::{
    vec2, Align, Align2, Area, Button, Color32, FontFamily, FontId, Key, KeyboardShortcut, Layout,
    Modifiers, RichText, Rounding, Stroke,
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

        /*We have to clone here because of the closure*/
        let user_message_clone = self.client_ui.usr_msg.clone();

        //We will reconstruct the buffer
        let mut split = user_message_clone.split('@').collect::<Vec<_>>();

        //We just pattern match for the sake of never panicing, if we called .unwrap() on this it would still (im 99% sure) work, and its still nicer than (...).get(.len() - 1)
        if let Some(last) = split.last_mut() {
            //If the last slice of the string (split by @) doesnt contain any spaces we can paint everything else
            if !last.contains(' ') {
                //Set this var true if the @ menu is being displayed;
                //* self.get_connected_users function MUST be called before showing the text input widget, so this way we can actually consume the ArrowUp and Down keys
                self.client_ui.display_user_list = self.get_connected_users(ctx);

                //Consume input when we are diplaying the user list
                if self.client_ui.display_user_list {
                    ctx.input_mut(|reader| {
                        //Clone var to avoid error
                        let user_message_clone = self.client_ui.usr_msg.clone();

                        //We will reconstruct the buffer
                        let mut split = user_message_clone.split('@').collect::<Vec<_>>();

                        if let Some(buffer) = split.last_mut() {
                            //If we have already typed in the full username OR there are no username matches in what we typed in we can return, so we wont consume the enter key therefor were going to send the message
                            for seen in &self.client_ui.incoming_msg.user_seen_list {
                                if !(seen.username.contains(*buffer) && *buffer != seen.username) {
                                    return;
                                }
                            }

                            //If the ENTER key is pressed append the name to the self.client_ui.text_edit_buffer
                            if reader.consume_key(Modifiers::NONE, Key::Enter)
                                && !self.client_ui.incoming_msg.user_seen_list.is_empty()
                            {
                                //format the string so the @ stays
                                let formatted_string = self.client_ui.incoming_msg.user_seen_list
                                    [self.client_ui.user_selector_index as usize]
                                    .username
                                    .to_string();

                                *buffer = &formatted_string;

                                //Concat the vector after modifying it, we know that every piece of string is split by a '@' so we can join them all by one, therefor avoiding deleting previous @s cuz theyre not present when concating a normal vec (constructed from a string, split by @s)
                                let split_concat = split.join("@");

                                //Set the buffer to the concatenated vector, append the @ to the 0th index
                                self.client_ui.usr_msg = split_concat;
                            };
                        }
                    });
                }
            }
        }

        //Create widget
        let text_widget = egui::TextEdit::multiline(&mut self.client_ui.usr_msg)
            .font(FontId {
                size: self.font_size,
                family: FontFamily::default(),
            })
            .hint_text(format!("Message to: {}", self.client_ui.send_on_ip))
            .desired_width(ui.available_width() - self.client_ui.text_widget_offset * 1.3)
            .desired_rows(0)
            .return_key(KeyboardShortcut::new(Modifiers::SHIFT, Key::Enter))
            .frame(false);

        //Create scroll area
        let msg_scroll = egui::ScrollArea::vertical()
            .id_source("usr_input")
            .stick_to_bottom(true)
            .auto_shrink([false, true])
            .min_scrolled_height(self.font_size * 2.)
            .show(&mut frame_ui, |ui| {
                let mut text_widget = text_widget.show(ui);

                if let Some(cursor) = self.client_ui.text_edit_cursor {
                    text_widget.cursor_range = Some(CursorRange::one(Cursor {
                        ccursor: CCursor::new(cursor),
                        ..Default::default()
                    }));
                }

                text_widget.response
            });

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
                self.buttons(
                    ui,
                    ctx,
                    matches!(self.client_connection.state, ConnectionState::Connected(_)),
                );
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
                                &self.opened_account.uuid,
                                &self.login_username,
                                self.client_ui.replying_to,
                            ))
                        }

                        for file_path in &self.client_ui.files_to_send {
                            //Check for no user fuckery
                            if file_path.exists() {
                                self.send_msg(ClientMessage::construct_file_msg(
                                    file_path.clone(),
                                    &self.opened_account.uuid,
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
                                            &self.opened_account.uuid,
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

    fn get_connected_users(&mut self, ctx: &egui::Context) -> bool {
        let split_user_msg = self.client_ui.usr_msg.split(['@']).collect::<Vec<_>>();

        //If the user didnt type @ || if the seen list is empty
        if split_user_msg.len() - 1 == 0 {
            return false;
        }

        Area::new("Users".into())
            .enabled(true)
            .anchor(Align2::LEFT_BOTTOM, vec2(50., -100.))
            .show(ctx, |ui| {
                //Draw background, before actually allocating the area
                if let Some(rect) = self.client_ui.connected_users_display_rect {
                    ui.painter().rect_filled(rect, 5., Color32::BLACK);
                }

                //First we display it, because then we can return from the logging if seen_list is empty
                let message_group = ui.group(|ui| {
                    ui.label(RichText::from("Users:").strong());
                    if let Some(last_str) = split_user_msg.last() {
                        if self.client_ui.incoming_msg.user_seen_list.is_empty() {
                            //Display greeting message
                            ui.label(RichText::from("Syncing. . .").color(Color32::RED));
                        }

                        for (index, client) in self
                            .client_ui
                            .incoming_msg
                            .user_seen_list
                            .iter()
                            .enumerate()
                        {
                            //If the search buffer is contained in the clients' username
                            if client.username.contains(last_str) {
                                if index == self.client_ui.user_selector_index as usize {
                                    ui.group(|ui| {
                                        ui.label(
                                            RichText::from(&client.username).color(Color32::YELLOW),
                                        );
                                    });
                                } else {
                                    ui.label(RichText::from(&client.username));
                                }
                            }
                        }
                    }
                });

                //Save rect taken up by area
                self.client_ui.connected_users_display_rect = Some(message_group.response.rect);

                //If the seen list is empty we should display a message indicating its loading but we should return before clamping because it would go -1 therefor we would be panicking
                if self.client_ui.incoming_msg.user_seen_list.is_empty() {
                    return;
                }

                //Log keyboard actions
                ctx.input_mut(|reader| {
                    if reader.consume_key(Modifiers::NONE, Key::ArrowUp) {
                        self.client_ui.user_selector_index -= 1;
                    };

                    if reader.consume_key(Modifiers::NONE, Key::ArrowDown) {
                        self.client_ui.user_selector_index += 1;
                    };
                });

                //Clamp to ensure usage safety, we take away 1 for obvious vector indexing reasons
                self.client_ui.user_selector_index = self
                    .client_ui
                    .user_selector_index
                    //*Make sure we return if ```self.client_ui.incoming_msg.user_seen_list``` is empty because then it'd overflow
                    .clamp(
                        0,
                        self.client_ui.incoming_msg.user_seen_list.len() as i32 - 1,
                    );
            });

        //request repaint so we'll show the latest info
        ctx.request_repaint();

        true
    }
}
