use chrono::format::StrftimeItems;
use chrono::Local;
use std::sync::Mutex;

use tonic::{transport::Server, Request, Response, Status};

use messages::message_server::{Message, MessageServer};
use messages::{MessageRequest, MessageResponse};


pub mod messages {
    tonic::include_proto!("messages");
}

#[derive(Debug, Default)]
pub struct MessageService {
    pub messages: Mutex<Vec<String>>,
    pub passw: String,
}

#[tonic::async_trait]
impl Message for MessageService {
    async fn send_message(
        &self,
        request: Request<MessageRequest>,
    ) -> Result<Response<MessageResponse>, Status> {
        println!("Got a request: {:?}", request);

        let req = request.into_inner();

        let current_datetime = Local::now();
        let format = StrftimeItems::new("%Y.%m.%d. %H:%M");
        let formatted_datetime = current_datetime.format_with_items(format);

        if !&req.is_sync && &req.password.trim() == &self.passw.trim() {
            match self.messages.lock() {
                Ok(mut ok) => {
                    ok.push(
                        format!(
                            "{} $ {} | {} ",
                            formatted_datetime, req.sent_by, req.message
                        ) + "\n",
                    );
                }
                Err(_) => {}
            };
        }
        let shared_messages = self.messages.lock().unwrap().clone();

        let handle = std::thread::spawn(move || {
            let final_msg: String = shared_messages
                .iter()
                .map(|s| s.to_string())
                .collect::<String>();

            final_msg
        });

        // Wait for the spawned thread to finish
        let final_msg = handle.join().unwrap();
        if &req.password.trim() == &self.passw.trim() {
            let reply = MessageResponse {
                message: format!("{}", final_msg),
            };

            Ok(Response::new(reply))
        }
        //invalid passw
        else {
            let reply = MessageResponse {
                message: format!("Invalid Password!"),
            };
            Ok(Response::new(reply))
        }
    }
}

pub async fn server_main(
    port: String,
    password: String,
    ip_v4: bool,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut addr: std::net::SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    if !ip_v4 {
        addr = format!("[::]:{}", port).parse()?;
    }
    let mut btc_service = MessageService::default();
    btc_service.passw = password;
    let messages = &btc_service.messages.lock().unwrap().to_vec();
    Server::builder()
        .add_service(MessageServer::new(btc_service))
        .serve(addr)
        .await?;

    Ok(messages.to_vec())
}
