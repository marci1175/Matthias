use egui::RichText;

//use crate::app::account_manager::write_file;
use crate::app::backend::{ClientMessage, ServerMessageType, TemplateApp};
use crate::app::client;

impl TemplateApp {
    pub fn file_message_instance(
        &mut self,
        item: &crate::app::backend::ServerOutput,
        ui: &mut egui::Ui,
    ) {
        if let ServerMessageType::Upload(file) = &item.MessageType {
            if ui
                .button(RichText::from(file.file_name.to_string()).size(self.font_size))
                .clicked()
            {
                let passw = self.client_ui.client_password.clone();
                let author = self.login_username.clone();
                let sender = self.ftx.clone();

                let message = ClientMessage::construct_file_request_msg(file.index, &passw, author);

                let connection = self.client_connection.clone();

                tokio::spawn(async move {
                    match client::send_msg(connection, message).await {
                        Ok(ok) => {
                            match sender.send(ok) {
                                Ok(_) => {}
                                Err(err) => {
                                    println!("{}", err);
                                }
                            };
                        }
                        Err(err) => {
                            println!("{err} ln 264")
                        }
                    }
                });
            }
        }
    }
}
