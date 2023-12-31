use crate::app::backend::TemplateApp;

use eframe::Frame;
use egui::{vec2, Align, Layout, RichText, ViewportCommand};

use std::sync::atomic::Ordering;

impl TemplateApp {
    pub fn state_mode_selection(&mut self, _frame: &mut Frame, ctx: &egui::Context) {
        //window settings
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(700., 300.)));
        ctx.send_viewport_cmd(ViewportCommand::Resizable(false));

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.allocate_ui(vec2(ui.available_width(), 20.), |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.label(RichText::from("Welcome,").weak().size(20.));
                    ui.label(
                        RichText::from(self.login_username.to_string())
                            .strong()
                            .size(20.),
                    );
                    if ui.button("Logout").clicked() {
                        self.main.mode_selector = false;
                    }
                });
            });

            ui.columns(2, |ui| {
                ui[0].with_layout(
                    Layout::centered_and_justified(egui::Direction::TopDown),
                    |ui| {
                        if ui
                            .add(egui::widgets::ImageButton::new(egui::include_image!(
                                "../../../icons/client.png"
                            )))
                            .on_hover_text("Enter Client mode")
                            .clicked()
                        {
                            self.main.client_mode = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(
                                1300., 800.,
                            )));
                            self.autosync_should_run.store(true, Ordering::Relaxed);
                        };
                    },
                );

                ui[1].with_layout(
                    Layout::centered_and_justified(egui::Direction::TopDown),
                    |ui| {
                        if ui
                            .add(egui::widgets::ImageButton::new(egui::include_image!(
                                "../../../icons/server.png"
                            )))
                            .on_hover_text("Enter Server mode")
                            .clicked()
                        {
                            self.main.server_mode = true;

                            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(
                                1000., 900.,
                            )));
                        };
                    },
                );
            });
        });
    }
}
