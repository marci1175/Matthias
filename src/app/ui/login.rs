use std::iter;

use crate::app::account_manager::{login, register};

use crate::app::backend::TemplateApp;
use device_query::Keycode;
use egui::{vec2, Align, Layout, RichText, ViewportCommand};

use windows_sys::w;
use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_ICONWARNING};

impl TemplateApp {
    pub fn state_login(
        &mut self,
        _frame: &mut eframe::Frame,
        ctx: &egui::Context,
        input_keys: &Vec<Keycode>,
    ) {
        //windows settings
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(500., 200.)));
        ctx.send_viewport_cmd(ViewportCommand::Resizable(false));

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                ui.label(RichText::from("széChat").strong().size(25.))
                    .on_hover_text(RichText::from(format!(
                        "Build date: {}",
                        include_str!("../../../build_info.szechat_build")
                    )));
                ui.label("Username");
                ui.text_edit_singleline(&mut self.login_username);
                ui.label("Password");

                ui.add(egui::TextEdit::singleline(&mut self.login_password).password(true));

                if ui.button("Login").clicked()
                    || input_keys.contains(&Keycode::Enter)
                        && !(self.login_password.is_empty() && self.login_username.is_empty())
                {
                    self.main.mode_selector =
                        match login(self.login_username.clone(), self.login_password.clone()) {
                            Ok(ok) => {
                                self.main.opened_account_path = ok;
                                true
                            }
                            Err(err) => {
                                std::thread::spawn(move || unsafe {
                                    MessageBoxW(
                                        0,
                                        str::encode_utf16(err.to_string().as_str())
                                            .chain(iter::once(0))
                                            .collect::<Vec<_>>()
                                            .as_ptr(),
                                        w!("Error"),
                                        MB_ICONERROR,
                                    );
                                });
                                false
                            }
                        };
                }

                ui.separator();
                ui.label(RichText::from("You dont have an account yet?").weak());
                if ui.button("Register").clicked()
                    && !self.login_username.is_empty()
                    && !self.login_password.is_empty()
                {
                    match register(self.login_username.clone(), self.login_password.clone()) {
                        Ok(_) => {}
                        Err(err) => {
                            std::thread::spawn(move || unsafe {
                                MessageBoxW(
                                    0,
                                    str::encode_utf16(err.to_string().as_str())
                                        .chain(iter::once(0))
                                        .collect::<Vec<_>>()
                                        .as_ptr(),
                                    w!("Error"),
                                    MB_ICONWARNING,
                                );
                            });
                        }
                    };
                };
            });
        });
    }
}
