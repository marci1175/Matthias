use messages::message_client::MessageClient;
use messages::MessageRequest;

pub mod messages {
    tonic::include_proto!("messages");
}

//main is for sending
pub async fn send_msg(
    username: String,
    msg: String,
    passw: String,
    ip: String,
    is_sync: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut client: MessageClient<tonic::transport::Channel> =
        MessageClient::connect(format!("http://{}", ip)).await?;

    let request = tonic::Request::new(MessageRequest {
        message: msg.trim().to_string(),
        sent_by: username,
        is_sync: is_sync,
        password: passw,
    });

    let response = client.send_message(request).await?.into_inner().clone();

    let message = response.message;

    Ok(message)
}
