use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{tcp::OwnedWriteHalf, TcpStream},
};

use super::backend::{
    fetch_incoming_message_lenght, ClientMessage, ServerMaster,
};

/// Sends connection request to the specified server handle, returns the server's response, this function does not create a new thread, and may block
pub async fn connect_to_server(
    mut connection: TcpStream,
    message: ClientMessage,
) -> anyhow::Result<(String, TcpStream)> {
    let message_as_string = message.struct_into_string();

    let message_bytes = message_as_string.as_bytes();

    //Send message lenght to server
    connection
        .write_all(&(message_bytes.len() as u32).to_be_bytes())
        .await?;

    //Send message to server
    connection.write_all(message_bytes).await?;

    //Read the server reply lenght
    //blocks here for unknown reason
    let msg_len = fetch_incoming_message_lenght(&mut connection).await?;

    //Create buffer with said lenght
    let mut msg_buffer = vec![0; msg_len as usize];

    //Read the server reply
    connection.read_exact(&mut msg_buffer).await?;

    Ok((String::from_utf8(msg_buffer)?, connection))
}

/// This function can take a ```MutexGuard<TcpStream>>``` as a connection, but it does not check if the buffer is writeable
/// It also waits for the server to reply, so it awaits a sever repsonse
pub async fn send_message<T>(mut connection: T, message: ClientMessage) -> anyhow::Result<String>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin,
{
    let message_string = dbg!(message.struct_into_string());

    let message_bytes = message_string.as_bytes();

    //Send message lenght to server
    connection
        .write_all(&message_bytes.len().to_be_bytes())
        .await?;

    //Send message to server
    connection.write_all(message_bytes).await?;

    //Read the server reply lenght
    let msg_len = fetch_incoming_message_lenght(&mut connection).await?;

    //Create buffer with said lenght
    let mut msg_buffer = vec![0; msg_len as usize];

    //Read the server reply
    connection.read_exact(&mut msg_buffer).await?;

    Ok(String::from_utf8(msg_buffer)?)
}

/// This function should only be used when we want to send normal messages (backend::ClientMessageType::ClientNormalMessage)
/// because we will recive the server's reply in a different place with the ```OwnedReadHalf```, thats why this function only needs an ```OwnedWriteHalf```
pub async fn send_message_without_reply(
    mut connection: OwnedWriteHalf,
    message: ClientMessage,
) -> anyhow::Result<()> {
    let message_string = message.struct_into_string();

    let message_bytes = message_string.as_bytes();

    //Send message lenght to server
    connection
        .write_all(&message_bytes.len().to_be_bytes())
        .await?;

    //Send message to server
    connection.write_all(message_bytes).await?;

    Ok(())
}

#[inline]
pub async fn recive_message() -> anyhow::Result<ServerMaster> {
    todo!()
}
