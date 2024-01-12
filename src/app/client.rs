use messages::{message_client::MessageClient, MessageRequest};

use super::backend::ClientMessage;
pub mod messages {
    tonic::include_proto!("messages");
}

//main is for sending
#[inline]
pub async fn send_msg(message: ClientMessage) -> Result<String, Box<dyn std::error::Error>> {
    let mut client: MessageClient<tonic::transport::Channel> =
        MessageClient::connect(format!("http://{}", message.Destination)).await?;

    let request = tonic::Request::new(MessageRequest {
        message: message.struct_into_string(),
    });

    let response = client.message_main(request).await?.into_inner().clone();

    let message = response.message;

    Ok(message)
}
