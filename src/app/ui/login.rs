use crate::app::backend::{display_error_message, login};

use crate::app::backend::Application;
use egui::{Align, Layout, RichText};

impl Application
{
    pub fn state_login(&mut self, _frame: &mut eframe::Frame, ctx: &egui::Context)
    {
        let is_focused = ctx.input(|input| input.focused);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                ui.label(RichText::from("Matthias").strong().size(25.))
                    .on_hover_text(RichText::from(format!(
                        "Build date: {}",
                        include_str!("../../../build_info.Matthias_build")
                    )));
                ui.label("Username");
                ui.text_edit_singleline(&mut self.login_username);
                ui.label("Password");

                ui.add(egui::TextEdit::singleline(&mut self.login_password).password(true));

                if ui.button("Login").clicked() && is_focused
                    || ctx.input(|reader| reader.key_down(egui::Key::Enter))
                        && !(self.login_password.is_empty() && self.login_username.is_empty())
                {
                    self.main.client_mode =
                        match login(self.login_username.clone(), self.login_password.clone()) {
                            Ok((account, _path_to_account)) => {
                                //Load the parsed text into the variable
                                self.opened_user_information = account;

                                true
                            },
                            Err(err) => {
                                //Avoid panicking when trying to display a Notification
                                //This is very rare but can still happen
                                display_error_message(err, self.toasts.clone());

                                false
                            },
                        };
                }

                ui.separator();
                ui.label(RichText::from("You dont have an account yet?").weak());
                if ui.button("Register").clicked() {
                    self.main.register_mode = true;
                };
            });
        });
    }
}
