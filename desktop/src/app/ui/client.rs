use egui::{
    load::LoadError, vec2, Align, Align2, Area, Color32, FontFamily, FontId, Id, Image,
    ImageButton, Layout, Pos2, RichText, Sense, Stroke,
};
use rodio::Decoder;
use std::{net::SocketAddr, sync::atomic::Ordering::Relaxed};
use tokio_util::sync::CancellationToken;

use crate::app::backend::{display_error_message, ClientMessage, ConnectionState, Voip};

use crate::app::backend::{Application, SearchType, ServerMessageType};

impl Application
{
    pub fn state_client(&mut self, _frame: &mut eframe::Frame, ctx: &egui::Context)
    {
        egui::TopBottomPanel::new(egui::panel::TopBottomSide::Top, "menu_area").show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.allocate_ui(vec2(300., 40.), |ui| {
                    if ui
                        .add(egui::widgets::ImageButton::new(egui::include_image!(
                            "../../../../assets/icons/logout.png"
                        )))
                        .clicked()
                    {
                        if self.server_has_started {
                            //Avoid panicking when trying to display a Notification
                            //This is very rare but can still happen
                            display_error_message("Server is running!", self.toasts.clone());
                        }
                        else {
                            self.autosync_shutdown_token.cancel();
                            self.server_sender_thread = None;

                            self.main.client_mode = false;
                        }
                    };
                })
                .response
                .on_hover_text("Logout");

                ui.allocate_ui(vec2(300., 40.), |ui| {
                    if ui
                        .add(egui::widgets::ImageButton::new(egui::include_image!(
                            "../../../../assets/icons/settings.png"
                        )))
                        .clicked()
                    {
                        self.settings_window = !self.settings_window;
                    };
                });
                ui.allocate_ui(vec2(300., 40.), |ui| {
                    if ui
                        .add(egui::widgets::ImageButton::new(egui::include_image!(
                            "../../../../assets/icons/search.png"
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

                ui.allocate_ui(vec2(10., 40.), |ui| {
                    ui.separator();
                });

                if matches!(self.client_connection.state, ConnectionState::Connected(_)) {
                    let port = self
                        .client_ui
                        .send_on_ip
                        .split(":")
                        .last()
                        .unwrap_or_default()
                        .to_string();

                    //Check for invalid port
                    if port.is_empty() {
                        //Avoid panicking when trying to display a Notification
                        //This is very rare but can still happen
                        display_error_message(
                            "Invalid address to send the message on.",
                            self.toasts.clone(),
                        );

                        return;
                    }

                    ui.allocate_ui(vec2(40., 40.), |ui| {
                        if let Some(voip) = self.client_ui.voip.clone() {
                            let disconnect_button = ui.add(ImageButton::new(Image::new(
                                egui::include_image!("../../../../assets/icons/call_red.png"),
                            )));

                            if disconnect_button.clicked() {
                                //Disable camera if it exists before everything else
                                if voip.camera_handle_is_open.load(Relaxed) {
                                    self.disable_camera(voip);
                                }

                                if let Some(voip_connected_clients) = &self
                                    .client_ui
                                    .incoming_messages
                                    .ongoing_voip_call
                                    .connected_clients
                                {
                                    voip_connected_clients.iter().for_each(|uuid| {
                                        ctx.forget_image(&format!("bytes://video_stream:{uuid}"));
                                    });
                                }

                                //Shut down listener server, and disconnect from server
                                self.send_msg(ClientMessage::construct_voip_disconnect(
                                    &self.opened_user_information.uuid,
                                ));

                                //Shutdown listener and recorder thread
                                self.voip_shutdown_token.cancel();

                                //Signal the voice recorder function to stop
                                let _ = self.record_audio_interrupter.send(());
                                //Reset state
                                self.client_ui.voip = None;
                                self.voip_thread = None;
                            }
                        }
                        else {
                            ui.add_enabled_ui(self.atx.is_none(), |ui| {
                                let call_button = ui.add(ImageButton::new(Image::new(
                                    egui::include_image!("../../../../assets/icons/call.png"),
                                )));

                                if call_button.clicked() {
                                    //Move sender into thread
                                    let sender = self.voip_connection_sender.clone();

                                    //Reset shutdown token State, if we had cancelled this token we must create a new one in order to reset its state
                                    //else its going to be cancelled and new threads will shut dwon immediately
                                    self.voip_shutdown_token = CancellationToken::new();

                                    let toasts = self.toasts.clone();

                                    match self.client_ui.send_on_ip.parse::<SocketAddr>() {
                                        Ok(socket_addr) => {
                                            //Spawn thread which will create the ```Voip``` instance
                                            tokio::spawn(async move {
                                                match socket_addr.is_ipv6() {
                                                    true => {
                                                        match Voip::new("[::]:0".to_string()).await {
                                                            Ok(voip) => {
                                                                // It is okay to unwrap since it doesnt matter if we panic
                                                                sender.send(voip).unwrap();
                                                            },
                                                            Err(err_ipv6) => {
                                                                display_error_message(format!("Could not bind to IPv6: {err_ipv6}"), toasts);
                                                            },
                                                        }
                                                    },
                                                    false => {
                                                        match Voip::new("0.0.0.0:0".to_string()).await {
                                                            Ok(voip) => {
                                                                sender.send(voip).unwrap();
                                                            },
                                                            Err(err_ipv4) => {
                                                                display_error_message(format!("Could not bind to IPv4: {err_ipv4}"), toasts);
                                                            },
                                                        }
                                                    },
                                                }
                                            });
                                        },
                                        Err(_err) => {
                                            display_error_message("Invalid address.", self.toasts.clone());
                                        },
                                    };

                                    //Lua callback
                                    self.client_ui.extension.event_call_extensions(
                                        crate::app::lua::EventCall::OnCallSend,
                                        &self.lua,
                                        None,
                                    );
                                }

                                call_button.on_hover_text("Start a group call");

                                //Callback
                                self.client_ui.extension.event_call_extensions(
                                    crate::app::lua::EventCall::OnCallSend,
                                    &self.lua,
                                    None,
                                );
                            });
                        }
                    });
                }
            });

            ui.allocate_space(vec2(ui.available_width(), 5.));
        });

        //IF there is an existing Voice call we can assume there are people connected to it
        if let Some(connected_clients) = self
            .client_ui
            .incoming_messages
            .ongoing_voip_call
            .clone()
            .connected_clients
        {
            egui::TopBottomPanel::new(egui::panel::TopBottomSide::Top, "voip_connected_users")
                .resizable(true)
                .show(ctx, |ui| {
                    let uuid = self.opened_user_information.uuid.clone();

                    //We should only display the settings menu if we are connected to a Voip call
                    if let Some(voip) = self.client_ui.voip.clone() {
                        //Settings for the client connected to an ongoing call
                        ui.allocate_ui(vec2(ui.available_width(), 30.), |ui| {
                            ui.horizontal_centered(|ui| {
                                if voip.enable_microphone.load(Relaxed) {
                                    if ui.add(ImageButton::new(egui::include_image!(
                                        "../../../../assets/icons/record.png"
                                    ))).clicked() {
                                        voip.enable_microphone.store(false, Relaxed);
                                    }
                                }
                                else if ui.add(ImageButton::new(egui::include_image!(
                                    "../../../../assets/icons/record_off.png"
                                ))).clicked() {
                                    voip.enable_microphone.store(true, Relaxed);
                                }

                                //If there isnt a camera added
                                if !voip.camera_handle_is_open.load(Relaxed) {
                                    //Display camera on button
                                    if ui
                                        .add(ImageButton::new(egui::include_image!(
                                            "../../../../assets/icons/camera.png"
                                        )))
                                        .clicked()
                                    {
                                        //Reset cancellation token
                                        self.voip_video_shutdown_token = CancellationToken::new();

                                        //Reset thread
                                        self.voip_video_thread = None;

                                        //Add camera handle to the voip
                                        match voip.add_camera_handle() {
                                            Ok(_) => (),
                                            Err(err) => {
                                                tracing::error!("{err}");

                                                display_error_message(err, self.toasts.clone());
                                            },
                                        };

                                        voip.camera_handle_is_open.store(true, Relaxed);

                                        //Send image connection message
                                        self.send_msg(ClientMessage::construct_voip_event(uuid.clone(), crate::app::backend::ClientVoipRequest::ImageConnected));
                                    }
                                }
                                else {
                                    //Display camera off button
                                    if ui
                                        .add(ImageButton::new(egui::include_image!(
                                            "../../../../assets/icons/camera_off.png"
                                        )))
                                        .clicked()
                                    {
                                        self.disable_camera(voip);
                                    }
                                }
                            });
                        });

                        ui.separator();
                    }

                    //Display the name of this part of the ui
                    ui.label(
                        RichText::from("Users connected to the voice chat:")
                            .weak()
                            .size(self.font_size / 2.),
                    );

                    //Put all of the connected users nxt to eachother
                    ui.horizontal(|ui| {
                        for connected_client_uuid in connected_clients.iter() {
                            ui.horizontal(|ui| {
                                self.display_icon_from_server(
                                    ctx,
                                    connected_client_uuid.clone(),
                                    ui,
                                );
                                ui.vertical(|ui| {
                                    //Display username
                                    match self
                                    .client_ui
                                    .incoming_messages
                                    .connected_clients_profile
                                    .get(connected_client_uuid)
                                    {
                                        Some(profile) => {
                                            ui.label(RichText::from(&profile.username).weak());
                                        },
                                        None => {
                                            self.request_client(connected_client_uuid.to_string());

                                            ui.label(RichText::from(format!(
                                                "Profile not found for: {connected_client_uuid}"
                                            )));
                                        },
                                    }

                                    //Display image
                                    match ctx.try_load_bytes(&format!("bytes://video_stream:{connected_client_uuid}")) {
                                        Ok(bytes_poll) => {
                                            match bytes_poll {
                                                egui::load::BytesPoll::Pending { .. } => {
                                                    ui.spinner();
                                                },
                                                egui::load::BytesPoll::Ready { .. } => {
                                                    ui.allocate_ui(vec2(360., 360.), |ui| {
                                                        ui.add(
                                                            Image::from_uri(format!("bytes://video_stream:{connected_client_uuid}"))
                                                        );
                                                    });
                                                },
                                            }
                                        },
                                        Err(err) => {
                                            if let LoadError::Loading(inner) = err {
                                                if inner != "Bytes not found. Did you forget to call Context::include_bytes?" {
                                                    tracing::error!("{}", inner);
                                                }
                                            }
                                            else {
                                                tracing::error!("{}", err);
                                            }
                                        }
                                    }
                                });
                            });
                        }
                    });
                });
        }

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
                        for (index, message) in self.client_ui.incoming_messages.message_list.iter().enumerate() {
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
                    true => vec2(-45.0, -(panel_height.clamp(58., f32::MAX) + 5.)),
                    false => vec2(-45.0, -45.),
                },
            )
            .show(ctx, |ui| {
                ui.allocate_ui(vec2(25., 25.), |ui| {
                    if ui
                        .add(egui::ImageButton::new(egui::include_image!(
                            "../../../../assets/icons/cross.png"
                        )))
                        .clicked()
                    {
                        self.client_ui.usr_msg_expanded = !self.client_ui.usr_msg_expanded;
                    };
                });
            });

        //Server reciver
        self.client_recv(ctx);

        //Client voip thread managemant
        self.client_voip_thread(ctx);

        match self.audio_bytes_rx.try_recv() {
            Ok(bytes) => {
                //Send audio file
                self.send_msg(ClientMessage::construct_file_msg_from_bytes(
                    bytes,
                    "wav".to_string(),
                    self.client_ui.messaging_mode.get_reply_index(),
                    self.opened_user_information.uuid.clone(),
                ));
            },
            Err(_err) => {
                // dbg!(_err);
            },
        }

        match self.audio_save_rx.try_recv() {
            Ok((sink, cursor, index, path_to_audio)) => {
                //Convert idx to usize
                let index = index as usize;

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
                    },
                    Err(err) => {
                        //Avoid panicking when trying to display a Notification
                        //This is very rare but can still happen
                        display_error_message(err, self.toasts.clone());
                    },
                }

                self.client_ui.audio_playback.settings_list[index].cursor = cursor;
                //Reset button state so it can be used again
                self.client_ui.audio_playback.settings_list[index].is_loading = false;
            },
            Err(_err) => {},
        }
    }

    fn disable_camera(&mut self, voip: Voip)
    {
        let uuid = self.opened_user_information.uuid.clone();
        //Drop camera handle
        voip.remove_camera_handle(&self.client_connection.client_secret, uuid.clone());

        voip.camera_handle_is_open.store(false, Relaxed);

        //Cancel webcam recording
        self.voip_video_shutdown_token.cancel();

        //Send image disconnection message
        self.send_msg(ClientMessage::construct_voip_event(
            uuid,
            crate::app::backend::ClientVoipRequest::ImageDisconnected,
        ));
    }
}
