use egui::{vec2, Align, Color32, Layout, Response, RichText};

//use crate::app::account_manager::write_file;
use crate::app::backend::{AudioSettings, ServerMessageType, TemplateApp, ScrollToMessage};

impl TemplateApp {
    pub fn client_ui_message_main(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
    ) -> egui::InnerResponse<()> {
        ui.allocate_ui(vec2(ui.available_width(), ui.available_height() - self.scroll_widget_rect.height() + 10.), |ui|{
            egui::ScrollArea::vertical()
                    .id_source("msg_area")
                    .stick_to_bottom(self.scroll_to_message.is_none())
                    .auto_shrink([false, true])
                    .show(ui, |ui| {

                        //Scroll to reply logic
                        if let Some(scroll_to_instance) = &self.scroll_to_message {
                            scroll_to_instance.messages[scroll_to_instance.index].scroll_to_me(Some(Align::Center));
                            
                            //Destroy instance
                            self.scroll_to_message = None;
                            self.scroll_to_message_index = None;
                        }

                        ui.allocate_ui(ui.available_size(), |ui| {

                            //Display welcome message if self.send_on_ip is empty
                            if self.send_on_ip.is_empty() {
                                ui.with_layout(Layout::centered_and_justified(egui::Direction::TopDown), |ui|{
                                    ui.label(RichText::from("To start chatting go to settings and set the IP to the server you want to connect to!").size(self.font_size).color(Color32::LIGHT_BLUE));
                                });
                            }

                            //Check if sink_list is bigger than messages, to avoid crashing
                            if self.audio_playback.sink_list.len() > self.incoming_msg.struct_list.len() {
                                for _ in 0..(self.audio_playback.sink_list.len() as i32 - self.incoming_msg.struct_list.len() as i32).abs() {
                                    self.audio_playback.sink_list.remove(self.audio_playback.sink_list.len() - 1);
                                }
                            }

                            //Allocate places manually for the audio playback (sink_list), but only allocate what we need
                            for _ in 0..(self.incoming_msg.struct_list.len() - self.audio_playback.sink_list.len()) {
                                self.audio_playback.sink_list.push(None);

                                //Define defaults, for speed and volume based on the same logic as above ^
                                self.audio_playback.settings_list.push(AudioSettings::default());
                            }
                            
                            let mut message_instances: Vec<Response> = Vec::new();

                            for (index, item) in self.incoming_msg.clone().struct_list.iter().enumerate() {
                        
                                let mut i: &String = &Default::default();

                                if let ServerMessageType::Normal(item) = &item.MessageType {
                                    i = &item.message;
                                }

                                let message_group = ui.group(|ui|
                                {
                                    if let Some(replied_to) = item.replying_to {
                                        ui.allocate_ui(vec2(ui.available_width(), self.font_size), |ui|{
                                            if ui.add(egui::widgets::Button::new(RichText::from(format!("Replying to: {}: {}",
                                            self.incoming_msg.struct_list[replied_to].Author,
                                            match &self.incoming_msg.struct_list[replied_to].MessageType {
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
                                                    self.scroll_to_message_index = Some(replied_to);
                                            
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
                                }
                                ).response.context_menu(|ui|{
                                    if ui.button("Reply").clicked() {
                                        self.replying_to = Some(index);
                                    }
                                    if ui.button("Copy text").clicked() {
                                        ctx.copy_text(i.clone());
                                    };
                                });
                                

                                //this functions for the reply autoscroll
                                message_instances.push(message_group);
                                

                            };
                            if let Some(scroll_to_reply) = self.scroll_to_message_index {
                                self.scroll_to_message = Some(ScrollToMessage::new(message_instances, scroll_to_reply));
                            }
                        });
                
                        if !self.usr_msg_expanded {
                            ui.allocate_space(vec2(ui.available_width(), 25.));
                        }
                    });
        })
    }
}
