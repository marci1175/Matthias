use messages::MessageRequest;

use super::backend::ClientMessage;
pub mod messages {
    tonic::include_proto!("messages");
}

use crate::app::backend::ClientConnection;

//main is for sending
#[inline]
pub async fn send_msg(
    connection: ClientConnection,
    message: ClientMessage,
) -> anyhow::Result<String> {
    if let Some(mut client) = connection.client.clone() {
        let request = tonic::Request::new(MessageRequest {
            message: message.struct_into_string(),
        });

        let response = client.message_main(request).await?.into_inner().clone();

        let message = response.message;

        //Reply
        Ok(message)
    } else {
        Err(anyhow::Error::msg("Request failed, see logs"))
    }
}
