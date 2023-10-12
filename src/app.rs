use std::time::Duration;
use egui::{vec2, Align, Layout, RichText};
use std::sync::{mpsc, Arc, Mutex};
use windows_sys::w;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, MB_ICONEXCLAMATION, MB_ICONINFORMATION,
};

mod account_manager;
mod client;
mod server;
mod networking;
use account_manager::{login, register};

use self::networking::ipv4_get;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    //login page
    login_username: String,
    #[serde(skip)]
    login_password: String,

    //server main
    #[serde(skip)]
    server_has_started: bool,
    #[serde(skip)]
    public_ip: String,
    //server settings
    server_req_password: bool,

    server_password: String,

    open_on_port: String,

    //thread communication for server
    #[serde(skip)]
    srx: mpsc::Receiver<String>,
    #[serde(skip)]
    stx: mpsc::Sender<String>,

    //child windows
    #[serde(skip)]
    settings_window: bool,

    //main
    #[serde(skip)]
    client_mode: bool,
    #[serde(skip)]
    server_mode: bool,
    #[serde(skip)]
    mode_selector: bool,

    //client main
    send_on_ip: String,
    req_passw: bool,
    client_password: String,
    //font
    font_size: f32,

    //msg
    #[serde(skip)]
    usr_msg: String,
    #[serde(skip)]
    incoming_msg_time: Vec<String>,
    #[serde(skip)]
    incoming_msg: String,

    //thread communication for client
    #[serde(skip)]
    rx: mpsc::Receiver<String>,
    #[serde(skip)]
    tx: mpsc::Sender<String>,
    //data sync
    #[serde(skip)]
    drx: mpsc::Receiver<String>,
    #[serde(skip)]
    dtx: mpsc::Sender<String>,
    #[serde(skip)]
    has_init: bool,
    #[serde(skip)]
    autosync_sender: Option<mpsc::Sender<String>>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel::<String>();
        let (stx, srx) = mpsc::channel::<String>();
        let (dtx, drx) = mpsc::channel::<String>();
        Self {
            //login page
            login_username: String::new(),
            login_password: String::new(),

            //server_main
            server_has_started: false,
            public_ip: String::new(),

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
            mode_selector: false,

            //client main
            send_on_ip: String::new(),
            req_passw: false,
            client_password: String::new(),
            //font
            font_size: 20.,

            //msg
            usr_msg: String::new(),
            incoming_msg_time: Vec::new(),
            incoming_msg: String::new(),
            //thread communication for client
            rx,
            tx,
            //data sync
            drx,
            dtx,
            has_init: false,
            autosync_sender: None,
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
        TODO: MAKE CLIENT UI BETTER
        TODO: IMPEMENT PASSWORD FOR SERVERS

        */

        //window options

        //For image loading
        egui_extras::install_image_loaders(ctx);

        /*
        //data sync

        match client::send_msg("".into(), self.server_password.clone(), self.send_on_ip.clone(), true){
            Ok(_) => {},
            Err(_) => {}
        };
        */

        //Login Page
        if !(self.mode_selector || self.server_mode || self.client_mode) {
            //windows settings
            _frame.set_window_size(vec2(500., 200.));

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.label(RichText::from("széChat v3").strong().size(25.));
                    ui.label("Username");
                    ui.text_edit_singleline(&mut self.login_username);
                    ui.label("Password");

                    ui.add(egui::TextEdit::singleline(&mut self.login_password).password(true));

                    if ui.button("Login").clicked()
                        && !(self.login_password.is_empty() && self.login_username.is_empty())
                    {
                        match login(self.login_username.clone(), self.login_password.clone()) {
                            true => {
                                self.mode_selector = true;
                            }
                            false => {
                                std::thread::spawn(|| unsafe {
                                    MessageBoxW(
                                        0,
                                        w!("Invalid password or username"),
                                        w!("Error"),
                                        MB_ICONEXCLAMATION,
                                    );
                                });
                            }
                        };
                    }

                    ui.separator();
                    ui.label(RichText::from("You dont have an account yet?").weak());
                    if ui.button("Register").clicked() {
                        match register(self.login_username.clone(), self.login_password.clone()) {
                            true => {
                                std::thread::spawn(|| unsafe {
                                    MessageBoxW(
                                        0,
                                        w!("Successfully registered!"),
                                        w!("Success"),
                                        MB_ICONINFORMATION,
                                    );
                                });
                            }
                            false => {}
                        };
                    };
                });
            });
        }
        //Main page
        if self.mode_selector && !(self.server_mode || self.client_mode) {
            //main

            //window settings
            _frame.set_window_size(vec2(700., 300.));

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.label(
                        RichText::from(format!("Welcome, {}", self.login_username))
                            .size(20.)
                            .weak(),
                    );
                });
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
                    if self.server_req_password && !self.server_password.is_empty() {
                        ui.label(RichText::from(format!(
                            "Password : {}",
                            self.server_password
                        )));
                    }
                    if !self.server_has_started {
                        ui.label(RichText::from("Server setup").size(30.).strong());
                        ui.separator();
                        ui.label(RichText::from("Open on port").size(20.));
                        ui.text_edit_singleline(&mut self.open_on_port);
                        ui.separator();
                        ui.checkbox(&mut self.server_req_password, "Server requires password");
                        if self.server_req_password {
                            ui.text_edit_singleline(&mut self.server_password);
                        }
                        let temp_open_on_port = &self.open_on_port;

                        if ui.button("Start").clicked() {
                            let temp_tx = self.stx.clone();
                            let server_pw = self.server_password.clone();
                            self.server_has_started = match temp_open_on_port.parse::<i32>() {
                                Ok(port) => {
                                    tokio::spawn(async move {
                                        match server::server_main(port.to_string(), server_pw).await
                                        {
                                            Ok(ok) => {
                                                dbg!(&ok);

                                                let mut concatenated_string = String::new();

                                                for s in &ok {
                                                    concatenated_string.push_str(s);
                                                }

                                                match temp_tx.send(ok.join(&concatenated_string)) {
                                                    Ok(_) => {}
                                                    Err(err) => {
                                                        println!("ln 214 {}", err)
                                                    }
                                                };
                                            }
                                            Err(err) => {
                                                println!("ln 208 {:?}", err);
                                            }
                                        };
                                    });
                                    true
                                }
                                Err(_) => {
                                    unsafe {
                                        MessageBoxW(0, w!("Error"), w!("Enter a valid port!"), 0);
                                    }
                                    false
                                }
                            };
                        }
                        ui.separator();
                    } else {
                        if self.public_ip.is_empty() {
                            let tx = self.dtx.clone();
                            std::thread::spawn(move || {
                                tx.send(ipv4_get().unwrap())
                            });
                            match self.drx.recv(){
                                Ok(ok) => { self.public_ip = ok },
                                Err(err) => { eprintln!("{}", err) },
                            }
                        }
                        ui.text_edit_singleline(&mut format!("Public ip address : {}", self.public_ip.trim()));
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

                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        ui.allocate_ui(vec2(300., 40.), |ui| {
                            if ui
                                .add(egui::widgets::ImageButton::new(egui::include_image!(
                                    "../icons/logout.png"
                                )))
                                .clicked()
                            {
                                self.client_mode = false;
                            };
                        })
                        .response
                        .on_hover_text("Logout");
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

                    ui.allocate_space(vec2(ui.available_width(), 5.));
                },
            );

            //msg_area
            egui::CentralPanel::default().show(ctx, |ui| {
                //Messages go here
                ui.allocate_ui(
                    vec2(
                        ui.available_width(),
                        ui.available_height() - (_frame.info().window_info.size[1] / 5. + 10.),
                    ),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .id_source("msg_area")
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                ui.separator();

                                ui.allocate_ui(ui.available_size(), |ui| {
                                    ui.label(
                                        RichText::from(self.incoming_msg.clone())
                                            .size(self.font_size),
                                    );
                                });

                                ui.separator();
                            });
                    },
                );
            });

            //usr_input
            egui::TopBottomPanel::bottom("usr_input").show_animated(ctx, true, |ui| {
                ui.allocate_space(vec2(ui.available_width(), 5.));

                ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                    ui.allocate_ui(
                        vec2(
                            ui.available_width() - 100.,
                            _frame.info().window_info.size[1] / 5.,
                        ),
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
                    if ui
                        .add(egui::widgets::ImageButton::new(egui::include_image!(
                            "../icons/send_msg.png"
                        )))
                        .clicked()
                    {
                        let temp_msg = self.usr_msg.clone();
                        let tx = self.tx.clone();
                        let username = self.login_username.clone();
                        let passw = self.client_password.clone();
                        let _ = match self.send_on_ip.clone().parse::<String>() {
                            Ok(ok) => {
                                tokio::spawn(async move {
                                    match client::send_msg(username, temp_msg, passw, ok, false)
                                        .await
                                    {
                                        Ok(ok) => {
                                            match tx.send(ok) {
                                                Ok(_) => {}
                                                Err(err) => {
                                                    println!("{}", err);
                                                }
                                            };
                                        }
                                        Err(err) => {
                                            println!("ln 321 {}", err);
                                        }
                                    };
                                });
                            }
                            Err(_) => unsafe {
                                MessageBoxW(0, w!("asd2"), w!("asd"), 0);
                            },
                        };
                    }
                });
                //receive server answer unconditionally
                match self.rx.try_recv() {
                    Ok(ok) => {
                        /*let incoming_msg: Vec<&str> = ok.split(";").collect();
                        let collect_msg: Vec<&str> = incoming_msg.iter().take(incoming_msg.len() - 1).cloned().collect();
                        let final_msg = collect_msg.concat();
                        let final_time = incoming_msg[incoming_msg.len() - 1].to_string();

                        self.incoming_msg.push(final_msg);
                        self.incoming_msg_time.push(final_time);
                        */
                        self.incoming_msg = ok
                    }
                    Err(_err) => {
                        //println!("ln 332 {}", err);
                    }
                };

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
                    ui.checkbox(&mut self.req_passw, "Set password");
                    if self.req_passw {
                        ui.text_edit_singleline(&mut self.client_password);
                    };
                } else if self.server_mode {
                }
            });
    }
}
