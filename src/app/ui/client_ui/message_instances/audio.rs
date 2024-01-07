use base64::engine::general_purpose;
use base64::Engine;
use egui::{vec2, Align, Layout};
use rodio::{Decoder, Sink};
use std::path::PathBuf;

//use crate::app::account_manager::write_file;
use crate::app::backend::{ClientMessage, PlaybackCursor, ServerMessageType, TemplateApp};
use crate::app::client::{self};
use std::fs;
impl TemplateApp {
    pub fn audio_message_instance(
        &mut self,
        item: &crate::app::backend::ServerOutput,
        ui: &mut egui::Ui,
        current_index_in_message_list: usize,
    ) {
        self.client_ui.send_on_ip_base64_encoded =
            general_purpose::URL_SAFE_NO_PAD.encode(self.client_ui.send_on_ip.clone());

        //Create folder for audios for later problem avoidance
        let _ = fs::create_dir_all(PathBuf::from(format!(
            "{}{}{}{}",
            env!("APPDATA"),
            "\\Matthias\\Client\\",
            self.client_ui.send_on_ip_base64_encoded,
            "\\Audios"
        )));

        if let ServerMessageType::Audio(audio) = &item.MessageType {
            let path = PathBuf::from(format!(
                "{}\\Matthias\\Client\\{}\\Audios\\{}",
                env!("APPDATA"),
                self.client_ui.send_on_ip_base64_encoded,
                audio.index
            ));
            ui.allocate_ui(vec2(300., 150.), |ui| {
                match path.exists() {
                    true => {
                        //if we already have the sound file :::

                        ui.with_layout(Layout::top_down(Align::Center), |ui| {
                            match self.client_ui.audio_playback.sink_list[current_index_in_message_list]
                                .as_mut()
                            {
                                Some(sink) => match sink.is_paused() {
                                    true => {
                                        if ui.button("Play").clicked() {
                                            sink.play();
                                        };
                                    }
                                    false => {
                                        //ui.label(format!("{:?}", self.client_ui.audio_playback.settings_list[current_index_in_message_list].cursor.cursor.lock().unwrap().remaining_slice().len()));
                                        //let cursor = self.client_ui.audio_playback.settings_list[current_index_in_message_list].cursor.cursor.lock().unwrap();
                                        if ui.button("Stop").clicked() {
                                            sink.clear();
                                            //Reset value
                                            self.client_ui.audio_playback.sink_list[current_index_in_message_list] = None;
                                        }
                                    }
                                },
                                None => {
                                    if ui.button("Play").clicked() {
                                        self.client_ui.send_on_ip_base64_encoded =
                                            general_purpose::URL_SAFE_NO_PAD.encode(self.client_ui.send_on_ip.clone());
                                        let file = PathBuf::from(format!(
                                            "{}\\Matthias\\Client\\{}\\Audios\\{}",
                                            env!("APPDATA"),
                                            self.client_ui.send_on_ip_base64_encoded,
                                            audio.index
                                        ));

                                        let file_stream_to_be_read = fs::read(file).unwrap_or_default();
                                        self.client_ui.audio_playback.settings_list[current_index_in_message_list].cursor = PlaybackCursor::new(file_stream_to_be_read);
                                        self.client_ui.audio_playback.sink_list
                                            [current_index_in_message_list] = Some(
                                            Sink::try_new(&self.client_ui.audio_playback.stream_handle)
                                                .unwrap(),
                                        );
                                        let sink = self.client_ui.audio_playback.sink_list
                                            [current_index_in_message_list]
                                            .as_mut()
                                            .unwrap();

                                        let source = Decoder::new(self.client_ui.audio_playback.settings_list[current_index_in_message_list].cursor.clone() /*We can assume its always Some because we just set it to some above (lol)*/);
                                        match source {
                                            Ok(source) => {

                                                sink.append(source);
                                                sink.play();

                                            }
                                            Err(err) => {
                                                dbg!(err);
                                            }
                                        }

                                    };
                                }
                            }
                        });

                        //Set properties of audio stream, when there is one
                        if let Some(sink) =
                            self.client_ui.audio_playback.sink_list[current_index_in_message_list].as_mut()
                        {
                            //Set volume
                            sink.set_volume(
                                self.client_ui.audio_playback.settings_list[current_index_in_message_list]
                                    .volume,
                            );

                            sink.set_speed(
                                self.client_ui.audio_playback.settings_list[current_index_in_message_list]
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
                                0.0..=5.,
                            )
                            .text("Volume")
                            .step_by(0.05),
                        );
                        //Audio speed
                        ui.add(
                            egui::Slider::new(
                                &mut self.client_ui.audio_playback.settings_list
                                    [current_index_in_message_list]
                                    .speed,
                                0.1..=5.,
                            )
                            .text("Speed")
                            .step_by(0.05),
                        );
                    }
                    false => {
                        //create decoy file, to manually create a race condition
                        if let Err(err) = std::fs::write(
                            path,
                            "This is a placeholder file, this will get overwritten (hopefully)",
                        ) {
                            println!("Error when creating a decoy: {err}");
                            return;
                        };

                        //check if we are visible
                        if !ui.is_visible() {
                            return;
                        }

                        //We dont have file on our local system so we have to ask the server to provide it
                        let passw = self.client_ui.client_password.clone();
                        let author = self.login_username.clone();
                        let send_on_ip = self.client_ui.send_on_ip.clone();
                        let sender = self.audio_save_tx.clone();

                        let message = ClientMessage::construct_audio_request_msg(
                            audio.index,
                            passw,
                            author,
                            send_on_ip,
                        );

                        tokio::spawn(async move {
                            match client::send_msg(message).await {
                                Ok(ok) => {
                                    match sender.send(ok) {
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
            });
        }
    }
}
