use Messages::message_client::MessageClient;
use Messages::{MessageRequest, MessageResponse};

pub mod Messages {
    tonic::include_proto!("messages");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = MessageClient::connect("http://[::1]:50051").await?;
    let mut msg: String = String::new();
    std::io::stdin().read_line(&mut msg).expect("msg");
    let request = tonic::Request::new(MessageRequest {
        message: msg.trim().to_string(),
        is_sync: false,
    });

    let response = client.send_message(request).await?;

    println!("RESPONSE={:?}", response);

    Ok(())
}
