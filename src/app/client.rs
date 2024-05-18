use messages::MessageRequest;
use std::{ops::{Deref, DerefMut}, sync::Arc};
use tap::Tap;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
};

use super::backend::{
    create_vec_with_len, fetch_incoming_message_lenght, ClientMessage, ServerMaster,
};
pub mod messages {
    tonic::include_proto!("messages");
}

/// Sends connection request to the specified server handle, returns the server's response, this function does not create a new thread, and may block
pub async fn connect_to_server(
    connection: TcpStream,
    message: ClientMessage,
) -> anyhow::Result<(String, TcpStream)>
{
    let (mut reader, mut writer) = connection.into_split();

    writer.writable().await?;

    let message_as_string = message.struct_into_string();

    let message_bytes = message_as_string.as_bytes();

    writer.write_all(&message_bytes.len().to_be_bytes()).await?;

    writer.writable().await?;

    writer.write_all(message_bytes).await?;

    writer.flush().await?;

    //Read the server reply
    reader.readable().await?;

    let msg_len = fetch_incoming_message_lenght(&mut reader).await?;

    let mut msg_buffer = create_vec_with_len::<u8>(msg_len as usize);

    reader.read_exact(&mut msg_buffer).await;

    Ok((String::from_utf8(msg_buffer)?, reader.reunite(writer)?))
}

pub async fn send_message<T>(
    connection: T,
    message: ClientMessage
) -> anyhow::Result<String>
where
T: AsyncReadExt + AsyncWriteExt + Unpin,
{

    Ok(())
}

#[inline]
pub async fn recive_message() -> anyhow::Result<ServerMaster> {
    todo!()
}
