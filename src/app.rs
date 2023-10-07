use std::{sync::mpsc , io};
use log::debug;
use windows_sys::w;
use windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW;
use egui::{vec2, Align, Layout, RichText};

mod client;
mod server;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    //server main

    #[serde(skip)]
    server_has_started: bool,
    //server settings

    #[serde(skip)]
    server_req_password: bool,

    server_password: String,

    #[serde(skip)]
    open_on_port: String,

    //thread communication for server
    #[serde(skip)]
    srx : mpsc::Receiver<String>,
    #[serde(skip)]
    stx : mpsc::Sender<String>,

    //child windows
    #[serde(skip)]
    settings_window: bool,

    //main
    #[serde(skip)]
    client_mode: bool,
    #[serde(skip)]
    server_mode: bool,

    //client main
    send_on_ip: String,

    //font
    font_size: f32,

    //msg
    usr_msg: String,
    incoming_msg: String,

    //thread communication for client
    #[serde(skip)]
    rx : mpsc::Receiver<String>,
    #[serde(skip)]
    tx : mpsc::Sender<String>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel::<String>();
        let (stx, srx) = mpsc::channel::<String>();
        Self {
            //server_main
            server_has_started: false,
            //server settings
            server_req_password: false,
            server_password: String::default(),
            open_on_port: String::default(),

            //thread communication for server
            srx,
            stx,

            //child windows
            settings_window: false,

            //main
            client_mode: false,
            server_mode: false,

            //client main
            send_on_ip: String::new(),
            //font
            font_size: 20.,

            //msg
            usr_msg: String::new(),
            incoming_msg: String::new(),
            //thread communication
            rx,
            tx,
        }
    }
}

impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for TemplateApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        /*devlog:
        TODO: MAKE QUALITY BETTER!!!
        TODO: MAKE BASIC SERVER UI, IMPLEMENT BASIC FUNCTIONALITY
        TODO: MAKE CLIENT UI BETTER
        */

        //window options

        //For image loading
        egui_extras::install_image_loaders(ctx);

        //Main page
        if !(self.client_mode || self.server_mode) {
            //main

            //window settings
            _frame.set_window_size(vec2(700., 300.));

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.columns(2, |ui| {
                    ui[0].with_layout(
                        Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            if ui
                                .add(egui::widgets::ImageButton::new(egui::include_image!(
                                    "../icons/client.png"
                                )))
                                .on_hover_text("Enter Client mode")
                                .clicked()
                            {
                                self.client_mode = true;
                                _frame.set_window_size(vec2(1300., 800.));
                            };
                        },
                    );

                    ui[1].with_layout(
                        Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            if ui
                                .add(egui::widgets::ImageButton::new(egui::include_image!(
                                    "../icons/server.png"
                                )))
                                .on_hover_text("Enter Server mode")
                                .clicked()
                            {
                                self.server_mode = true;
                                _frame.set_window_size(vec2(1000., 900.));
                            };
                        },
                    );
                });
            });
        }

        //Server page
        if self.server_mode {
            //settings
            egui::TopBottomPanel::top("srvr_settings").show(ctx, |ui| {
                ui.allocate_ui(vec2(300., 40.), |ui| {
                    if ui
                        .add(egui::widgets::ImageButton::new(egui::include_image!(
                            "../icons/settings.png"
                        )))
                        .clicked()
                    {
                        self.settings_window = !self.settings_window;
                    };
                });
            });
            //main
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.label(RichText::from("Server mode").strong().size(30.));
                    ui.label(RichText::from("Message stream").size(20.));
                    if !self.server_has_started {

                        ui.label(RichText::from("Server setup").size(30.).strong());
                        ui.separator();
                        ui.label(RichText::from("Open on port").size(20.));
                        ui.text_edit_singleline(&mut self.open_on_port);

                        let temp_open_on_port = &self.open_on_port;

                        if ui.button("Start").clicked() {
                            let temp_tx = self.stx.clone();
                            self.server_has_started = match temp_open_on_port.parse::<i32>(){
                                Ok(port) => {
                                    tokio::spawn(async move {

                                        match server::server_main(port.to_string()).await {
                                            Ok(ok) => {
                                                dbg!(&ok);

                                                let mut concatenated_string = String::new();

                                                for s in &ok {
                                                    concatenated_string.push_str(s);
                                                }

                                                match temp_tx.send(ok.join(&concatenated_string)){
                                                    Ok(_) => {},
                                                    Err(err) => {println!("ln 214 {}", err)},
                                                };

                                            },
                                            Err(err) => {
                                                println!("ln 208 {:?}", err);
                                            }
                                        };
                                    
                                    });
                                    true
                                },
                                Err(_) => {
                                    unsafe {
                                        MessageBoxW(0, w!("asd"), w!("asd"), 0);
                                    }
                                    false
                                }
                            };
                            
                        }
                    }
                    else {
                        //if server has already started
                        match self.srx.try_recv(){
                            Ok(ok) => {
                                ui.label(
                                    RichText::from(ok).size(15.)
                                );
                            },
                            Err(err) => {
                                //println!("ln  234 : {}", err);
                            },
                        };
                    }
                });
            });
        }

        //Client page
        if self.client_mode {
           
            egui::TopBottomPanel::new(egui::panel::TopBottomSide::Top, "setting_area").show(
                ctx,
                |ui| {
                    ui.allocate_space(vec2(ui.available_width(), 5.));

                    ui.allocate_ui(vec2(300., 40.), |ui| {
                        if ui
                            .add(egui::widgets::ImageButton::new(egui::include_image!(
                                "../icons/settings.png"
                            )))
                            .clicked()
                        {
                            self.settings_window = !self.settings_window;
                        };
                    });

                    ui.allocate_space(vec2(ui.available_width(), 5.));
                },
            );
            
            
            //msg_area
            egui::CentralPanel::default().show(ctx, |ui| {
                //Messages go here
                ui.allocate_ui(vec2(ui.available_width(), ui.available_height() - (_frame.info().window_info.size[1] / 5. + 10.)), |ui|{
                    egui::ScrollArea::vertical()
                    .id_source("msg_area")
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::from(&self.incoming_msg)
                                .size(self.font_size)
                        );
                    });
                });
                
                    
            });

            //usr_input
            egui::TopBottomPanel::bottom("usr_input").show_animated(ctx, true, |ui| {
                ui.allocate_space(vec2(ui.available_width(), 5.));

                ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                    ui.allocate_ui(
                        vec2(ui.available_width() - 100., _frame.info().window_info.size[1] / 5.),
                        |ui| {
                            egui::ScrollArea::vertical()
                                .id_source("usr_input")
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    ui.with_layout(
                                        egui::Layout::top_down_justified(Align::Center),
                                        |ui| {
                                            ui.add_sized(
                                                ui.available_size(),
                                                egui::TextEdit::multiline(&mut self.usr_msg).font(
                                                    egui::FontId::proportional(self.font_size),
                                                ),
                                            );
                                        },
                                    );
                                });
                        },
                    );
                    if ui.button("Send").clicked() {
                        let temp_msg = self.usr_msg.clone();
                        let tx = self.tx.clone();
                        let temp_port = match self.send_on_ip.clone().parse::<String>(){
                            Ok(ok) => {
                                tokio::spawn(async move {

                                match client::send_msg(temp_msg, "".into(), ok, false).await {
                                    Ok(ok) => {
                                        match tx.send(ok + "\n"){
                                            Ok(_) => {}
                                            Err(err) => {
                                                println!("{}", err);
                                            }
                                        };
                                    },
                                    Err(err) => {
                                        println!("ln 321 {}",err);
                                    }
                                };
                                
                            });
                        },
                            Err(_) => {
                                unsafe {
                                    MessageBoxW(0, w!("asd2"), w!("asd"), 0);
                                }
                            }
                        };
                        

                        match self.rx.try_recv() {
                            Ok(ok) => {
                                dbg!(ok.clone());
                                self.incoming_msg = ok
                            }
                            Err(err) => {
                                println!("ln 332 {}", err);
                            }
                        };
                    };
                });

                ui.allocate_space(vec2(ui.available_width(), 5.));
            });
            
        }

        //children windows
        egui::Window::new("Settings")
            .open(&mut self.settings_window)
            .show(ctx, |ui| {
                //show client mode settings
                if self.client_mode {
                    ui.label("Message editor text size");
                    ui.add(egui::Slider::new(&mut self.font_size, 1.0..=100.0).text("Text size"));
                    ui.separator();
                    ui.label("Connect to an ip address");
                    ui.text_edit_singleline(&mut self.send_on_ip);
                    if ui.button("Sync").clicked() {

                    };
                } else if self.server_mode {
                    ui.checkbox(&mut self.server_req_password, "Server requires password");
                    if self.server_req_password {
                        ui.text_edit_singleline(&mut self.server_password);
                    }
                }
            });
    }
}
