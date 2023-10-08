use messages::message_client::MessageClient;
use messages::MessageRequest;

pub mod messages {
    tonic::include_proto!("messages");
}

//main is for sending
pub async fn send_msg(
    msg: String,
    passw: String,
    ip: String,
    is_sync: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut client = MessageClient::connect(format!("http://{}", ip)).await?;

    let request = tonic::Request::new(MessageRequest {
        message: msg.trim().to_string(),
        is_sync: is_sync,
        password: passw,
    });

    let response = client.send_message(request).await?;

    let message = response.into_inner().message;

    Ok(message)
}
