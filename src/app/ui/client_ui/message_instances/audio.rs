use base64::engine::general_purpose;
use base64::Engine;

use egui::{vec2, Align, Layout};

use rodio::{Decoder, Sink};

use std::io::BufReader;

use std::{fs::File, path::PathBuf};

//use crate::app::account_manager::write_file;
use crate::app::backend::{ClientMessage, ServerMessageType, TemplateApp};
use std::fs;
use crate::app::client::{self};
impl TemplateApp {
    pub fn audio_message_instance(
        &mut self,
        item: &crate::app::backend::ServerOutput,
        ui: &mut egui::Ui,
    ) {
        //Create folder for audios for later problem avoidance
        let _ = fs::create_dir_all(PathBuf::from(format!("{}{}{}{}",env!("APPDATA"), "\\szeChat\\Client\\", self.send_on_ip_base64_encoded, "\\Audios")));

        if let ServerMessageType::Audio(audio) = &item.MessageType {
            ui.allocate_ui(vec2(300., 150.), |ui| {
                let path = PathBuf::from(format!(
                    "{}\\szeChat\\Client\\{}\\Audios\\{}",
                    env!("APPDATA"),
                    self.send_on_ip_base64_encoded,
                    audio.index
                ));

                match path.exists()
                {
                    true => {
                        //if we already have the sound file :::

                        ui.with_layout(Layout::top_down(Align::Center), |ui| {
                            match self.audio_playback.sink.as_mut() {
                                Some(sink) => match sink.is_paused() {
                                    true => {
                                        if ui.button("Play").clicked() {
                                            sink.play();
                                        };
                                    }
                                    false => {
                                        if ui.button("Stop").clicked() {
                                            sink.pause()
                                        }
                                    }
                                },
                                None => {
                                    if ui.button("Play").clicked() {
                                        let file = BufReader::new(
                                            File::open(PathBuf::from(format!(
                                                "{}\\szeChat\\Client\\{}\\Audios\\{}",
                                                env!("APPDATA"),
                                                general_purpose::URL_SAFE_NO_PAD
                                                    .encode(self.send_on_ip.clone()),
                                                audio.index
                                            )))
                                            .unwrap(),
                                        );

                                        let source = Decoder::new(file).unwrap();

                                        self.audio_playback.sink = Some(
                                            Sink::try_new(&self.audio_playback.stream_handle)
                                                .unwrap(),
                                        );

                                        let sink = self.audio_playback.sink.as_mut().unwrap();

                                        sink.append(source);

                                        sink.play();
                                    };
                                }
                            }
                        });

                        //Set properties of audio stream, when there is one
                        if let Some(sink) = self.audio_playback.sink.as_mut() {
                            //Set volume
                            sink.set_volume(self.audio_playback.volume);
                        
                            sink.set_speed(self.audio_playback.speed);
                        }

                        ui.label(&audio.file_name);
                        //Audio volume
                        ui.label("Volume");
                        ui.add(egui::Slider::new(&mut self.audio_playback.volume, 0.1..=10.).step_by(0.1));
                        //Audio speed
                        ui.label("Speed");
                        ui.add(egui::Slider::new(&mut self.audio_playback.speed, 0.1..=10.).step_by(0.1));
                    }
                    false => {
                        //create decoy file, to manually create a race condition
                        if let Err(err) = std::fs::write(path, "This is a placeholder file, this will get overwritten (hopefully)"){
                            println!("Error when creating a decoy: {err}");
                            return;
                        };

                        //check if we are visible
                        if !ui.is_visible() {
                            return;
                        }

                        //We dont have file on our local system so we have to ask the server to provide it
                        let passw = self.client_password.clone();
                        let author = self.login_username.clone();
                        let send_on_ip = self.send_on_ip.clone();
                        let sender = self.audio_save_tx.clone();

                        let message = ClientMessage::construct_audio_request_msg(
                            audio.index,
                            passw,
                            author,
                            send_on_ip,
                        );

                        self.requests.audio = tokio::spawn(async move {
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
                        })
                        .is_finished();
                    }
                };
            });
        }
    }
}
