use crate::app::backend::{
    AudioSettings, ClientMessage, ScrollToMessage, ServerMessageType, TemplateApp,
};
use egui::{vec2, Align, Color32, Layout, Response, RichText};

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
                            let message_instances: Vec<Response> = Vec::new();

                            for (iter_index, item) in self.client_ui.incoming_msg.clone().struct_list.iter().enumerate() {
                                //Emoji tray pops up when right clicking on a message
                                let message_group = ui.group(|ui| {
                                        if let Some(replied_to) = item.replying_to {
                                            ui.allocate_ui(vec2(ui.available_width(), self.font_size), |ui|{
                                                if ui.add(egui::widgets::Button::new(RichText::from(format!("{}: {}",
                                                self.client_ui.incoming_msg.struct_list[replied_to].Author,
                                                match &self.client_ui.incoming_msg.struct_list[replied_to].MessageType {
                                                    ServerMessageType::Deleted => "Deleted message".to_string(),
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
                                                    //These message enums (described below), are supposed to have a side effect on messages which we are already storing
                                                    //ServerMessageType::Deleted gets displayed thats why that not here
                                                    //ServerMessageType::Edit(_)
                                                    //ServerMessageType::Reaction(())
                                                    _ => { unreachable!() }
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
                                        self.message_display(item, ui, ctx, iter_index);

                                        //Display Message date
                                        ui.label(RichText::from(item.MessageDate.to_string()).size(self.font_size / 1.5).color(Color32::DARK_GRAY));

                                        if let ServerMessageType::Normal(inner_msg) = &item.MessageType {
                                            if inner_msg.has_been_edited {
                                                ui.label(RichText::from("(Edited)").strong());
                                            }
                                        }

                                        egui::ScrollArea::horizontal().id_source(/* Autoassign id's to interated scroll widgets */ ui.next_auto_id()).max_height(self.font_size).show(ui, |ui|{
                                            ui.horizontal(|ui| {

                                                //Check if there is a reaction list vector already allocated non the index of the specific message
                                                match self.client_ui.incoming_msg.reaction_list.get(iter_index) {
                                                    Some(reactions) => {
                                                        for item in &reactions.message_reactions {
                                                            ui.group(|ui| {
                                                                ui.label(RichText::from(item.char.to_string()).size(self.font_size / 1.1))
                                                            });
                                                            ui.label(RichText::from(item.times.to_string()).size(self.font_size / 1.3));
                                                        }
                                                    },
                                                    None => {
                                                        eprintln!("message_main.rs: No reaction list allocated for message {}", iter_index);
                                                    },
                                                }

                                            });
                                        });

                                        if ui.is_rect_visible(ui.min_rect()) && *self.client_ui.last_seen_msg_index.lock().unwrap() < iter_index {
                                            *self.client_ui.last_seen_msg_index.lock().unwrap() = iter_index;
                                        }
                                    }
                                );

                                //Display where the users seen their last message
                                ui.horizontal(|ui| {
                                    for client in &self.client_ui.seen_list {
                                        if iter_index == client.index {
                                            //Make it more visible
                                            ui.group(|ui| {
                                                ui.label(RichText::from(&client.username));
                                            });
                                        }
                                    }
                                });

                                message_group.response.context_menu(|ui|{
                                    //Client-side uuid check, there is a check in the server file
                                    if item.uuid == self.opened_account.uuid && item.MessageType != ServerMessageType::Deleted {
                                        ui.horizontal(|ui| {

                                            //We should only thisplay the text edit widget if its on an editable message
                                            if matches!(item.MessageType, ServerMessageType::Normal(_)) {
                                                ui.text_edit_multiline(&mut self.client_ui.text_edit_buffer);
                                            }

                                            ui.vertical(|ui| {
                                                ui.allocate_ui(vec2(100., 10.), |ui| {
                                                    //We should only display the `edit` button if its anormal message thus its editable
                                                    if matches!(item.MessageType, ServerMessageType::Normal(_)) && ui.button("Edit").clicked() {
                                                        self.send_msg(
                                                            ClientMessage::construct_client_message_edit(iter_index, Some(self.client_ui.text_edit_buffer.clone()), &self.opened_account.uuid, &self.opened_account.username)
                                                        );
                                                        self.client_ui.text_edit_buffer.clear();
                                                        ui.close_menu();
                                                    }

                                                    if ui.button("Delete").clicked() {
                                                        self.send_msg(
                                                            ClientMessage::construct_client_message_edit(iter_index, None, &self.opened_account.uuid, &self.opened_account.username)
                                                        );
                                                        self.client_ui.text_edit_buffer.clear();
                                                        ui.close_menu();
                                                    }
                                                });
                                            });
                                        });

                                        ui.separator();
                                    }

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

                                                                let uuid = self.opened_account.uuid.clone();

                                                                let message = ClientMessage::construct_reaction_msg(
                                                                    chr, iter_index, &self.login_username, &uuid,
                                                                );
                                                                let connection = self.client_connection.clone();

                                                                tokio::spawn(async move {
                                                                    match connection.send_message(message).await {
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
                                        self.client_ui.replying_to = Some(iter_index);
                                    }
                                    if let ServerMessageType::Normal(inner) = &item.MessageType {
                                        if ui.button("Copy text").clicked() {
                                            ctx.copy_text(inner.message.clone());
                                        };
                                    }
                                });

                                // message_group.response.paint_debug_info();
                                // message_instances.push(message_group.response);
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
