use egui::{Layout, Align, Image, vec2, Vec2};

mod client;


#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] 
pub struct TemplateApp {
    //window options
    #[serde(skip)]
    window_size: Vec2,


    #[serde(skip)]
    client_mode: bool,
    #[serde(skip)]
    server_mode: bool,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            //window options
            window_size: vec2(700., 300.),
            client_mode: false,
            server_mode: false,
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
        
        
        */

        //window options
        _frame.set_window_size(self.window_size);


        //for image loading
        egui_extras::install_image_loaders(ctx);

        //Main page
        if !(self.client_mode || self.server_mode)
        {
            egui::CentralPanel::default().show(ctx, |ui|{
                let Layout = Layout::left_to_right(Align::Center);
                
                ui.columns(2, |ui|{
    
                    ui[0].with_layout(Layout::centered_and_justified(egui::Direction::TopDown), |ui|{
                        if ui.add(
                            egui::widgets::ImageButton::new(
                                egui::include_image!("../icons/client.png")
                            )
                        ).on_hover_text("Enter Client mode")
                         .clicked() {
                            self.client_mode = true;
                         };
                    });
                    
    
                    ui[1].with_layout(Layout::centered_and_justified(egui::Direction::TopDown), |ui|{
                        if ui.add(egui::widgets::ImageButton::new(
                            egui::include_image!("../icons/server.png")
    
                            )
                        ).on_hover_text("Enter Server mode")
                         .clicked() {
                            self.server_mode = true;
                         };
                        
                        
                    });     
    
                });
    
                            
            });
        }
        
        //Server page
        if self.server_mode
        {

        }

        //Client page
        if self.client_mode
        {
            
        }

    }
}

