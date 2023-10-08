use std::sync::Mutex;
use chrono::Local;
use chrono::format::StrftimeItems;

use tonic::{transport::Server, Request, Response, Status};

use messages::message_server::{Message, MessageServer};
use messages::{MessageRequest, MessageResponse};

pub mod messages {
    tonic::include_proto!("messages");
}

#[derive(Debug, Default)]
pub struct MessageService {
    pub messages: Mutex<Vec<String>>,
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
        let format = StrftimeItems::new("%Y-%m-%d %H-%M");
        let formatted_datetime = current_datetime.format_with_items(format);

        if !&req.is_sync {
            match self.messages.lock() {
                Ok(mut ok) => {
                    ok.push(format!("{} $ {} | {} ", formatted_datetime , req.sent_by ,req.message) + "\n");
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

        let reply = MessageResponse {
            message: format!("{}", final_msg),
        };
        
        Ok(Response::new(reply))
    }
}

pub async fn server_main(port: String) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let addr = format!("[::1]:{}", port).parse()?;

    let btc_service = MessageService::default();
    let messages = &btc_service.messages.lock().unwrap().to_vec();
    Server::builder()
        .add_service(MessageServer::new(btc_service))
        .serve(addr)
        .await?;

    Ok(messages.to_vec())
}
