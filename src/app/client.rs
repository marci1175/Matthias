use messages::{message_client::MessageClient, MessageRequest};
use tonic::transport::{Endpoint, Channel};

use super::backend::{ClientMessage, TemplateApp};
pub mod messages {
    tonic::include_proto!("messages");
}

//main is for sending
#[inline]
    pub async fn send_msg(message: ClientMessage) -> Result<String, Box<dyn std::error::Error>> {
    
        let mut client = MessageClient::new(todo!());
    
        let request = tonic::Request::new(MessageRequest {
            message: message.struct_into_string(),
        });
    
        let response = client.message_main(request).await?.into_inner().clone();
    
        let message = response.message;
    
        Ok(message)
    }