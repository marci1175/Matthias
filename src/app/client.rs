use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{tcp::OwnedReadHalf, TcpStream},
    sync::Mutex,
};

use super::backend::{fetch_incoming_message_lenght, ClientMessage, ConnectionPair};

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

use std::sync::Arc;

pub struct ServerReply {
    reader: Arc<Mutex<OwnedReadHalf>>,
}

impl ServerReply {
    pub async fn wait_for_response(&self) -> anyhow::Result<String> {
        let reader = &mut *self.reader.lock().await;
        // Read the server reply lenght
        let msg_len = fetch_incoming_message_lenght(reader).await?;

        //Create buffer with said lenght
        let mut msg_buffer = vec![0; msg_len as usize];

        //Read the server reply
        reader.read_exact(&mut msg_buffer).await?;

        Ok(String::from_utf8(msg_buffer)?)
    }

    pub fn new(reader: Arc<Mutex<OwnedReadHalf>>) -> Self {
        Self { reader }
    }
}

/// This function can take a ```MutexGuard<TcpStream>>``` as a connection
/// It also waits for the server to reply, so it awaits a sever repsonse
/// This function returns a wait for response value, which means when awaiting on the returned value of this function we are awaiting the response from the server
pub async fn send_message(
    connection_pair: ConnectionPair,
    message: ClientMessage,
) -> anyhow::Result<ServerReply> {
    let mut writer = connection_pair.writer.lock().await;

    let message_string = message.struct_into_string();

    let message_bytes = message_string.as_bytes();

    //Send message lenght to server
    writer
        .write_all(&(message_bytes.len() as u32).to_be_bytes())
        .await?;

    //Send message to server
    writer.write_all(message_bytes).await?;

    writer.flush().await?;

    Ok(ServerReply::new(connection_pair.reader))
}
