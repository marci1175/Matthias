use egui::{vec2, Align, Color32, Layout, Response, RichText};

//use crate::app::account_manager::write_file;
use crate::app::{
    backend::{AudioSettings, ClientMessage, ScrollToMessage, ServerMessageType, TemplateApp},
    client,
};

impl TemplateApp {
    pub fn client_ui_message_main(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
    ) -> egui::InnerResponse<()> {
        ui.allocate_ui(vec2(ui.available_width(), ui.available_height() - self.client_ui.scroll_widget_rect.height() + 10.), |ui|{
            egui::ScrollArea::vertical()
                    .id_source("msg_area")
                    .stick_to_bottom(self.client_ui.scroll_to_message.is_none())
                    .auto_shrink([false, true])
                    .show(ui, |ui| {

                        //Scroll to reply logic
                        if let Some(scroll_to_instance) = &self.client_ui.scroll_to_message {
                            scroll_to_instance.messages[scroll_to_instance.index].scroll_to_me(Some(Align::Center));
                            //Destroy instance
                            self.client_ui.scroll_to_message = None;
                            self.client_ui.scroll_to_message_index = None;
                        }

                        ui.allocate_ui(ui.available_size(), |ui| {

                            //Display welcome message if self.send_on_ip is empty
                            if self.client_ui.send_on_ip.is_empty() {
                                ui.with_layout(Layout::centered_and_justified(egui::Direction::TopDown), |ui|{
                                    ui.label(RichText::from("To start chatting go to settings and set the IP to the server you want to connect to!").size(self.font_size).color(Color32::LIGHT_BLUE));
                                });
                            }

                            //Check if sink_list is bigger than messages, to avoid crashing
                            if self.client_ui.audio_playback.sink_list.len() > self.client_ui.incoming_msg.struct_list.len() {
                                for _ in 0..(self.client_ui.audio_playback.sink_list.len() as i32 - self.client_ui.incoming_msg.struct_list.len() as i32).abs() {
                                    self.client_ui.audio_playback.sink_list.remove(self.client_ui.audio_playback.sink_list.len() - 1);
                                }
                            }

                            //Allocate places manually for the audio playback (sink_list), but only allocate what we need
                            for _ in 0..(self.client_ui.incoming_msg.struct_list.len() - self.client_ui.audio_playback.sink_list.len()) {
                                self.client_ui.audio_playback.sink_list.push(None);

                                //Define defaults, for speed and volume based on the same logic as above ^
                                self.client_ui.audio_playback.settings_list.push(AudioSettings::default());
                            }
                            let mut message_instances: Vec<Response> = Vec::new();

                            for (index, item) in self.client_ui.incoming_msg.clone().struct_list.iter().enumerate() {
                                let mut i: &String = &Default::default();

                                if let ServerMessageType::Normal(item) = &item.MessageType {
                                    i = &item.message;
                                }

                                //Emoji tray pops up when right clicking on a message
                                let message_group = ui.group(|ui|
                                    {
                                        if let Some(replied_to) = item.replying_to {
                                            ui.allocate_ui(vec2(ui.available_width(), self.font_size), |ui|{
                                                if ui.add(egui::widgets::Button::new(RichText::from(format!("{}: {}",
                                                self.client_ui.incoming_msg.struct_list[replied_to].Author,
                                                match &self.client_ui.incoming_msg.struct_list[replied_to].MessageType {
                                                    ServerMessageType::Audio(audio) => format!("Sound {}", audio.file_name),
                                                    ServerMessageType::Image(_img) => "Image".to_string(),
                                                    ServerMessageType::Upload(upload) => format!("Upload {}", upload.file_name),
                                                    ServerMessageType::Normal(msg) => {
                                                        let mut message_clone = msg.message.clone();
                                                        if message_clone.clone().len() > 20 {
                                                            message_clone.truncate(20);
                                                            message_clone.push_str(" ...");
                                                        }
                                                        message_clone.to_string()
                                                },
                                            })
                                            ).size(self.font_size / 1.5))
                                                .frame(false))
                                                    .clicked() {
                                                        //implement scrolling to message
                                                        self.client_ui.scroll_to_message_index = Some(replied_to);
                                                    }
                                            });
                                        }
                                        //Display author
                                        ui.label(RichText::from(item.Author.to_string()).size(self.font_size / 1.3).color(Color32::WHITE));

                                        //IMPORTANT: Each of these functions have logic inside them for displaying
                                        self.markdown_text_display(i, ui);
                                        self.audio_message_instance(item, ui, index);
                                        self.file_message_instance(item, ui);
                                        self.image_message_instance(item, ui, ctx);

                                        //Display Message date
                                        ui.label(RichText::from(item.MessageDate.to_string()).size(self.font_size / 1.5).color(Color32::DARK_GRAY));
                                        egui::ScrollArea::horizontal().id_source(/* Autoassign id's to interated scroll widgets */ ui.next_auto_id()).max_height(self.font_size).show(ui, |ui|{
                                            ui.horizontal(|ui| {
                                                for item in &item.reactions.message_reactions {
                                                    ui.group(|ui| {
                                                        ui.label(RichText::from(item.char.to_string()).size(self.font_size / 1.1))
                                                    });
                                                    ui.label(RichText::from(item.times.to_string()).size(self.font_size / 1.3));
                                                }
                                            });
                                        });
                                    }
                                    ).response.context_menu(|ui|{
                                        ui.menu_button("React", |ui| {
                                            let filter = &self.filter;
                                            let named_chars = self.named_chars
                                                .entry(egui::FontFamily::Monospace)
                                                .or_insert_with(|| TemplateApp::available_characters(ui, egui::FontFamily::Monospace));

                                            ui.allocate_ui(vec2(300., 300.), |ui|{
                                                egui::ScrollArea::vertical().show(ui, |ui| {
                                                    ui.horizontal_wrapped(|ui| {
                                                        ui.spacing_mut().item_spacing = egui::Vec2::splat(2.0);

                                                        for (&chr, name) in named_chars {
                                                            if filter.is_empty()
                                                                || name.contains(filter)
                                                                || *filter == chr.to_string()
                                                            {
                                                                let button = egui::Button::new(
                                                                    egui::RichText::new(chr.to_string()).font(egui::FontId {
                                                                        size: self.font_size,
                                                                        family: egui::FontFamily::Proportional,
                                                                    }),
                                                                )
                                                                .frame(false);
                                                                if ui.add(button).clicked() {
                                                                    let message = ClientMessage::construct_reaction_msg(
                                                                        chr, index, &self.login_username, self.client_ui.req_passw.then_some((|| &self.client_ui.client_password)()),
                                                                    );
                                                                    let connection = self.client_connection.clone();

                                                                    tokio::spawn(async move {
                                                                        match client::send_msg(connection, message).await {
                                                                            Ok(_) => {},
                                                                            Err(err) => println!("{err}"),
                                                                        };
                                                                    });
                                                                }
                                                            }
                                                        }
                                                    });
                                                });
                                            });
                                        });

                                        if ui.button("Reply").clicked() {
                                            self.client_ui.replying_to = Some(index);
                                        }
                                        if ui.button("Copy text").clicked() {
                                            ctx.copy_text(i.clone());
                                        };
                                });
                                if let Some(message_group) = message_group {
                                    message_instances.push(message_group.response);
                                }

                            };
                            if let Some(scroll_to_reply) = self.client_ui.scroll_to_message_index {
                                self.client_ui.scroll_to_message = Some(ScrollToMessage::new(message_instances, scroll_to_reply));
                            }
                        });
                        if self.client_ui.usr_msg_expanded {
                            ui.allocate_space(vec2(ui.available_width(), 25.));
                        }
                    });
        })
    }
}
