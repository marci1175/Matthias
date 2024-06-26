use egui::{
    vec2, Align, Align2, Area, Color32, FontFamily, FontId, Id, Layout, Pos2, RichText, Sense,
    Stroke,
};
use rodio::{Decoder, Sink};
use tokio_util::sync::CancellationToken;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio::select;

use crate::app::backend::{
    decrypt_aes256, display_error_message, write_audio, write_file, write_image, ClientMessage,
    ClientMessageType, ConnectionState, MessageReaction, PlaybackCursor, Reaction, ServerReplyType,
    ServerSync,
};

use crate::app::backend::{SearchType, ServerImageReply, ServerMessageType, TemplateApp};
use crate::app::client::ServerReply;

impl TemplateApp {
    pub fn state_client(&mut self, _frame: &mut eframe::Frame, ctx: &egui::Context) {
        egui::TopBottomPanel::new(egui::panel::TopBottomSide::Top, "settings_area").show(
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
                            self.main.client_mode = false;

                            if self.server_has_started {
                                display_error_message("Can not log out while server is running.");
                                return;
                            }

                            self.autosync_shutdown_token.cancel();
                            self.server_sender_thread = None;

                            self.main.client_mode = false;
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
                    ui.allocate_ui(vec2(300., 40.), |ui| {
                        if ui
                            .add(egui::widgets::ImageButton::new(egui::include_image!(
                                "../../../icons/search.png"
                            )))
                            .clicked()
                        {
                            self.client_ui.search_mode = !self.client_ui.search_mode;
                        };
                    });
                    ui.allocate_ui(vec2(300., 50.), |ui| {
                        ui.label(RichText::from("Welcome,").weak().size(20.));
                        ui.label(
                            RichText::from(self.opened_user_information.username.to_string())
                                .strong()
                                .size(20.),
                        );
                    });
                });

                ui.allocate_space(vec2(ui.available_width(), 5.));
            },
        );

        //Message input panel
        let usr_panel = egui::TopBottomPanel::bottom("usr_input")
            .max_height(ctx.used_size().y / 2.)
            .show_animated(ctx, self.client_ui.usr_msg_expanded, |ui| {
                ui.add_enabled_ui(
                    matches!(self.client_connection.state, ConnectionState::Connected(_)),
                    |ui| {
                        let msg_tray = self.message_tray(ui, ctx);

                        self.client_ui.text_widget_offset = msg_tray.response.rect.width();

                        ui.allocate_space(vec2(ui.available_width(), 5.));
                    },
                );
            });

        //We have to render the message area after everything else, because then we will be using the area whats left of the ui
        //msg_area
        egui::CentralPanel::default().show(ctx, |ui| {
            //Drop file warning
            self.client_ui.drop_file_animation =
                ui.input(|input| !input.raw.clone().hovered_files.is_empty());

            if self.client_ui.animation_state >= 0. {
                //Get window size
                let window_size = ui.input(|reader| reader.screen_rect().max).to_vec2();

                //Define default font
                let font_id = FontId {
                    family: FontFamily::default(),
                    size: self.font_size,
                };

                //Draw background fading animation
                ui.painter().rect_filled(
                    egui::Rect::EVERYTHING,
                    0.,
                    Color32::from_rgba_premultiplied(
                        0,
                        0,
                        0,
                        (self.client_ui.animation_state / 3.) as u8,
                    ),
                );

                //Draw rectangle in the middle where the text also appears
                Area::new("warning_overlay".into()).show(ctx, |ui| {
                    ui.painter().rect(
                        egui::Rect {
                            min: Pos2::new(
                                window_size[0] / 3.,
                                window_size[0] / 5. + self.client_ui.animation_state / 50.,
                            ),
                            max: Pos2::new(
                                window_size[0] / 1.5,
                                window_size[0] / 3. + self.client_ui.animation_state / 50.,
                            ),
                        },
                        5.0,
                        Color32::from_rgba_unmultiplied(
                            0,
                            0,
                            0,
                            self.client_ui.animation_state as u8 / 8,
                        ),
                        Stroke::default(),
                    );
                    ui.painter().text(
                        Pos2::new(
                            window_size[0] / 2.,
                            window_size[0] / 4. + self.client_ui.animation_state / 50.,
                        ),
                        Align2([Align::Center, Align::Center]),
                        "Drop to upload",
                        font_id,
                        Color32::from_rgba_unmultiplied(
                            255,
                            255,
                            255,
                            self.client_ui.animation_state as u8,
                        ),
                    );
                });
            }

            //Animate self.client_ui.animation_state by incrementing it with 255. / 0.4 per sec
            self.client_ui.animation_state = ctx.animate_value_with_time(
                Id::from("warning_overlay"),
                match self.client_ui.drop_file_animation {
                    true => 255.,
                    false => 0.,
                },
                0.4,
            );

            let dropped_files = ui.input(|reader| reader.raw.clone().dropped_files);
            if !dropped_files.is_empty() {
                let dropped_file_path = dropped_files[0].path.clone().unwrap_or_default();

                self.client_ui.files_to_send.push(dropped_file_path);
            }

            //Messages go here, check if there is a connection
            ui.add_enabled_ui(
                matches!(self.client_connection.state, ConnectionState::Connected(_)),
                |ui| {
                    self.client_ui_message_main(ui, ctx);
                },
            );
        });

        //search area
        if self.client_ui.search_mode {
            egui::SidePanel::right("search_panel").exact_width(ctx.used_size().x / 3.5).show(ctx, |ui|{
                ui.separator();
                ui.horizontal(|ui|{

                    //Dont allow displaying search buffer when in file or reply searching
                    if !(self.client_ui.search_parameter == SearchType::File || self.client_ui.search_parameter == SearchType::Reply) {
                        ui.allocate_ui(vec2(ui.available_width() / 2., ui.available_height()), |ui| {
                            ui.add(
                                egui::widgets::TextEdit::singleline(&mut self.client_ui.search_buffer).hint_text("Search for: ")
                            );
                        });
                    }

                    egui::ComboBox::from_id_source("search_filter")
                            // .icon(|ui, rect, widget_visuals, is_open, above_or_belov| {})
                            .selected_text(format!("{}", self.client_ui.search_parameter.clone()))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.client_ui.search_parameter, SearchType::Message , "Message");
                                ui.selectable_value(&mut self.client_ui.search_parameter, SearchType::Date, "Date");
                                ui.selectable_value(&mut self.client_ui.search_parameter, SearchType::Name, "Name");
                                ui.selectable_value(&mut self.client_ui.search_parameter, SearchType::Reply, "Reply");
                                ui.selectable_value(&mut self.client_ui.search_parameter, SearchType::File, "File");
                            });
                });
                ui.separator();

                //For the has_search logic to work and for the rust compiler not to underline everything
                egui::ScrollArea::new([true, true]).auto_shrink([false, true]).show(ui, |ui|{
                    ui.allocate_ui(ui.available_size(), |ui|{
                        let mut has_search = false;
                        for (index, message) in self.client_ui.incoming_msg.struct_list.iter().enumerate() {
                            match self.client_ui.search_parameter {
                                SearchType::Name => {
                                    if let ServerMessageType::Normal(inner_message) = &message.message_type {
                                        if message.author.contains(self.client_ui.search_buffer.trim()) && !self.client_ui.search_buffer.trim().is_empty() {
                                            let group = ui.group(|ui|{
                                                ui.label(RichText::from(message.author.to_string()).size(self.font_size / 1.3).color(Color32::WHITE));
                                                ui.label(RichText::from(inner_message.message.to_string()));
                                                ui.small(&message.message_date);
                                            });

                                            if group.response.interact(Sense::click()).clicked() {
                                                self.client_ui.scroll_to_message_index = Some(index)
                                            };

                                            group.response.on_hover_text("Click to jump to message");

                                            has_search = true;
                                        }
                                    }
                                },
                                SearchType::Message => {
                                    if let ServerMessageType::Normal(inner_message) = &message.message_type {
                                        if inner_message.message.contains(self.client_ui.search_buffer.trim()) && !self.client_ui.search_buffer.trim().is_empty() {
                                            let group = ui.group(|ui|{
                                                ui.label(RichText::from(message.author.to_string()).size(self.font_size / 1.3).color(Color32::WHITE));
                                                ui.label(RichText::from(inner_message.message.to_string()));
                                                ui.small(&message.message_date);
                                            });

                                            if group.response.interact(Sense::click()).clicked() {
                                                self.client_ui.scroll_to_message_index = Some(index)
                                            };

                                            group.response.on_hover_text("Click to jump to message");

                                            has_search = true;
                                        }
                                    }
                                },
                                SearchType::Date => {
                                    if let ServerMessageType::Normal(inner_message) = &message.message_type {
                                        if message.message_date.contains(self.client_ui.search_buffer.trim()) && !self.client_ui.search_buffer.trim().is_empty() {
                                            let group = ui.group(|ui|{
                                                ui.label(RichText::from(message.author.to_string()).size(self.font_size / 1.3).color(Color32::WHITE));
                                                ui.label(RichText::from(inner_message.message.to_string()));
                                                ui.small(&message.message_date);
                                            });

                                            if group.response.interact(Sense::click()).clicked() {
                                                self.client_ui.scroll_to_message_index = Some(index)
                                            };

                                            group.response.on_hover_text("Click to jump to message");

                                            has_search = true;
                                        }
                                    }
                                },
                                SearchType::Reply => {
                                    if let ServerMessageType::Normal(inner_message) = &message.message_type {
                                        if message.replying_to.is_some() && !self.client_ui.search_buffer.trim().is_empty() {
                                            let group = ui.group(|ui|{
                                                ui.label(RichText::from(message.author.to_string()).size(self.font_size / 1.3).color(Color32::WHITE));
                                                ui.label(RichText::from(inner_message.message.to_string()));
                                                ui.small(&message.message_date);
                                            });

                                            if group.response.interact(Sense::click()).clicked() {
                                                self.client_ui.scroll_to_message_index = Some(index)
                                            };

                                            group.response.on_hover_text("Click to jump to message");

                                            has_search = true;
                                        }
                                    }
                                }
                                SearchType::File => {
                                    if let ServerMessageType::Upload(inner_message) = &message.message_type {
                                        let group = ui.group(|ui|{
                                            ui.label(RichText::from(message.author.to_string()).size(self.font_size / 1.3).color(Color32::WHITE));

                                            //This button shouldnt actually do anything becuase when this message group gets clicked it throws you to the message
                                            if ui.small_button(inner_message.file_name.to_string()).clicked() {
                                                self.client_ui.scroll_to_message_index = Some(index)
                                            };
                                            ui.small(&message.message_date);
                                        });

                                        if group.response.interact(Sense::click()).clicked() {
                                            self.client_ui.scroll_to_message_index = Some(index)
                                        };

                                        group.response.on_hover_text("Click to jump to message");

                                        has_search = true;
                                    }
                                    /* Inner value shouldnt actaully be used since its only used for asking for a file, and to stay compact i wont implement image displaying in search mode */
                                    if let ServerMessageType::Image( _ ) = &message.message_type {
                                        let group = ui.group(|ui|{
                                            ui.label(RichText::from(message.author.to_string()).size(self.font_size / 1.3).color(Color32::WHITE));

                                            //This button shouldnt actually do anything becuase when this message group gets clicked it throws you to the message
                                            if ui.small_button("Image").clicked() {
                                                self.client_ui.scroll_to_message_index = Some(index)
                                            };
                                            ui.small(&message.message_date);
                                        });

                                        if group.response.interact(Sense::click()).clicked() {
                                            self.client_ui.scroll_to_message_index = Some(index)
                                        };

                                        group.response.on_hover_text("Click to jump to message");

                                        has_search = true;
                                    }
                                    if let ServerMessageType::Audio( _ ) = &message.message_type {
                                        let group = ui.group(|ui|{
                                            ui.label(RichText::from(message.author.to_string()).size(self.font_size / 1.3).color(Color32::WHITE));

                                            //This button shouldnt actually do anything becuase when this message group gets clicked it throws you to the message
                                            if ui.small_button("Audio").clicked() {
                                                self.client_ui.scroll_to_message_index = Some(index)
                                            };
                                            ui.small(&message.message_date);
                                        });
                                        if group.response.interact(Sense::click()).clicked() {
                                            self.client_ui.scroll_to_message_index = Some(index)
                                        };

                                        group.response.on_hover_text("Click to jump to message");

                                        has_search = true;
                                    }

                                }
                            }
                        }

                        //Display no result :(
                        if !has_search && !self.client_ui.search_buffer.trim().is_empty() {
                            ui.label(RichText::from("Based on these parameters, no messages were found").color(Color32::RED));
                        }

                    });
                });
            });
        }

        //This is only to display the files added to the list which will be sent
        self.file_tray(ctx);

        let panel_height = match usr_panel {
            Some(panel) => panel.response.interact_rect.size()[1],
            None => 0.,
        };

        //message box expanded
        Area::new("usr_msg_expand".into())
            .anchor(
                Align2::RIGHT_BOTTOM,
                match self.client_ui.usr_msg_expanded {
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
                        self.client_ui.usr_msg_expanded = !self.client_ui.usr_msg_expanded;
                    };
                });
            });

        //Server reciver
        self.client_recv(ctx);

        match self.audio_save_rx.try_recv() {
            Ok((sink, cursor, index, path_to_audio)) => {
                //Check if the request was unsuccesful, so we can reset the states
                if sink.is_none() {
                    //Reset state
                    self.client_ui.audio_playback.settings_list[index].is_loading = false;
                    return;
                }

                //Modify audio player
                self.client_ui.audio_playback.sink_list[index] = sink;

                //Set path
                self.client_ui.audio_playback.settings_list[index].path_to_audio = path_to_audio;

                let sink = self.client_ui.audio_playback.sink_list[index]
                    .as_mut()
                    .unwrap();

                let source = Decoder::new(
                    cursor.clone(), /*We can assume its always Some because we just set it to some above (lol)*/
                );

                match source {
                    Ok(source) => {
                        sink.append(source);

                        sink.play();
                    }
                    Err(err) => {
                        display_error_message(err);
                    }
                }

                self.client_ui.audio_playback.settings_list[index].cursor = cursor;
                //Reset button state so it can be used again
                self.client_ui.audio_playback.settings_list[index].is_loading = false;
            }
            Err(_err) => {}
        }
    }

    ///This functions is used for clients to recive messages from the server (this doesnt not check validity of the order of the messages, altough this may not be needed as tcp takes care of this)
    fn client_recv(&mut self, ctx: &egui::Context) {
        //This should only run when the connection is valid
        if let ConnectionState::Connected(connection_pair) = self.client_connection.state.clone() {
            self.server_sender_thread.get_or_insert_with(|| {
                //Clone so we can move it into the closure
                let sender = self.server_output_sender.clone();

                //Clone the reader so we can move it in the closure
                let reader = connection_pair.reader.clone();

                //Clone the sender so that 2 threads can each get a sender
                let sender_clone = sender.clone();

                //We clone ctx, so we can call request_repaint from inside the thread
                let context_clone = ctx.clone();

                //Thread cancellation token
                let shutdown_token = self.autosync_shutdown_token.child_token();

                //We have to clone for the 2nd thread
                let shutdown_token_clone = shutdown_token.clone();

                //Spawn server reader thread
                tokio::spawn(async move {
                    loop {
                        let server_reply_handle = &ServerReply {
                            reader: reader.clone(),
                        };

                        select! {
                        //Recive input from main thread to shutdown
                            _ = shutdown_token.cancelled() => {
                                break;
                            },

                            reply = ServerReply::wait_for_response(server_reply_handle) => {
                                match reply {
                                    //If we have a reponse from the server
                                    Ok(response) => {
                                        //Check for special cases like server disconnecting
                                        if response == "Server disconnecting from client." {
                                            break;
                                        }

                                        //Request repaint
                                        context_clone.request_repaint();
                                        
                                        //Send to reciver
                                        sender_clone.send(Some(response)).expect("Error occured when trying to send message, after reciving message from client");
                                    },
                                    Err(err) => {
                                        dbg!(&err);
                                        eprintln!("client.rs\nError occured when the client tried to recive a message from the server: {err}");
                                        eprintln!("Early end of file error is a normal occurence after disconnecting");
                                        display_error_message(err);

                                        //Error appeared, after this the tread quits, so there arent an inf amount of threads running
                                        let _ = sender_clone.send(None);

                                        break;
                                    },
                                }
                            }
                        }
                    }
                });

                //Init sync message
                let mut message = ClientMessage::construct_sync_msg(
                    &self.client_connection.password,
                    &self.login_username,
                    &self.opened_user_information.uuid,
                    //Send how many messages we have, the server will compare it to its list, and then send the missing messages, reducing traffic
                    self.client_ui.incoming_msg.struct_list.len(),
                    Some(*self.client_ui.last_seen_msg_index.lock().unwrap()),
                );

                let last_seen_message_index = self.client_ui.last_seen_msg_index.clone();

                //Spawn server syncer thread
                tokio::spawn(async move {
                    loop {
                        //This patter match will always return true, the message were trying to pattern match is constructed above 
                        //We should update the message for syncing, so we will provide the latest info to the server
                        if let ClientMessageType::SyncMessage(inner) = &mut message.message_type {
                            tokio::time::sleep(Duration::from_secs(2)).await;

                            //We should only check for the value after sleep
                            if shutdown_token_clone.is_cancelled() {
                                break;
                            }

                            let index = *last_seen_message_index.lock().unwrap();

                            if inner.last_seen_message_index < Some(index) {
                                inner.last_seen_message_index = Some(index);

                                //We only send a sync packet if we need to
                                //We only have to send the sync message, since in the other thread we are reciving every message sent to us
                                match connection_pair.send_message(message.clone()).await {
                                    Ok(_) => {},
                                    Err(err) => {
                                        dbg!(err);
                                        //Error appeared, after this the tread quits, so there arent an inf amount of threads running
                                        sender.send(None).expect("Failed to signal thread error");
                                        break;
                                    }
                                };
                            }
                        }
                        else
                        {
                            panic!("The message watning to be sent isnt a clientsyncmessage (as required), check what youve modified");
                        }
                    }
                });
            });

            //Try to recive the threads messages
            //Get sent to the channel to be displayed, if the connections errors out, do nothing lol cuz its prolly cuz the sender hadnt done anything
            match self.server_output_reciver.try_recv() {
                Ok(msg) => {
                    //show messages
                    if let Some(message) = msg {
                        //Decrypt the server's reply
                        match decrypt_aes256(&message, &self.client_connection.client_secret) {
                            Ok(decrypted_message) => {
                                let incoming_struct: Result<ServerSync, serde_json::Error> =
                                    serde_json::from_str(&decrypted_message);
                                match incoming_struct {
                                    Ok(msg) => {
                                        //Always make sure to store the latest user_seen list
                                        self.client_ui.incoming_msg.user_seen_list =
                                            msg.user_seen_list;

                                        //If its a sync message then we dont need to back it up
                                        if matches!(
                                            msg.message.message_type,
                                            ServerMessageType::Sync(_)
                                        ) {
                                            return;
                                        }

                                        match &msg.message.message_type {
                                            ServerMessageType::Edit(message) => {
                                                if let Some(new_message) =
                                                    message.new_message.clone()
                                                {
                                                    if let ServerMessageType::Normal(inner) =
                                                        &mut self.client_ui.incoming_msg.struct_list
                                                            [message.index as usize]
                                                            .message_type
                                                    {
                                                        inner.message = new_message;
                                                        inner.has_been_edited = true;
                                                    }
                                                } else {
                                                    self.client_ui.incoming_msg.struct_list
                                                        [message.index as usize]
                                                        .message_type = ServerMessageType::Deleted;
                                                }
                                            }
                                            ServerMessageType::Reaction(message) => {
                                                //Search if there has already been a reaction added
                                                if let Some(index) =
                                                    self.client_ui.incoming_msg.reaction_list
                                                        [message.index as usize]
                                                        .message_reactions
                                                        .iter()
                                                        .position(|item| item.char == message.char)
                                                {
                                                    //If yes, increment the reaction counter
                                                    self.client_ui.incoming_msg.reaction_list
                                                        [message.index as usize]
                                                        .message_reactions[index]
                                                        .times += 1;
                                                } else {
                                                    //If no, add a new reaction counter
                                                    self.client_ui.incoming_msg.reaction_list
                                                        [message.index as usize]
                                                        .message_reactions
                                                        .push(Reaction {
                                                            char: message.char,
                                                            times: 1,
                                                        })
                                                }
                                            }
                                            _ => {
                                                //Allocate Message vec for the new message
                                                self.client_ui
                                                    .incoming_msg
                                                    .reaction_list
                                                    .push(MessageReaction::default());

                                                //We can append the missing messages sent from the server, to the self.client_ui.incoming_msg.struct_list vector
                                                self.client_ui
                                                    .incoming_msg
                                                    .struct_list
                                                    .push(msg.message);
                                            }
                                        }
                                    }
                                    //If converting the message to a ServerSync then it was probably a ServerReplyType enum
                                    Err(_err) => {
                                        let incoming_reply: Result<
                                            ServerReplyType,
                                            serde_json::Error,
                                        > = serde_json::from_str(&decrypted_message);

                                        match incoming_reply {
                                            Ok(inner) => {
                                                match inner {
                                                    ServerReplyType::FileReply(file) => {
                                                        let _ = write_file(file);
                                                    }
                                                    ServerReplyType::ImageReply(image) => {
                                                        //Forget image so itll be able to get displayed
                                                        ctx.forget_image(&format!(
                                                            "bytes://{}",
                                                            image.index
                                                        ));

                                                        //load image to the said URI
                                                        ctx.include_bytes(
                                                            format!("bytes://{}", image.index),
                                                            image.bytes,
                                                        );
                                                    }
                                                    ServerReplyType::AudioReply(audio) => {
                                                        let stream_handle = self
                                                            .client_ui
                                                            .audio_playback
                                                            .stream_handle
                                                            .clone();

                                                        let sender = self.audio_save_tx.clone();

                                                        //ONLY USE THIS PATH WHEN YOU ARE SURE THAT THE FILE SPECIFIED ON THIS PATH EXISTS
                                                        let path_to_audio = PathBuf::from(format!(
                                                            "{}\\Matthias\\Client\\{}\\Audios\\{}",
                                                            env!("APPDATA"),
                                                            self.client_ui
                                                                .send_on_ip_base64_encoded,
                                                            audio.index
                                                        ));

                                                        let _ = write_audio(
                                                            audio.clone(),
                                                            self.client_ui.send_on_ip.clone(),
                                                        );

                                                        while !path_to_audio.exists() {
                                                            //Block until it exists, we can do this because we are in a different thread then main
                                                        }

                                                        let file_stream_to_be_read =
                                                            fs::read(&path_to_audio)
                                                                .unwrap_or_default();
                                                        let cursor = PlaybackCursor::new(
                                                            file_stream_to_be_read,
                                                        );
                                                        let sink = Some(
                                                            Sink::try_new(&stream_handle).unwrap(),
                                                        );

                                                        sender
                                                            .send((
                                                                sink,
                                                                cursor,
                                                                //Is this needed
                                                                audio.index as usize,
                                                                path_to_audio,
                                                            ))
                                                            .unwrap();
                                                    }
                                                    ServerReplyType::ClientReply(client_reply) => {
                                                        self.client_ui
                                                            .incoming_msg
                                                            .connected_clients_profile
                                                            .insert(
                                                                client_reply.uuid.clone(),
                                                                client_reply.profile.clone(),
                                                            );

                                                        //Forget old placeholder bytes
                                                        ctx.forget_image(&format!(
                                                            "bytes://{}",
                                                            client_reply.uuid
                                                        ));

                                                        //Pair URI with profile image
                                                        ctx.include_bytes(
                                                            format!(
                                                                "bytes://{}",
                                                                client_reply.uuid
                                                            ),
                                                            client_reply
                                                                .profile
                                                                .small_profile_picture,
                                                        );
                                                    }
                                                }
                                            }
                                            Err(_err) => {
                                                dbg!(_err);
                                            }
                                        }
                                    }
                                }
                            }
                            Err(_err) => {
                                display_error_message(message);
                            }
                        }
                    } else {
                        //Signal the remaining thread to be shut down
                        // self.autosync_shutdown_token.cancel();
                        // wtf? investigate

                        //Then the thread got an error, we should reset the state
                        dbg!("Client reciver or sync thread panicked");
                    }
                }
                Err(_err) => {
                    // dbg!(_err);
                }
            }
        }
    }
}
