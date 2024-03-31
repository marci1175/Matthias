use egui::{vec2, Align, Layout};
use rodio::{Decoder, Sink};
use std::path::PathBuf;
use tap::{Tap, TapFallible};

//use crate::app::account_manager::write_file;
use crate::app::backend::{
    display_error_message, write_audio, ClientMessage, PlaybackCursor, ServerAudioReply, ServerMessageType, TemplateApp
};
use crate::app::client::{self};
use std::fs;
impl TemplateApp {
    pub fn audio_message_instance(
        &mut self,
        item: &crate::app::backend::ServerOutput,
        ui: &mut egui::Ui,
        current_index_in_message_list: usize,
    ) {
        if let ServerMessageType::Audio(audio) = &item.MessageType {
            //Create folder for audios for later problem avoidance
            let _ = fs::create_dir_all(PathBuf::from(format!(
                "{}{}{}{}",
                env!("APPDATA"),
                "\\Matthias\\Client\\",
                self.client_ui.send_on_ip_base64_encoded,
                "\\Audios"
            )));

            ui.allocate_ui(vec2(300., 150.), |ui| {
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
                                    let file = PathBuf::from(format!(
                                        "{}\\Matthias\\Client\\{}\\Audios\\{}",
                                        env!("APPDATA"),
                                        self.client_ui.send_on_ip_base64_encoded,
                                        audio.index
                                    ));

                                    //If the user has clicked the play button only then we download the desirted audio file! Great optimisation
                                    if !file.exists() {
                                        let sender = self.audio_save_tx.clone();

                                        let message = ClientMessage::construct_audio_request_msg(
                                            audio.index,
                                            self.client_ui.client_password.clone(),
                                            self.login_username.clone(),
                                        );

                                        let connection = self.client_connection.clone();
                                        let send_on_ip = self.client_ui.send_on_ip.clone();
                                        let stream_handle =
                                            self.client_ui.audio_playback.stream_handle.clone();
                                        let current_index = current_index_in_message_list;

                                        tokio::spawn(async move {
                                            match client::send_msg(connection, message).await {
                                                Ok(response) => {
                                                    let file_serve: Result<
                                                        ServerAudioReply,
                                                        serde_json::Error,
                                                    > = serde_json::from_str(&response);
                                                    let _ = write_audio(
                                                        file_serve.unwrap(),
                                                        send_on_ip,
                                                    );

                                                    let file_stream_to_be_read =
                                                        fs::read(file).unwrap_or_default();
                                                    let cursor =
                                                        PlaybackCursor::new(file_stream_to_be_read);
                                                    let sink = Some(
                                                        Sink::try_new(&stream_handle).unwrap(),
                                                    );

                                                    let _ = sender
                                                        .send((sink, cursor, current_index))
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
                                                        .send((None, PlaybackCursor::new(Vec::new()), current_index))
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
                            });
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
    }
}
