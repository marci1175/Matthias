use std::{fs, path::PathBuf};

use egui::{vec2, Align, Align2, Area, Color32, Context, Image, Layout, RichText, Sense, Ui};

use crate::app::backend::{
    parse_incoming_message, write_file, Application, ClientMessage, MessageDisplay,
    ServerFileReply, ServerImageUpload,
};
use rodio::{Decoder, Source};

//use crate::app::account_manager::write_file;
use crate::app::backend::PlaybackCursor;
impl Application {
    /// This function is used to displayed the messages wrapped information (The message itself)
    pub fn message_display(
        &mut self,
        message: &crate::app::backend::ServerOutput,
        ui: &mut Ui,
        ctx: &egui::Context,
        current_index_in_message_list: usize,
    ) {
        match &message.message_type {
            //File upload
            crate::app::backend::ServerMessageType::Upload(inner) => {
                let button =
                    ui.button(RichText::from(inner.file_name.to_string()).size(self.font_size));
                button.paint_debug_info();
                //If we want to download the file included in the message
                if button.clicked() {
                    let message = ClientMessage::construct_file_request_msg(
                        inner.signature.clone(),
                        &self.opened_user_information.uuid,
                    );

                    let connection = self.client_connection.clone();

                    tokio::spawn(async move {
                        connection.send_message(message).await.unwrap();
                    });
                }
            }
            crate::app::backend::ServerMessageType::Normal(message) => {
                let messages = parse_incoming_message(message.message.clone());
                let mut messages_iter = messages.iter();

                'mainloop: loop {
                    let mut cont = false;

                    ui.horizontal_wrapped(|ui| {
                        for message in messages_iter.by_ref() {
                            //If there is a newline in the messages vector we need to break out of the horizontal wrapped "loop", so well keep drawing in the next line
                            if message.inner_message == MessageDisplay::NewLine {
                                cont = true;
                                return;
                            }

                            message.display(ui, ctx);
                        }
                    });

                    if cont {
                        continue 'mainloop;
                    }

                    //Break when we have finished iterating over the messages
                    break;
                }
            }
            crate::app::backend::ServerMessageType::Image(picture) => {
                ui.allocate_ui(vec2(300., 300.), |ui| {
                    match ctx.try_load_bytes(&format!("bytes://{}", picture.signature)) {
                        Ok(bytes_poll) => {
                            //display picture from bytes
                            if let egui::load::BytesPoll::Ready {bytes, ..} = bytes_poll {
                                //If the bytes are indicated as being requested we can put there a spinner
                                if bytes.to_vec() == vec![0] {
                                    ui.spinner();
                                    return;
                                }
                                let image_widget = ui.add(egui::widgets::Image::from_uri(
                                    format!("bytes://{}", picture.signature),
                                ));
                                if image_widget.interact(Sense::click()).clicked() {
                                    self.client_ui.image_overlay = true;
                                }
                                image_widget.context_menu(|ui| {
                                    if ui.button("Save").clicked() {
                                        //always name the file ".png", NOTE: USE WRITE FILE BECAUSE WRITE IMAGE IS AUTOMATIC WITHOUT ASKING THE USER
                                        let image_save = ServerFileReply {
                                            bytes: bytes.to_vec(),
                                            file_name: PathBuf::from("image.png"),
                                        };
                                        let _ = crate::app::backend::write_file(image_save);
                                    }
                                });
                                if self.client_ui.image_overlay {
                                    self.image_overlay_draw(ui, ctx, bytes.to_vec(), picture);
                                }
                            }
                        }
                        Err(load_error) => {
                            if let egui::load::LoadError::Loading(inner) = load_error {
                                if inner == "Bytes not found. Did you forget to call Context::include_bytes?" {
                                    //check if we are visible
                                    if !ui.is_rect_visible(ui.min_rect()) {
                                        return;
                                    }
                                    //Load an empty byte to the said URI
                                    ctx.include_bytes(format!("bytes://{}", picture.signature), vec![0]);
                                    //We dont have file on our local system so we have to ask the server to provide it
                                    let uuid = &self.opened_user_information.uuid;
                                    let message =
                                        ClientMessage::construct_image_request_msg(picture.signature.clone(), uuid);
                                    let connection = self.client_connection.clone();
                                    tokio::spawn(async move {
                                        //We only have to send the message it will get recived in a diff place
                                        connection.clone().send_message(message).await.unwrap();
                                    });
                                }
                                else {
                                    dbg!(inner);
                                }
                            }
                        }
                    };
                });
            }
            crate::app::backend::ServerMessageType::Audio(audio) => {
                //Create folder for audios for later problem avoidance
                let _ = fs::create_dir_all(PathBuf::from(format!(
                    "{}{}{}{}",
                    env!("APPDATA"),
                    "\\Matthias\\Client\\",
                    self.client_ui.send_on_ip_base64_encoded,
                    "\\Audios"
                )));

                //ONLY USE THIS PATH WHEN YOU ARE SURE THAT THE FILE SPECIFIED ON THIS PATH EXISTS
                let path_to_audio = PathBuf::from(format!(
                    "{}\\Matthias\\Client\\{}\\Audios\\{}",
                    env!("APPDATA"),
                    self.client_ui.send_on_ip_base64_encoded,
                    audio.signature
                ));

                ui.allocate_ui(vec2(300., 150.), |ui| {
                    ui.with_layout(Layout::top_down(Align::Center), |ui| {
                        match self.client_ui.audio_playback.sink_list[current_index_in_message_list]
                            .as_mut()
                        {
                            Some(sink) => match sink.is_paused() {
                                //Audio is stopped
                                true => {
                                    if ui.button("Play").clicked() {
                                        sink.play();
                                    };
                                }
                                //Audio is running
                                false => {
                                    //Display cursor placement
                                    let mut cursor = self.client_ui.audio_playback.settings_list
                                        [current_index_in_message_list]
                                        .cursor
                                        .cursor
                                        .lock()
                                        .unwrap();

                                    //Construct new decoder
                                    if let Ok(decoder) = Decoder::new(PlaybackCursor::new(
                                        cursor.clone().into_inner(),
                                    )) {
                                        // Always set the cursor_pos to the cursor's position as a temp value
                                        let mut cursor_pos =
                                            <std::io::Cursor<std::vec::Vec<u8>> as Clone>::clone(
                                                &cursor,
                                            )
                                            .into_inner()
                                            .len()
                                                / decoder.sample_rate() as usize;

                                        //Why the fuck does this always return a None?!
                                        if let Some(total_dur) = dbg!(decoder.total_duration()) {
                                            // If it has been changed, then change the real cursors position too
                                            if ui
                                                .add(
                                                    egui::Slider::new(
                                                        &mut cursor_pos,
                                                        0..=total_dur.as_secs() as usize,
                                                    )
                                                    .show_value(false)
                                                    .text("Set player"),
                                                )
                                                .changed()
                                            {
                                                //Set cursor poition
                                                cursor.set_position(
                                                    (cursor_pos * decoder.sample_rate() as usize)
                                                        as u64,
                                                );
                                            };
                                        }
                                    };

                                    if ui.button("Stop").clicked() {
                                        sink.pause();
                                    }
                                }
                            },
                            None => {
                                let is_loading = self.client_ui.audio_playback.settings_list
                                    [current_index_in_message_list]
                                    .is_loading;

                                if is_loading {
                                    ui.label("Requesting file from server, please wait!");
                                }

                                //This should be enabled when the audio isnt loading
                                ui.add_enabled_ui(!is_loading, |ui| {
                                    if ui.button("Play").clicked() {
                                        //If the user has clicked the play button only then we download the desirted audio file! Great optimisation
                                        if !path_to_audio.exists() {
                                            let message =
                                                ClientMessage::construct_audio_request_msg(
                                                    audio.signature.clone(),
                                                    &self.opened_user_information.uuid,
                                                );

                                            let connection = self.client_connection.clone();

                                            tokio::spawn(async move {
                                                connection.send_message(message).await.unwrap();
                                            });

                                            //Set button to be disabled
                                            self.client_ui.audio_playback.settings_list
                                                [current_index_in_message_list]
                                                .is_loading = true;
                                        }
                                    };
                                })
                                .response
                                .paint_debug_info();
                            }
                        }
                    });

                    //Set properties of audio stream, when there is one
                    if let Some(sink) = self.client_ui.audio_playback.sink_list
                        [current_index_in_message_list]
                        .as_mut()
                    {
                        //Set volume
                        sink.set_volume(
                            self.client_ui.audio_playback.settings_list
                                [current_index_in_message_list]
                                .volume,
                        );

                        sink.set_speed(
                            self.client_ui.audio_playback.settings_list
                                [current_index_in_message_list]
                                .speed,
                        );
                    }

                    /*
                    let pos = self.client_ui.audio_playback.settings_list[current_index_in_message_list].cursor_offset;
                    if let Some(cursor) = self.client_ui.audio_playback.settings_list[current_index_in_message_list].cursor.as_mut() {
                        cursor.set_position(pos);
                        let range = self.client_ui.audio_playback.settings_list
                        [current_index_in_message_list]
                        .cursor.clone().unwrap().position() + self.client_ui.audio_playback.settings_list
                        [current_index_in_message_list]
                        .cursor.clone().unwrap().remaining_slice().len() as u64;
                        //Cursor
                        ui.add(
                            egui::Slider::new(
                                &mut self.client_ui.audio_playback.settings_list
                                    [current_index_in_message_list]
                                    .cursor_offset,
                                0..=range,
                            )
                            .text("Volume")
                            .step_by(1.)
                        );
                    }
                    */

                    ui.label(&audio.file_name);
                    //Audio volume
                    ui.add(
                        egui::Slider::new(
                            &mut self.client_ui.audio_playback.settings_list
                                [current_index_in_message_list]
                                .volume,
                            0.01..=5.,
                        )
                        .text("Volume")
                        .step_by(0.01),
                    );
                    //Audio speed
                    ui.add(
                        egui::Slider::new(
                            &mut self.client_ui.audio_playback.settings_list
                                [current_index_in_message_list]
                                .speed,
                            0.01..=5.,
                        )
                        .text("Speed")
                        .step_by(0.01),
                    );
                });
            }
            crate::app::backend::ServerMessageType::Deleted => {
                ui.label(
                    RichText::from("Deleted message")
                        .strong()
                        .size(self.font_size),
                );
            }
            crate::app::backend::ServerMessageType::Server(server_msg) => {
                let message = match server_msg {
                    crate::app::backend::ServerMessage::Connect(profile) => {
                        format!("@{} has connected to the server.", profile.username)
                    }
                    crate::app::backend::ServerMessage::Disconnect(profile) => {
                        format!("@{} has disconnected from the server.", profile.username)
                    }
                    crate::app::backend::ServerMessage::Ban(profile) => {
                        format!("@{} has been banned from the server.", profile.username)
                    }
                };

                //We can safely unwrap here since the message is defined above
                let message_tag_tdx = message.find('@').unwrap();
                let whole_tag = message[message_tag_tdx + 1..]
                    .split_whitespace()
                    .collect::<Vec<&str>>();

                let name_sent_to = whole_tag.first();
                ui.label(RichText::from(&message).size(self.font_size).color({
                    if let Some(tagged_name) = name_sent_to {
                        if *tagged_name == self.opened_user_information.username {
                            Color32::YELLOW
                        } else {
                            Color32::GRAY
                        }
                    } else {
                        Color32::GRAY
                    }
                }));
            }
            crate::app::backend::ServerMessageType::VoipConnection(connection_type) => {
                match connection_type {
                    crate::app::backend::ServerVoipState::Connected(client_uuid) => {
                        let profile = self
                            .client_ui
                            .incoming_messages
                            .connected_clients_profile
                            .get(client_uuid.as_str())
                            .unwrap()
                            .clone();

                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::from(format!(
                                    "@{} has connected to the group call.",
                                    profile.username
                                ))
                                .size(self.font_size),
                            );
                        });
                    }
                    crate::app::backend::ServerVoipState::Disconnected(client_uuid) => {
                        let profile = self
                            .client_ui
                            .incoming_messages
                            .connected_clients_profile
                            .get(client_uuid)
                            .unwrap()
                            .clone();

                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::from(format!(
                                    "@{} has disconnected from the group call.",
                                    profile.username
                                ))
                                .size(self.font_size),
                            );
                        });
                    }
                };
            }
            crate::app::backend::ServerMessageType::Edit(_)
            | crate::app::backend::ServerMessageType::Reaction(_)
            | crate::app::backend::ServerMessageType::Sync(_) => {
                unimplemented!("ServerMessageType::Edit(_) & ServerMessageType::Reaction(_) & crate::app::backend::ServerMessageType::Sync(_) should not be displayed")
            }
        }
    }

    pub fn image_overlay_draw(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &Context,
        image_bytes: Vec<u8>,
        picture: &ServerImageUpload,
    ) {
        //Image overlay
        ui.painter().rect_filled(
            egui::Rect::EVERYTHING,
            0.,
            Color32::from_rgba_premultiplied(0, 0, 0, 180),
        );

        Area::new("image_overlay".into())
            .movable(false)
            .anchor(Align2::CENTER_CENTER, vec2(0., 0.))
            .show(ctx, |ui| {
                ui.allocate_ui(
                    vec2(ui.available_width() / 1.3, ui.available_height() / 1.3),
                    |ui| {
                        ui.add(egui::widgets::Image::from_bytes(
                            format!("bytes://{}", picture.signature),
                            image_bytes.clone(),
                        )) /*Add the same context menu as before*/
                        .context_menu(|ui| {
                            if ui.button("Save").clicked() {
                                //always name the file ".png"
                                let image_save = ServerFileReply {
                                    bytes: image_bytes.clone(),
                                    file_name: PathBuf::from(".png"),
                                };
                                let _ = write_file(image_save);
                            }
                        });
                    },
                );
            });

        Area::new("image_overlay_exit".into())
            .movable(false)
            .anchor(Align2::RIGHT_TOP, vec2(-100., 100.))
            .show(ctx, |ui| {
                ui.allocate_ui(vec2(25., 25.), |ui| {
                    if ui
                        .add(egui::ImageButton::new(egui::include_image!(
                            "../../../../../icons/cross.png"
                        )))
                        .clicked()
                    {
                        self.client_ui.image_overlay = false;
                    }
                })
            });
    }
}
