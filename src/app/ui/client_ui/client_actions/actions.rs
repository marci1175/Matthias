//use crate::app::account_manager::write_file;
use crate::app::backend::{ClientMessage, TemplateApp};
use crate::app::client::{self};

impl TemplateApp {
    /*

        ::  DEPRICATED FUNCTIONS ::

    pub fn send_audio(&mut self, file: std::path::PathBuf) {
        let passw = self.client_password.clone();
        let ip = self.send_on_ip.clone();
        let author = self.login_username.clone();
        let replying_to = self.replying_to;

        let message = ClientMessage::construct_audio_msg(file, ip, passw, author, replying_to);

        tokio::spawn(async move {
            let _ = client::send_msg(message).await;
        });
    }

    pub fn send_picture(&mut self, file: std::path::PathBuf) {
        let passw = self.client_password.clone();
        let ip = self.send_on_ip.clone();
        let author = self.login_username.clone();
        let replying_to = self.replying_to;

        let message = ClientMessage::construct_image_msg(file, ip, passw, author, replying_to);

        tokio::spawn(async move {
            let _ = client::send_msg(message).await;
        });
    }
    */

    pub fn send_file(&mut self, file: std::path::PathBuf) {
        let passw = self.client_password.clone();
        let ip = self.send_on_ip.clone();
        let author = self.login_username.clone();
        let replying_to = self.replying_to;

        let message = ClientMessage::construct_file_msg(file, ip, passw, author, replying_to);

        tokio::spawn(async move {
            let _ = client::send_msg(message).await;
        });
    }
}
