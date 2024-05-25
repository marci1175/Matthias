use std::{fs, path::PathBuf};

use egui::{vec2, Align, Align2, Area, Color32, Context, Layout, RichText, Sense, Ui};
use regex::Regex;

use crate::app::backend::{
    write_file, ClientMessage, ServerFileReply, ServerImageUpload, TemplateApp,
};
use rodio::{Decoder, Sink, Source};
use tap::TapFallible;

//use crate::app::account_manager::write_file;
use crate::app::backend::{display_error_message, write_audio, PlaybackCursor, ServerAudioReply};
impl TemplateApp {
    pub fn message_display(
        &mut self,
        message: &crate::app::backend::ServerOutput,
        ui: &mut Ui,
        ctx: &egui::Context,
        current_index_in_message_list: usize,
    ) {
        match &message.MessageType {
            //File upload
            crate::app::backend::ServerMessageType::Upload(inner) => {
                let button =
                    ui.button(RichText::from(inner.file_name.to_string()).size(self.font_size));
                button.paint_debug_info();

                //If we want to download the file included in the message
                if button.clicked() {
                    let passw = self.client_ui.client_password.clone();
                    let author = self.login_username.clone();
                    let message =
                        ClientMessage::construct_file_request_msg(inner.index, &passw, author);

                    let connection = self.client_connection.clone();

                    tokio::spawn(async move {
                        match connection.send_message(message).await {
                            Ok(ok) => {
                                //If we cant wait for response, we should panic
                                let msg = ok.wait_for_response().await.unwrap();

                                let file_serve: Result<ServerFileReply, serde_json::Error> =
                                    serde_json::from_str(&msg);
                                let _ = write_file(file_serve.unwrap());
                            }
                            Err(err) => {
                                println!("{err} ln 264")
                            }
                        }
                    });
                }
            }
            crate::app::backend::ServerMessageType::Normal(inner_message) => {
                if (inner_message.message.contains('[') && inner_message.message.contains(']'))
                    && (inner_message.message.contains('(') && inner_message.message.contains(')'))
                {
                    let regex =
                        Regex::new(r"\[\s*(?P<text>[^\]]*)\]\((?P<link_target>[^)]+)\)").unwrap();

                    let mut captures: Vec<String> = Vec::new();

                    for capture in regex.captures_iter(&inner_message.message) {
                        //We iterate over all the captures
                        for i in 1..capture.len() {
                            //We push back the captures into the captures vector
                            captures.push(capture[i].to_string());
                        }
                    }

                    if captures.is_empty() {
                        ui.label(RichText::from(&inner_message.message).size(self.font_size));
                    } else {
                        ui.horizontal(|ui| {
                            let inner_message_clone = inner_message.message.clone();

                            let temp = inner_message_clone.split_whitespace().collect::<Vec<_>>();

                            for item in temp.iter() {
                                if let Some(capture) = regex.captures(item) {
                                    // capture[0] combined
                                    // capture[1] disp
                                    // capture[2] URL
                                    ui.hyperlink_to(capture[1].to_string(), capture[2].to_string());
                                } else {
                                    ui.label(*item);
                                }
                            }
                        });
                    }
                } else if let Some(index) = inner_message.message.find('@') {
                    let whole_tag = inner_message.message[index + 1..]
                        .split_whitespace()
                        .collect::<Vec<&str>>();

                    let name_sent_to = whole_tag.first();
                    ui.label(
                        RichText::from(&inner_message.message)
                            .size(self.font_size)
                            .color({
                                if let Some(tagged_name) = name_sent_to {
                                    if *tagged_name == self.opened_account.username {
                                        Color32::YELLOW
                                    } else {
                                        Color32::GRAY
                                    }
                                } else {
                                    Color32::GRAY
                                }
                            }),
                    );
                } else if inner_message.message.contains('#')
                    && inner_message.message.rmatches('#').count() <= 5
                {
                    let split_lines = inner_message.message.rsplit_once('#').unwrap();
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::from(split_lines.0.replace('#', "")).size(self.font_size),
                        );
                        ui.label(
                            RichText::from(split_lines.1).strong().size(
                                self.font_size
                                    * match inner_message
                                        .message
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
                                    } as f32,
                            ),
                        );
                    });
                } else {
                    ui.label(RichText::from(&inner_message.message).size(self.font_size));
                }
            }
            crate::app::backend::ServerMessageType::Image(picture) => {
                let path = PathBuf::from(format!(
                    "{}\\Matthias\\Client\\{}\\Images\\{}",
                    env!("APPDATA"),
                    self.client_ui.send_on_ip_base64_encoded,
                    picture.index
                ));
                ui.allocate_ui(vec2(300., 300.), |ui| {
                    match fs::read(&path) {
                        Ok(image_bytes) => {
                            //display picture from bytes
                            let image_widget = ui.add(egui::widgets::Image::from_bytes(
                                format!("bytes://{}", picture.index),
                                image_bytes.clone(),
                            ));

                            if image_widget.interact(Sense::click()).clicked() {
                                self.client_ui.image_overlay = true;
                            }

                            image_widget.context_menu(|ui| {
                                if ui.button("Save").clicked() {
                                    //always name the file ".png", NOTE: USE WRITE FILE BECAUSE WRITE IMAGE IS AUTOMATIC WITHOUT ASKING THE USER
                                    let image_save = ServerFileReply {
                                        bytes: image_bytes.clone(),
                                        file_name: PathBuf::from("image.png"),
                                    };
                                    let _ = crate::app::backend::write_file(image_save);
                                }
                            });

                            if self.client_ui.image_overlay {
                                self.image_overlay_draw(ui, ctx, image_bytes, picture);
                            }
                        }
                        Err(_err) => {
                            //create decoy file, to manually create a race condition
                            let _ = fs::create_dir_all(PathBuf::from(format!(
                                "{}\\Matthias\\Client\\{}\\Images",
                                env!("APPDATA"),
                                self.client_ui.send_on_ip_base64_encoded,
                            )));

                            if let Err(err) = std::fs::write(
                                path,
                                "This is a placeholder file, this will get overwritten (hopefully)",
                            ) {
                                println!("Error when creating a decoy: {err}");
                                return;
                            };

                            //check if we are visible
                            if !ui.is_rect_visible(ui.min_rect()) {
                                return;
                            }

                            //We dont have file on our local system so we have to ask the server to provide it
                            let uuid = &self.opened_account.uuid;
                            let author = self.login_username.clone();
                            let sender = self.itx.clone();

                            let message = ClientMessage::construct_image_request_msg(
                                picture.index,
                                uuid,
                                author,
                            );

                            let connection = self.client_connection.clone();

                            tokio::spawn(async move {
                                match connection.send_message(message).await {
                                    Ok(ok) => {
                                        //If a problem appeared whilst waiting for response its okay to panic since it is not the main thread
                                        let msg = ok.wait_for_response().await.unwrap();

                                        match sender.send(msg) {
                                            Ok(_) => {}
                                            Err(err) => {
                                                println!("{}", err);
                                            }
                                        };
                                    }
                                    Err(err) => {
                                        println!("{err} ln 264")
                                    }
                                }
                            });
                        }
                    };
                })
                .response
                .paint_debug_info();
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
                    audio.index
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
                                            let sender = self.audio_save_tx.clone();

                                            let message =
                                                ClientMessage::construct_audio_request_msg(
                                                    audio.index,
                                                    &self.opened_account.uuid,
                                                    self.login_username.clone(),
                                                );

                                            let connection = self.client_connection.clone();
                                            let send_on_ip = self.client_ui.send_on_ip.clone();
                                            let stream_handle =
                                                self.client_ui.audio_playback.stream_handle.clone();
                                            let current_index = current_index_in_message_list;

                                            tokio::spawn(async move {
                                                match connection.send_message(message).await {
                                                    Ok(response) => {
                                                        let message = response
                                                            .wait_for_response()
                                                            .await
                                                            .unwrap();

                                                        let file_serve: Result<
                                                            ServerAudioReply,
                                                            serde_json::Error,
                                                        > = serde_json::from_str(&message);
                                                        let _ = write_audio(
                                                            file_serve.unwrap(),
                                                            send_on_ip,
                                                        );

                                                        let file_stream_to_be_read =
                                                            fs::read(&path_to_audio)
                                                                .unwrap_or_default();
                                                        let cursor = PlaybackCursor::new(
                                                            file_stream_to_be_read,
                                                        );
                                                        let sink = Some(
                                                            Sink::try_new(&stream_handle).unwrap(),
                                                        );

                                                        let _ = sender
                                                            .send((
                                                                sink,
                                                                cursor,
                                                                current_index,
                                                                path_to_audio,
                                                            ))
                                                            .tap_err_dbg(|dbg| {
                                                                tracing::error!("{dbg:?}")
                                                            });
                                                    }
                                                    Err(err) => {
                                                        //The error will be logged
                                                        tracing::error!("{err}");

                                                        //The error will be displayed here
                                                        display_error_message(err);

                                                        //The error will be sent, we wont have to do anything when reciving it
                                                        let _ = sender
                                                            .send((
                                                                None,
                                                                PlaybackCursor::new(Vec::new()),
                                                                current_index,
                                                                path_to_audio,
                                                            ))
                                                            .tap_err_dbg(|dbg| {
                                                                tracing::error!("{dbg:?}")
                                                            });
                                                    }
                                                }
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
                            format!("bytes://{}", picture.index),
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
