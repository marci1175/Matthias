use crate::app::backend::{
    Application, ClientMessage, ConnectionState, MessagingMode, ServerMessageType, EMOJI_TUPLES,
};
use crate::app::ui::client_ui::client_actions::audio_recording::{
    audio_recording_with_recv, create_wav_file, record_audio_for_set_duration,
};
use chrono::Utc;
use egui::load::{BytesPoll, LoadError};
use egui::text::{CCursor, CCursorRange};
use egui::{
    vec2, Align, Align2, Area, Color32, FontFamily, FontId, Image, Key, KeyboardShortcut, Layout,
    Modifiers, RichText, Rounding, ScrollArea, Stroke,
};
use hound::WavWriter;
use rand::Rng;
use rfd::FileDialog;
use std::fs::{self};
use std::io::{BufRead, BufReader, Cursor, Write};
use std::sync::{mpsc, Mutex};

impl Application {
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

        let mut frame_ui = ui.child_ui(code_rect, Layout::default(), None);

        self.display_user_recommendation(ctx);

        self.display_emoji_recommendation(ctx);

        //If the key was not consumed by any of the two previous functions, we will edit the latest message sent by us
        //We wont allow this when we are either editing a message or we are replying to one
        if !(matches!(self.client_ui.messaging_mode, MessagingMode::Edit(_))
            || matches!(self.client_ui.messaging_mode, MessagingMode::Reply(_)))
            && self.client_ui.message_buffer.is_empty()
        {
            ctx.input_mut(|reader| {
                //Check if this key was pressed
                //We will not consume this key since its not sure we can acually edit the message
                if reader.key_pressed(Key::ArrowUp) {
                    //Iter over all the messages so we will get the latest message sent by us
                    for (idx, message) in self
                        .client_ui
                        .incoming_messages
                        .message_list
                        .iter()
                        .enumerate()
                    {
                        //Validate editable message
                        if let ServerMessageType::Normal(inner) = &message.message_type {
                            if message.uuid == self.opened_user_information.uuid
                                && message.message_type != ServerMessageType::Deleted
                            {
                                //If we can edit said message we can safely consume the key
                                reader.consume_key(Modifiers::NONE, Key::ArrowUp);

                                self.client_ui.messaging_mode = MessagingMode::Edit(idx);
                                self.client_ui.message_buffer = inner.message.to_string();
                            }
                        }
                    }
                }
            });
        }

        //Create widget
        let text_widget = egui::TextEdit::multiline(&mut self.client_ui.message_buffer)
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

                //IF the user has modified this value then we should apply it to the text editor
                if let Some(cursor_idx) = self.client_ui.text_edit_cursor_desired_index {
                    text_widget
                        .state
                        .cursor
                        .set_char_range(Some(CCursorRange::one(CCursor::new(cursor_idx))));

                    //Store state
                    text_widget.state.store(ctx, text_widget.response.id);

                    //Reset desired index
                    self.client_ui.text_edit_cursor_desired_index = None;
                }

                //We should reset the value to its updated value
                if let Some(cursor_rng) = text_widget.cursor_range {
                    self.client_ui.text_edit_cursor_index =
                        cursor_rng.as_ccursor_range().sorted()[0].index;
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
                vec2(-30., -msg_scroll.inner_rect.size().y / 2. + 4.),
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

    fn display_user_recommendation(&mut self, ctx: &egui::Context) {
        /*We have to clone here because of the closure*/
        let user_message_clone = self.client_ui.message_buffer.clone();

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
                        let user_message_clone = self.client_ui.message_buffer.clone();

                        //We will reconstruct the buffer
                        let mut split = user_message_clone.split('@').collect::<Vec<_>>();

                        if let Some(buffer) = split.last_mut() {
                            //If we have already typed in the full username OR there are no username matches in what we typed in we can return, so we wont consume the enter key therefor were going to send the message
                            for seen in &self.client_ui.incoming_messages.user_seen_list {
                                let profile = self
                                    .client_ui
                                    .incoming_messages
                                    .connected_clients_profile
                                    .get(&seen.uuid);
                                if let Some(profile) = profile {
                                    let username = profile.username.clone();

                                    if !(username.contains(*buffer) && *buffer != username) {
                                        return;
                                    }
                                }
                            }

                            //If the ENTER key is pressed append the name to the self.client_ui.text_edit_buffer
                            if reader.consume_key(Modifiers::NONE, Key::Enter)
                                && !self.client_ui.incoming_messages.user_seen_list.is_empty()
                            {
                                //format the string so the @ stays
                                if let Some(profile) = self
                                    .client_ui
                                    .incoming_messages
                                    .connected_clients_profile
                                    .get(
                                        &self.client_ui.incoming_messages.user_seen_list
                                            [self.client_ui.user_selector_index as usize]
                                            .uuid,
                                    )
                                {
                                    let formatted_string = profile.username.clone();

                                    *buffer = &formatted_string;

                                    //Concat the vector after modifying it, we know that every piece of string is split by a '@' so we can join them all by one, therefor avoiding deleting previous @s cuz theyre not present when concating a normal vec (constructed from a string, split by @s)
                                    let split_concat = split.join("@");

                                    //Set the buffer to the concatenated vector, append the @ to the 0th index
                                    self.client_ui.message_buffer = split_concat;
                                }
                            };
                        }
                    });
                }
            }
        }
    }

    fn display_emoji_recommendation(&mut self, ctx: &egui::Context) {
        /*We have to clone here because of the closure*/
        let user_message_clone = self.client_ui.message_buffer.clone();

        //We will reconstruct the buffer
        let split = user_message_clone
            .split(':')
            .map(|item| item.to_string())
            .collect::<Vec<String>>();

        let split_clone = split.clone();

        //We just pattern match for the sake of never panicing, if we called .unwrap() on this it would still (im 99% sure) work, and its still nicer than (...).get(.len() - 1)
        if let Some(last) = split.last() {
            //If the last slice of the string (split by :) doesnt contain any spaces we can paint everything else
            if !last.contains(' ') && !last.is_empty() && split_clone.len() > 1 {
                let matched_emojis = self.get_emojis(ctx, last.to_string());

                //If there are no emoji matches we wont allow the user to send the mssage
                //I might remove this later
                if matched_emojis.is_empty() {
                    return;
                }

                //Consume input when we are diplaying the user list
                ctx.input_mut(|reader| {
                    //Clone var to avoid error
                    let user_message_clone = self.client_ui.message_buffer.clone();

                    //If we have already typed in the emoji OR there are no emoji matches in what we typed in we can return, so we wont consume the enter key therefor were going to send the message
                    //If the ENTER key is pressed append the name to the self.client_ui.text_edit_buffer
                    if reader.consume_key(Modifiers::NONE, Key::Enter) {
                        //We will reconstruct the original list
                        let mut split = user_message_clone.split(':').collect::<Vec<_>>();
                        if let Some(last) = split.last_mut() {
                            let mut formatted_string = matched_emojis
                                [self.client_ui.emoji_selector_index as usize]
                                .clone();

                            //Make sure we close the emoji
                            formatted_string.push(':');

                            *last = &formatted_string;

                            //Concat the vector after modifying it, we know that every piece of string is split by a '@' so we can join them all by one, therefor avoiding deleting previous @s cuz theyre not present when concating a normal vec (constructed from a string, split by @s)
                            let split_concat = split.join(":");

                            //Set the buffer to the concatenated vector, append the @ to the 0th index
                            self.client_ui.message_buffer = split_concat;

                            self.client_ui.text_edit_cursor_desired_index =
                                Some(self.client_ui.message_buffer.len());
                        }
                    };
                });
            }
        }
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
                        if !(self.client_ui.message_buffer.trim().is_empty()
                            || self
                                .client_ui
                                .message_buffer
                                .trim_end_matches('\n')
                                .is_empty())
                        {
                            match self.client_ui.messaging_mode {
                                MessagingMode::Edit(index) => {
                                    self.send_msg(ClientMessage::construct_client_message_edit(
                                        index,
                                        Some(self.client_ui.message_buffer.clone()),
                                        &self.opened_user_information.uuid,
                                    ))
                                }
                                //If its reply or normal mode we can just send the message and call get_reply_index on it
                                _ => self.send_msg(ClientMessage::construct_normal_msg(
                                    &self.client_ui.message_buffer,
                                    &self.opened_user_information.uuid,
                                    self.client_ui.messaging_mode.get_reply_index(),
                                )),
                            }
                        }

                        for file_path in &self.client_ui.files_to_send {
                            //Check for no user fuckery
                            if file_path.exists() {
                                self.send_msg(ClientMessage::construct_file_msg(
                                    file_path.clone(),
                                    &self.opened_user_information.uuid,
                                    self.client_ui.messaging_mode.get_reply_index(),
                                ));
                            }
                        }

                        //clear vectors
                        self.client_ui.files_to_send.clear();
                        self.client_ui.messaging_mode = MessagingMode::Normal;
                        self.client_ui.message_buffer.clear();
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
                    let emoji_button = ui.menu_button(
                        RichText::from(self.client_ui.random_emoji.clone())
                            .size(self.font_size * 1.2),
                        |ui| {
                            //If selected_emoji isnt a Some(_) then the user didnt click anything
                            if let Some(emoji_name) = self.draw_emoji_selector(ui, ctx) {
                                let is_inserting_front = self.client_ui.text_edit_cursor_index
                                    == self.client_ui.message_buffer.len();

                                self.client_ui.message_buffer.insert_str(
                                    self.client_ui.text_edit_cursor_index,
                                    &format!(":{}:", emoji_name),
                                );

                                if is_inserting_front {
                                    self.client_ui.text_edit_cursor_index =
                                        self.client_ui.message_buffer.len();
                                }

                                ui.close_menu();
                            }
                        },
                    );

                    if emoji_button.response.clicked() {
                        self.main.emoji_mode = !self.main.emoji_mode;
                    };

                    if emoji_button.response.hovered() {
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
                            let stop_recording_button = ui
                                .allocate_ui(
                                    vec2(ui.available_width(), self.font_size * 1.5),
                                    |ui| {
                                        ui.add(
                                            egui::ImageButton::new(egui::include_image!(
                                                "../../../../../../icons/record.png"
                                            ))
                                            .tint(Color32::RED),
                                        )
                                    },
                                )
                                .inner;

                            //Signal the recording thread to stop
                            if stop_recording_button.clicked() {
                                //Just send something, it doesnt really matter
                                atx.send(false).unwrap();

                                //Destroy state
                                self.atx = None;
                            }

                            //Display stats
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

                        //Move into thread
                        let audio_bytes_sender = self.audio_bytes_tx.clone();

                        tokio::spawn(async move {
                            let bytes = audio_recording_with_recv(rx).unwrap();

                            //These bytes can be played back with rodio (Wav format)
                            let playback_bytes = create_wav_file(bytes);

                            audio_bytes_sender.send(playback_bytes).unwrap();
                        });
                    }
                });
            });
        });
    }

    fn get_connected_users(&mut self, ctx: &egui::Context) -> bool {
        let split_user_msg = self
            .client_ui
            .message_buffer
            .split(['@'])
            .collect::<Vec<_>>();

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
                        if self.client_ui.incoming_messages.user_seen_list.is_empty() {
                            //Display greeting message
                            ui.label(RichText::from("Syncing. . .").color(Color32::RED));
                        }

                        for (index, client) in self
                            .client_ui
                            .incoming_messages
                            .user_seen_list
                            .iter()
                            .enumerate()
                        {
                            //If the search buffer is contained in the clients' username
                            if let Some(profile) = self
                                .client_ui
                                .incoming_messages
                                .connected_clients_profile
                                .get(&client.uuid)
                            {
                                let username = profile.username.clone();

                                if username.contains(last_str) {
                                    if index == self.client_ui.user_selector_index as usize {
                                        ui.group(|ui| {
                                            ui.label(
                                                RichText::from(&username).color(Color32::YELLOW),
                                            );
                                        });
                                    } else {
                                        ui.label(RichText::from(&username));
                                    }
                                }
                            }
                            //If the profile was not found then we can ask for it
                            else {
                                ui.spinner();
                            }
                        }
                    }
                });

                //Save rect taken up by area
                self.client_ui.connected_users_display_rect = Some(message_group.response.rect);

                //If the seen list is empty we should display a message indicating its loading but we should return before clamping because it would go -1 therefor we would be panicking
                if self.client_ui.incoming_messages.user_seen_list.is_empty() {
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
                        self.client_ui.incoming_messages.user_seen_list.len() as i32 - 1,
                    );
            });

        //request repaint so we'll show the latest info
        ctx.request_repaint();

        true
    }

    /// This function draws all the recommended emojis, and returns the list of the emojis' name which contain the ```emoji_name``` arg.
    fn get_emojis(&mut self, ctx: &egui::Context, emoji_name: String) -> Vec<String> {
        let matched_emojis: Vec<String> = EMOJI_TUPLES
            .keys()
            .filter(|key| key.contains(&emoji_name))
            .map(|s| s.to_string())
            .collect();

        Area::new("Emojis".into())
            .enabled(true)
            .anchor(Align2::LEFT_BOTTOM, vec2(50., -100.))
            .show(ctx, |ui| {
                //Draw background, before drawing the area, so it wont overlap
                if let Some(rect) = self.client_ui.emojis_display_rect {
                    ui.painter().rect_filled(rect, 5., Color32::BLACK);
                }

                ui.allocate_ui(vec2(100., 350.), |ui| {
                    let emoji_group = ui.group(|ui| {
                        //Display main title
                        ui.label("Matching emojis:");

                        ui.separator();

                        //Create scroll area
                        ScrollArea::vertical().show(ui, |ui| {
                            //Iter over matching emoji names
                            for (index, emoji_name) in matched_emojis.iter().enumerate() {
                                //Allocate ui for one entry in the emoji
                                ui.allocate_ui(vec2(25., 25.), |ui| {
                                    //Emoji entry group
                                    let emoji_group = ui.group(|ui| {
                                        //Create entry
                                        ui.horizontal_centered(|ui| {
                                            //Display name of the emoji, if the selector index and the iter index matches we draw the name with yellow
                                            if self.client_ui.emoji_selector_index as usize == index {
                                                ui.label(RichText::from(emoji_name).color(Color32::YELLOW));
                                            }
                                            else {
                                                ui.label(RichText::from(emoji_name));
                                            }

                                            //Display the emoji itself
                                            ui.allocate_ui(vec2(30., 30.), |ui| {
                                                //Try to load the emoji bytes
                                                match ctx.try_load_bytes(&format!("bytes://{}", emoji_name)) {
                                                    Ok(bytespoll) => {
                                                        if let BytesPoll::Ready { size:_, bytes, mime:_ } = bytespoll {
                                                            if bytes.to_vec() == vec![0] {
                                                                ui.spinner();
                                                                ui.label(RichText::from("The called emoji was not found in the emoji header").color(Color32::RED));
                                                                eprintln!("The called emoji was not found in the emoji header: {}", emoji_name);
                                                            }
                                                            ui.add(Image::from_uri(&format!("bytes://{}", emoji_name)));
                                                        }
                                                    },
                                                    Err(err) => {
                                                        if let LoadError::Loading(inner) = err {
                                                            if inner == "Bytes not found. Did you forget to call Context::include_bytes?" {
                                                                //check if we are visible, so there are no unnecessary requests
                                                                if !ui.is_rect_visible(ui.min_rect()) {
                                                                    return;
                                                                }
                                                                ctx.include_bytes(format!("bytes://{}", &emoji_name), EMOJI_TUPLES.get(emoji_name).map_or_else(|| vec![0], |v| v.to_vec()));
                                                            } else {
                                                                dbg!(inner);
                                                            }
                                                        } else {
                                                            dbg!(err);
                                                        }
                                                    },
                                                }
                                            });
                                        });
                                    });

                                    //If the user uses the arrows to navigate in the emoji entries we should make the current highlighted emoji always visible
                                    if index == self.client_ui.emoji_selector_index as usize {
                                        emoji_group.response.scroll_to_me(None);
                                    }
                                });
                            }
                        });
                    });

                self.client_ui.emojis_display_rect = Some(emoji_group.response.rect);

                //if there are no matched emojis we return to, avoid panicing cuz of the clamping, and to avoid consuming inputs
                if matched_emojis.is_empty() {
                    return;
                }

                //Log keyboard actions
                ctx.input_mut(|reader| {
                    if reader.consume_key(Modifiers::NONE, Key::ArrowUp) {
                        self.client_ui.emoji_selector_index -= 1;
                    };

                    if reader.consume_key(Modifiers::NONE, Key::ArrowDown) {
                        self.client_ui.emoji_selector_index += 1;
                    };
                });

                //Clamp to ensure usage safety, we take away 1 for obvious vector indexing reasons
                self.client_ui.emoji_selector_index = self
                    .client_ui
                    .emoji_selector_index
                    //*Make sure we return if ```self.client_ui.incoming_msg.user_seen_list``` is empty because then it'd overflow
                    .clamp(
                        0,
                        matched_emojis.len() as i32 - 1,
                    );
                });


                //We repaint to make the background repaint instantly
                ctx.request_repaint();
            });

        matched_emojis
    }
}
