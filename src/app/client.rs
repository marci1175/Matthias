use Messages::message_client::MessageClient;
use Messages::{MessageRequest, MessageResponse};

pub mod Messages {
    tonic::include_proto!("messages");
}

//main is for sending
pub async fn send_msg(msg: String) -> Result<String, Box<dyn std::error::Error>> {
    let mut client = MessageClient::connect("http://[::1]:50051").await?;

    let request = tonic::Request::new(MessageRequest {
        message: msg.trim().to_string(),
        is_sync: false,
    });

    let response = client.send_message(request).await?;

    let message = response.into_inner().message;

    println!("RESPONSE={}", message);

    Ok(message)
}
