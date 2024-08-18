use crate::app::{
    backend::{
        Application, AudioSettings, ClientMessage, MessagingMode, ScrollToMessage,
        ServerMessageType,
    },
    server::SERVER_UUID,
    ui::client_ui::widgets::emoji_tray::emoji::display_emoji,
};
use egui::{
    load::{BytesPoll, LoadError},
    vec2, Align, Button, Color32, Image, Layout, Response, RichText, Sense,
};

impl Application
{
    pub fn client_ui_message_main(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
    ) -> egui::InnerResponse<()>
    {
        ui.allocate_ui(vec2(ui.available_width(), ui.available_height()), |ui|{
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
                            if self.client_ui.audio_playback.sink_list.len() > self.client_ui.incoming_messages.message_list.len() {
                                for _ in 0..(self.client_ui.audio_playback.sink_list.len() as i32 - self.client_ui.incoming_messages.message_list.len() as i32).abs() {
                                    self.client_ui.audio_playback.sink_list.remove(self.client_ui.audio_playback.sink_list.len() - 1);
                                }
                            }

                            //Allocate places manually for the audio playback (sink_list), but only allocate what we need
                            for _ in 0..(self.client_ui.incoming_messages.message_list.len() - self.client_ui.audio_playback.sink_list.len()) {
                                self.client_ui.audio_playback.sink_list.push(None);

                                //Define defaults, for speed and volume based on the same logic as above ^
                                self.client_ui.audio_playback.settings_list.push(AudioSettings::default());
                            }

                            let mut message_instances: Vec<Response> = Vec::new();

                            for (iter_index, item) in self.client_ui.incoming_messages.clone().message_list.iter().enumerate() {
                                self.draw_message(ui, item, ctx, iter_index, &mut message_instances);
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

    /// This function draws the message at the ui passted in as an argument.
    /// The message instance created by this function has a context_menu called upon it.
    /// This function handles all the message types because this function is to display the whole message.
    fn draw_message(
        &mut self,
        ui: &mut egui::Ui,
        item: &crate::app::backend::ServerOutput,
        ctx: &egui::Context,
        iter_index: usize,
        message_instances: &mut Vec<Response>,
    )
    {
        //Emoji tray pops up when right clicking on a message
        let message_group = ui.group(|ui| {
            if let Some(replied_to) = item.replying_to {
                ui.allocate_ui(vec2(ui.available_width(), self.font_size), |ui| {
                    ui.horizontal(|ui| {
                        self.display_icon_from_server(
                            ctx,
                            self.client_ui.incoming_messages.message_list[replied_to]
                                .uuid
                                .clone(),
                            ui,
                        );

                        if ui
                            .add(
                                egui::widgets::Button::new(
                                    RichText::from(format!(
                                        "{}: {}",
                                        self.client_ui.incoming_messages.message_list[replied_to]
                                            .author,
                                        match &self.client_ui.incoming_messages.message_list
                                            [replied_to]
                                            .message_type
                                        {
                                            ServerMessageType::Deleted =>
                                                "Deleted message".to_string(),
                                            ServerMessageType::Audio(audio) =>
                                                format!("Sound {}", audio.file_name),
                                            ServerMessageType::Image(_img) => "Image".to_string(),
                                            ServerMessageType::Upload(upload) =>
                                                format!("Upload {}", upload.file_name),
                                            ServerMessageType::Normal(msg) => {
                                                let mut message_clone = msg.message.clone();
                                                if message_clone.clone().len() > 20 {
                                                    message_clone.truncate(20);
                                                    message_clone.push_str(" ...");
                                                }
                                                message_clone.to_string()
                                            },
                                            ServerMessageType::Server(server) => match server {
                                                crate::app::backend::ServerMessage::Connect(
                                                    profile,
                                                ) => {
                                                    format!("{} has connected", profile.username)
                                                },
                                                crate::app::backend::ServerMessage::Disconnect(
                                                    profile,
                                                ) => {
                                                    format!("{} has disconnected", profile.username)
                                                },
                                                crate::app::backend::ServerMessage::Ban(
                                                    profile,
                                                ) => {
                                                    format!(
                                                        "{} has benned banned",
                                                        profile.username
                                                    )
                                                },
                                            },
                                            ServerMessageType::Edit(_) => unreachable!(),
                                            ServerMessageType::Reaction(_) => unreachable!(),
                                            ServerMessageType::Sync(_) => unreachable!(),
                                            ServerMessageType::VoipEvent(_) => unreachable!(),
                                            ServerMessageType::VoipState(_) => unreachable!(),
                                        }
                                    ))
                                    .size(self.font_size / 1.5),
                                )
                                .frame(false),
                            )
                            .clicked()
                        {
                            //implement scrolling to message
                            self.client_ui.scroll_to_message_index = Some(replied_to);
                        }
                    });
                });
            }

            //Display author
            ui.horizontal(|ui| {
                //Profile picture
                self.display_icon_from_server(ctx, item.uuid.clone(), ui);
                //Client name
                ui.label(
                    RichText::from(item.author.to_string())
                        .size(self.font_size / 1.3)
                        .color(Color32::WHITE),
                );
            });

            //IMPORTANT: Each of these functions have logic inside them for displaying
            self.message_display(item, ui, ctx, iter_index);

            //Display Message date
            ui.label(
                RichText::from(item.message_date.to_string())
                    .size(self.font_size / 1.5)
                    .color(Color32::DARK_GRAY),
            );

            if let ServerMessageType::Normal(inner_msg) = &item.message_type {
                if inner_msg.has_been_edited {
                    ui.label(RichText::from("(Edited)").strong());
                }
            }

            egui::ScrollArea::horizontal()
                .id_source(
                    /* Autoassign id's to interated scroll widgets */ ui.next_auto_id(),
                )
                .max_height(self.font_size)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        //Check if there is a reaction list vector already allocated non the index of the specific message
                        match self
                            .client_ui
                            .incoming_messages
                            .reaction_list
                            .clone()
                            .get(iter_index)
                        {
                            Some(reactions) => {
                                for (idx, item) in reactions.message_reactions.iter().enumerate() {
                                    ui.push_id(idx, |ui| {
                                        let group = ui.group(|ui| {
                                            ui.allocate_ui(vec2(20., 20.), |ui| {
                                                display_emoji(ctx, &item.emoji_name, ui);
                                            });

                                            ui.label(
                                                RichText::from(item.authors.len().to_string())
                                                    .size(self.font_size / 1.3),
                                            );
                                        });

                                        let emoji_group_rect = ui.interact(
                                            group.response.rect,
                                            ui.next_auto_id(),
                                            Sense::click(),
                                        );

                                        if emoji_group_rect.clicked() {
                                            self.change_send_emoji(
                                                iter_index,
                                                item.emoji_name.clone(),
                                            );
                                        };
                                    });
                                }
                            },
                            None => {
                                // eprintln!("message_main.rs: No reaction list allocated for message {}", iter_index);
                            },
                        }
                    });
                });

            if ui.is_rect_visible(ui.min_rect())
                && *self.client_ui.last_seen_msg_index.lock().unwrap() < iter_index
            {
                *self.client_ui.last_seen_msg_index.lock().unwrap() = iter_index;
            }
        });

        //Display where the users seen their last message
        ui.horizontal(|ui| {
            for client in self.client_ui.incoming_messages.user_seen_list.clone() {
                if iter_index == client.index {
                    //Make it more visible
                    ui.group(|ui| {
                        //Profile picture
                        ui.allocate_ui(vec2(18., 18.), |ui| {
                            self.display_icon_from_server(ctx, client.uuid.clone(), ui);
                        });

                        //Client name
                        if let Some(profile) = self
                            .client_ui
                            .incoming_messages
                            .connected_clients_profile
                            .get(&client.uuid)
                        {
                            let username = profile.username.clone();

                            ui.label(RichText::from(&username).size(self.font_size / 1.3));
                        }
                        //If the profile was not found then we can ask for it
                        else {
                            ui.spinner();

                            self.request_client(client.uuid.clone());
                        };
                    });
                }
            }
        });

        //Back up reponse of message group, so we can scroll to it later if the user thinks like it
        message_instances.push(message_group.response.clone());

        message_group.response.context_menu(|ui| {
            let profile_menu_button = ui.menu_button("Profile", |ui| {
                //Check if the message was sent by the server, create a custom profile for it
                if item.uuid == SERVER_UUID {
                    //Add verification or somthing like that
                    ui.allocate_ui(vec2(ui.available_width(), 25.), |ui| {
                        ui.horizontal_centered(|ui| {
                            ui.label("This message was sent by the host server");
                            ui.allocate_ui(vec2(25., 25.), |ui| {
                                ui.add(Image::new(egui::include_image!(
                                    "../../../../../../assets/icons/tick.png"
                                )));
                            })
                        });
                    });
                }
                //If the message was sent by a normal user
                else {
                    //We can safely unwrap here
                    let user_profile = self
                        .client_ui
                        .incoming_messages
                        .connected_clients_profile
                        .get(&item.uuid)
                        .unwrap();
                    //Include full profile picture so it can be displayed
                    ctx.include_bytes(
                        "bytes://profile_picture",
                        user_profile.normal_profile_picture.clone(),
                    );

                    //Display 256px profile picture
                    ui.image("bytes://profile_picture");

                    ui.label(
                        RichText::from(user_profile.username.clone())
                            .size(25.)
                            .strong(),
                    );

                    ui.separator();

                    ui.label(format!("Uuid: {}", item.uuid));

                    if !user_profile.full_name.is_empty() {
                        ui.separator();

                        ui.label(format!("Full name: {}", user_profile.full_name));
                    };

                    ui.separator();

                    ui.label(format!("Birtdate: {}", user_profile.birth_date));

                    ui.separator();

                    if let Some(gender) = &user_profile.gender {
                        ui.label(format!(
                            "Gender: {}",
                            match gender {
                                true => "Female",
                                false => "Male",
                            }
                        ));
                    }
                }
            });

            //If profile_menu_button.inner.is_none() it is closed, so we can deallocate / forget the before loaded image
            if profile_menu_button.inner.is_none() {
                ctx.forget_image("bytes://profile_picture");
            }

            ui.separator();

            if ui
                .add(Button::image_and_text(
                    egui::include_image!("../../../../../../assets/icons/reply.png"),
                    "Reply",
                ))
                .clicked()
            {
                self.client_ui.messaging_mode = MessagingMode::Reply(iter_index);
                ui.close_menu();
            }
            ui.separator();

            //Client-side uuid check, there is a check in the server file
            if item.uuid == self.opened_user_information.uuid
                && item.message_type != ServerMessageType::Deleted
            {
                //We should only display the `edit` button if its anormal message thus its editable
                if let ServerMessageType::Normal(inner) = &item.message_type {
                    if ui
                        .add(Button::image_and_text(
                            egui::include_image!("../../../../../../assets/icons/edit.png"),
                            "Edit",
                        ))
                        .clicked()
                    {
                        self.client_ui.messaging_mode = MessagingMode::Edit(iter_index);
                        self.client_ui.message_buffer = inner.message.to_string();
                        ui.close_menu();
                    }
                }

                if ui
                    .add(Button::image_and_text(
                        egui::include_image!("../../../../../../assets/icons/delete.png"),
                        "Delete",
                    ))
                    .clicked()
                {
                    self.send_msg(ClientMessage::construct_client_message_edit(
                        iter_index,
                        None,
                        &self.opened_user_information.uuid,
                    ));
                    ui.close_menu();
                }

                ui.separator();
            }

            ui.menu_button("React", |ui| {
                if let Some(selected_emoji_name) = self.draw_emoji_selector(ui, ctx) {
                    self.change_send_emoji(iter_index, selected_emoji_name);
                }
            });

            if let ServerMessageType::Normal(inner) = &item.message_type {
                if ui
                    .add(Button::image_and_text(
                        egui::include_image!("../../../../../../assets/icons/copy.png"),
                        "Copy message",
                    ))
                    .clicked()
                {
                    ctx.copy_text(inner.message.clone());
                    ui.close_menu();
                };
            }
        });
    }

    /// ```iter_index```: Which message does this emoji change belong to
    /// ```selected_emoji_name```: Which message the user has selected
    /// This function tries to send / delete the emoji based on the passed arg
    /// NOTE: This function will send an emoji deletion message if you have already sent this specific emojio
    fn change_send_emoji(&mut self, iter_index: usize, selected_emoji_name: String)
    {
        //Check if there is an emoji already added
        if let Some(reaction) = self.client_ui.incoming_messages.reaction_list[iter_index]
            .message_reactions
            .iter()
            .find(|reaction| reaction.emoji_name == selected_emoji_name)
        {
            //Check if we have already sent this message, if yes we delete it
            if reaction
                .authors
                .iter()
                .any(|uuid| *uuid == self.opened_user_information.uuid)
            {
                self.send_msg(ClientMessage::construct_reaction_remove_msg(
                    selected_emoji_name,
                    iter_index,
                    &self.opened_user_information.uuid,
                ));
            }
            //If no, we can send it
            else {
                self.send_msg(ClientMessage::construct_reaction_msg(
                    selected_emoji_name,
                    iter_index,
                    &self.opened_user_information.uuid,
                ));
            }
        }
        //If no that means we can add the emoji
        else {
            self.send_msg(ClientMessage::construct_reaction_msg(
                selected_emoji_name,
                iter_index,
                &self.opened_user_information.uuid,
            ));
        }
    }

    pub fn request_client(&mut self, uuid: String)
    {
        //Ask the server for the specified client's profile picture
        self.send_msg(ClientMessage::construct_client_request_msg(
            uuid.clone(),
            &self.opened_user_information.uuid,
        ));
    }

    /// This function displays the 64x64 icon of a client based on their uuid
    /// This function also requests the server for the image if the image isnt available on the given URI
    pub fn display_icon_from_server(&mut self, ctx: &egui::Context, uuid: String, ui: &mut egui::Ui)
    {
        //If uuid is the server's we just include the image of the server
        if uuid == SERVER_UUID {
            ui.add(Image::new(egui::include_image!(
                "../../../../../../assets/icons/server_white64.png"
            )));

            return;
        }
        match ctx.try_load_bytes(&format!("bytes://{}", &uuid)) {
            //If the image was found on the URI
            Ok(bytes) => {
                //We want to wait until all the bytes are ready to display the image
                if let BytesPoll::Ready {
                    bytes,
                    size: _,
                    mime: _,
                } = bytes
                {
                    //If there is only a 0 in the bytes that indicates its a placeholder, thus we can display the spinner
                    if bytes.to_vec() == vec![0] {
                        ui.spinner();
                    }
                    else {
                        ui.add(egui::Image::from_uri(format!("bytes://{}", &uuid)));
                    }
                }
            },
            //If the image was not found on the URI
            Err(err) => {
                ui.spinner();
                if let LoadError::Loading(inner) = err {
                    if inner == "Bytes not found. Did you forget to call Context::include_bytes?" {
                        //check if we are visible, so there are no unnecessary requests
                        if !ui.is_rect_visible(ui.min_rect()) {
                            return;
                        }

                        //Ask the server for the specified client's profile picture
                        self.send_msg(ClientMessage::construct_client_request_msg(
                            uuid.clone(),
                            &self.opened_user_information.uuid,
                        ));

                        //If the server takees a lot of time to respond, we will prevent asking multiple times by creating a placeholder just as in the image displaying code
                        //We will forget this URI when loading in the real image
                        ctx.include_bytes(format!("bytes://{}", &uuid), vec![0]);
                    }
                    else {
                        tracing::error!("{}", inner);
                    }
                }
                else {
                    tracing::error!("{}", err);
                }
            },
        };
    }
}
