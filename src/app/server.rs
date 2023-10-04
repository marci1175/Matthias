use std::ops::{Deref, DerefMut};
use std::sync::Mutex;

use tonic::{transport::Server, Request, Response, Status};

use Messages::message_server::{Message, MessageServer};
use Messages::{MessageResponse, MessageRequest};

pub mod Messages {
    tonic::include_proto!("messages");
}

#[derive(Debug, Default)]
pub struct MessageService {
   pub messages: Mutex<Vec<String>>,
}

#[tonic::async_trait]
impl Message for MessageService {
    async fn send_message(&self, request: Request<MessageRequest> ) -> Result<Response<MessageResponse>, Status> {
        println!("Got a request: {:?}", request);

        let req = request.into_inner();
        
        if !&req.is_sync {
            match self.messages.lock(){
                Ok(mut ok) => {
                    ok.push(req.message);
                },
                Err(_) => {},
            };
        }

        let reply = MessageResponse {
            message: format!("{:?}", &self.messages.lock()).into(),
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let btc_service = MessageService::default();

    Server::builder()
        .add_service(MessageServer::new(btc_service))
        .serve(addr)
        .await?;

    Ok(())
}