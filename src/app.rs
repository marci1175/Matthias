use egui::{vec2, Align, Image, Layout, Vec2, panel::Side};

mod client;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    //child windows
    #[serde(skip)]
    settings_window: bool,

    //window options
    #[serde(skip)]
    window_size: Vec2,

    //main
    #[serde(skip)]
    client_mode: bool,
    #[serde(skip)]
    server_mode: bool,

    //client_mode

    //font
    font_size: f32,

    //msg
    usr_msg: String,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            //child windows
            settings_window: false,
            //window options
            window_size: vec2(700., 300.),

            //main
            client_mode: false,
            server_mode: false,

            //client_mode

            //font
            font_size: 20.,

            //msg
            usr_msg: String::new(),
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

        */

        //window options
        _frame.set_window_size(self.window_size);

        //For image loading
        egui_extras::install_image_loaders(ctx);

        //Main page
        if !(self.client_mode || self.server_mode) {
            //main
            egui::CentralPanel::default().show(ctx, |ui| {
                let Layout = Layout::left_to_right(Align::Center);

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
                            };
                        },
                    );
                });
            });
        }

        //Server page
        if self.server_mode {
            
        }

        //Client page
        if self.client_mode {
            // window options
            self.window_size = vec2(1300., 800.);

            //settings
            egui::TopBottomPanel::new(egui::panel::TopBottomSide::Top, "setting_area").show(ctx, |ui|{
                ui.allocate_space(vec2(ui.available_width(), 5.));

                ui.allocate_ui(vec2(300., 40.), |ui|{
                    if ui.add(
                        egui::widgets::ImageButton::new(
                            egui::include_image!("../icons/settings.png")
                        )
                    ).clicked(){
                        self.settings_window = !self.settings_window;
                    };
                });

                ui.allocate_space(vec2(ui.available_width(), 5.));
            });

            //msg_area
            egui::CentralPanel::default().show(ctx, |ui| {
                //Messages go here
                egui::ScrollArea::vertical()
                    .id_source("msg_area")
                    .stick_to_bottom(true)
                    .show(ui, |ui| {});
            });

            //usr_input
            egui::TopBottomPanel::bottom("usr_input").show_animated(ctx, true, |ui| {
                
                ui.allocate_space(vec2(ui.available_width(), 5.));

                ui.with_layout(Layout::left_to_right(Align::Min), |ui|{
                    ui.allocate_ui(
                        vec2(ui.available_width() - 100., ctx.used_size()[1] / 5.),
                        |ui| {
                            egui::ScrollArea::vertical()
                                .id_source("usr_input")
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    ui.with_layout(
                                        egui::Layout::top_down_justified(Align::Center),
                                        |ui| {
                                            ui.add_sized(ui.available_size(), 
    
                                        egui::TextEdit::multiline(&mut self.usr_msg)
                                            .font(egui::FontId::proportional(self.font_size))
                                        );
                                        },
                                    );
                                });
                        },
                    );
                    ui.button("Send");
                });
                
                ui.allocate_space(vec2(ui.available_width(), 5.));
            });
        }
    
    
        //children windows
        egui::Window::new("Settings")
            .open(&mut self.settings_window)
            .show(ctx, |ui|{
                ui.label("Message editor text size");
                ui.add(egui::Slider::new(&mut self.font_size, 0.0..=100.0).text("Text size"));
            });
    }
}
