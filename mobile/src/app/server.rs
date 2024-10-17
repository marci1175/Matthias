pub const SERVER_UUID: &str = "00000000-0000-0000-0000-000000000000";
pub const SERVER_AUTHOR: &str = "Server";

use std::{
    collections::HashMap, env, fs, io::Write, net::SocketAddr, path::PathBuf, sync::Arc,
    time::Duration,
};

use crate::app::client::{HASH_BYTE_OFFSET, IDENTIFICATOR_BYTE_OFFSET, UUID_BYTE_OFFSET};

use anyhow::{bail, Error, Result};
use chrono::Utc;
use dashmap::DashMap;
use egui::Context;
use indexmap::IndexMap;
use tokio_util::sync::CancellationToken;

use super::backend::{
    encrypt, encrypt_aes256, fetch_incoming_message_length, ClientLastSeenMessage,
    ClientMessageType, ClientProfile, ConnectedClient, ConnectionType, MessageReaction, Reaction,
    ReactionType, ServerClientReply, ServerMessageType,
    ServerMessageTypeDiscriminants::{
        Audio, Edit, Image, Normal, Reaction as ServerMessageTypeDiscriminantReaction, Sync,
        Upload, VoipConnection as Voip,
    },
    ServerReplyType, ServerSync, ServerVoip, ServerVoipReply, ServerVoipState,
};

use super::backend::{
    decrypt_aes256_bytes, encrypt_aes256_bytes, get_image_header,
    ClientFileRequestType as ClientRequestTypeStruct, ClientFileUpload as ClientFileUploadStruct,
    ClientMessage,
    ClientMessageType::{
        FileRequestType, FileUpload, MessageEdit, NormalMessage, Reaction as ClientReaction,
        SyncMessage, VoipConnection,
    },
    ImageHeader, ServerFileReply, ServerImageReply, ServerMaster, UdpMessageType,
};

use tokio::{
    io::AsyncWrite,
    net::{tcp::OwnedReadHalf, UdpSocket},
    select,
    sync::{mpsc, mpsc::Receiver},
    task::JoinHandle,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{self, tcp::OwnedWriteHalf},
};

use super::backend::{ServerAudioReply, ServerOutput};

#[derive(Debug, Default)]
pub struct MessageService
{
    /// Contains all the messages
    pub messages: Arc<tokio::sync::Mutex<Vec<ServerOutput>>>,

    /// Contains all of the reactions added to the messages
    /// Please note that the ```MessageReaction struct contains the emoji list itself```
    /// Needs rework
    pub reactions: Arc<tokio::sync::Mutex<Vec<MessageReaction>>>,

    /// This is the required password by the server this password is hashed with argon2, and is compared with the hashed client password
    pub passw: String,

    /// This is the list, which we will send the files from, these are generated file names, so names will rarely ever match (1 / 1.8446744e+19) chance
    /// The names are not relevant since when downloading them the client will always ask for a new name
    pub file_list: Arc<DashMap<String, PathBuf>>,

    /// This list contains a list of the path to the stored images
    /// When the client is asking for a file, they provide an index (which we provided originally when syncing, aka sending the latest message to all the clients)
    pub image_list: Arc<DashMap<String, PathBuf>>,

    /// This list contains a list of the path to the stored audio files
    /// When the client is asking for a file, they provide an index (which we provided originally when syncing, aka sending the latest message to all the clients)
    pub audio_list: Arc<DashMap<String, PathBuf>>,

    /// This list contains the names of the saved audios, since we generate a random name for the files we want to store
    /// We also dont ask the user the provide a name whenever playing an audio (requesting it from the server)
    pub audio_names: Arc<DashMap<String, Option<String>>>,

    ///connected clients
    pub connected_clients: Arc<tokio::sync::Mutex<Vec<ConnectedClient>>>,

    /// Client secret
    pub decryption_key: [u8; 32],

    /// Client last seen message
    pub clients_last_seen_index: Arc<tokio::sync::Mutex<Vec<ClientLastSeenMessage>>>,

    /// This hashmap contains the connected clients' profiles
    /// In this hashmap the key is the connecting client's uuid, and the value is the ClientProfile struct (which will later get converted to string with serde_json)
    pub connected_clients_profile: Arc<tokio::sync::Mutex<HashMap<String, ClientProfile>>>,

    /// This field contains all the shared fields, these fields are shared with the frontend
    pub shared_fields: Arc<tokio::sync::Mutex<SharedFields>>,

    pub voip: Option<ServerVoip>,

    opened_on_port: String,
}

/// This struct has fields which are exposed to the Ui / Main thread, so they can freely modified via the channel system
#[derive(Debug, Clone, Default)]
pub struct SharedFields
{
    /// This list contains the banned uuids
    pub banned_uuids: Arc<tokio::sync::Mutex<Vec<String>>>,
}

/// Shutting down server also doesnt work we will have to figure a way out on how to stop client readers (probably a broadcast channel)
pub async fn server_main(
    port: String,
    password: String,
    //This signals all the client receivers to be shut down
    cancellation_token: CancellationToken,
    connected_clients_profile_list: Arc<DashMap<String, ClientProfile>>,
    //We pass in ctx so we can request repaint when someone connects
    ctx: Context,
) -> Result<Arc<tokio::sync::Mutex<SharedFields>>, Box<dyn std::error::Error>>
{
    //Start listening
    let tcp_listener = net::TcpListener::bind(format!("[::]:{}", port)).await?;

    //Server default information
    let msg_service = Arc::new(tokio::sync::Mutex::new(MessageService {
        passw: encrypt(password),
        decryption_key: rand::random::<[u8; 32]>(),
        opened_on_port: port,
        ..Default::default()
    }));

    //This is used to shutdown the main server thread
    let cancellation_child = cancellation_token.child_token();

    //This is used to shutdown the Ui-Server sync thread
    let cancellation_child_clone = cancellation_child.clone();

    //We have to clone here to be able to move this into the thread
    let msg_service_clone = msg_service.clone();

    //Server thread
    let _: tokio::task::JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        loop {
            //Wait for incoming connections or wait till the server gets shut down
            let (stream, socket_addr) = select! {
                _ = cancellation_child.cancelled() => {
                    //shutdown server
                    break;
                }
                connection = tcp_listener.accept() => {
                    connection?
                }
            };

            //split client stream, so we will be able to store these separately
            let (reader, writer) = stream.into_split();

            //We need to clone here too, to pass it into the listener thread
            let message_service_clone = msg_service_clone.clone();

            //Listen for future client messages (IF the client stays connected)
            spawn_client_reader(
                Arc::new(tokio::sync::Mutex::new(reader)),
                Arc::new(tokio::sync::Mutex::new(writer)),
                message_service_clone,
                cancellation_token.child_token(),
                socket_addr,
            );
        }
        Ok(())
    });

    //We have to clone here to be able to move it into the thread
    let message_service_clone = msg_service.clone();

    //This thread keeps in sync with the ui, so the user can interact with the servers settings
    let _: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        loop {
            select! {
                //We should only init a sync 3 secs
                _ = tokio::time::sleep(Duration::from_secs(3)) => {
                    ctx.request_repaint();

                    let message_service_lock = message_service_clone.lock().await;

                    //The original client list contained by the server
                    let connected_clients_server = message_service_lock.connected_clients_profile.lock().await.clone();

                    connected_clients_profile_list.clear();

                    //Since we cant just rewrite the connected_clients we clear and then insert every
                    for (key, value) in connected_clients_server.into_iter() {
                        connected_clients_profile_list.insert(key.clone(), value);
                    }

                    let mut clients = message_service_lock.connected_clients.lock().await;

                    //Iter through connected clients
                    for (idx, client) in clients.clone().iter().enumerate() {
                        //Iter through banned uuids
                        for banned_uuid in message_service_lock.shared_fields.lock().await.banned_uuids.lock().await.iter() {
                            //If there is a matching uuid in the connected clients list and the banned uuids, we should disconnect using the handle
                            if client.uuid == *banned_uuid {
                                message_service_lock.handle_server_ban(client, &mut clients, idx).await?;
                            }
                        }
                    }
                },

                _ = cancellation_child_clone.cancelled() => {
                    //shutdown sync thread
                    break;
                },
            }
        }
        Ok(())
    });

    //Lock message service so we can access the fields
    let msg_svc = msg_service.lock().await;

    //We return an Arc<Rwlock> handle to the banned uuids, which can be later modified by the Ui
    Ok(msg_svc.shared_fields.clone())
}

/// This function does not need to be async since it spawn an async thread anyway
/// Spawn reader thread, this will constantly listen to the client which was connected, this thread will only finish if the client disconnects
#[inline]
fn spawn_client_reader(
    reader: Arc<tokio::sync::Mutex<OwnedReadHalf>>,
    writer: Arc<tokio::sync::Mutex<OwnedWriteHalf>>,
    msg_service: Arc<tokio::sync::Mutex<MessageService>>,
    cancellation_token: CancellationToken,
    socket_addr: SocketAddr,
)
{
    let _: tokio::task::JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        loop {
            //Wait until client sends a message or thread gets cancelled
            let incoming_message = select! {
                //Check if the thread needs to be shut down
                _ = cancellation_token.cancelled() => {
                    //Send out shutdown messages to all the clients
                    //If thread has been cancelled break out of the loop, thus ending the thread
                    break;
                }

                msg = receive_message(reader.clone()) => {
                    msg?
                }
            };

            let mut message_service = msg_service.lock().await;

            match message_service
                .message_main(incoming_message, writer.clone(), socket_addr)
                .await
            {
                Ok(_) => {},
                Err(err) => {
                    println!("Listener on {socket_addr} shutting down, error processing a message: {err}");
                    tracing::error!("{}", err.root_cause());

                    break;
                },
            }
        }
        Ok(())
    });
}

#[inline]
async fn receive_message(reader: Arc<tokio::sync::Mutex<OwnedReadHalf>>) -> Result<String>
{
    let mut reader = reader.lock().await;

    let incoming_message_len = fetch_incoming_message_length(&mut *reader).await?;

    let mut message_buffer: Vec<u8> = vec![0; incoming_message_len as usize];

    //Wait until the client sends the main message
    reader.read_exact(&mut message_buffer).await?;

    let message = String::from_utf8(message_buffer)?;

    Ok(message)
}

#[inline]
/// This function iterates over all the connected clients and all the messages, and sends writes them all to their designated ```OwnedWriteHalf``` (All of the users see all of the messages)
/// This creates a server_master message, with the message passed in being the only one in the list of the messages
async fn sync_message_with_clients(
    //The connected clients
    connected_clients: Arc<tokio::sync::Mutex<Vec<ConnectedClient>>>,

    //The connected clients' seen list (the last message's index theyve last seen)
    user_seen_list: Arc<tokio::sync::Mutex<Vec<ClientLastSeenMessage>>>,

    //The message sent by the owner
    //This struct contains the owner of this message (by name & uuid)
    message: ServerOutput,

    key: [u8; 32],
) -> anyhow::Result<()>
{
    let mut connected_clients_locked = connected_clients.lock().await;

    let server_master = ServerSync {
        message,
        user_seen_list: user_seen_list.lock().await.to_vec(),
    };

    let server_master_string = server_master.struct_into_string();

    //Encrypt string
    let encrypted_string = encrypt_aes256(server_master_string, &key).unwrap();

    //Send message length
    let message_length = TryInto::<u32>::try_into(encrypted_string.as_bytes().len())?;

    for client in connected_clients_locked.iter_mut() {
        if let Some(client_handle) = &mut client.handle {
            let mut client_handle = client_handle.lock().await;

            client_handle
                .write_all(&message_length.to_be_bytes())
                .await?;

            //Send actual message
            client_handle.write_all(encrypted_string.as_bytes()).await?;

            client_handle.flush().await?;
        };
    }

    Ok(())
}

pub async fn send_message_to_client<T>(mut writer: T, message: String) -> anyhow::Result<()>
where
    T: AsyncWriteExt + Unpin + AsyncWrite,
{
    let message_bytes = message.as_bytes();

    //Send message length
    writer
        .write_all(&(message_bytes.len() as u32).to_be_bytes())
        .await?;

    //Send message
    writer.write_all(message_bytes).await?;

    Ok(())
}

/// This function will create a management thread, but only if the ```voip.threads``` field is None (Preventing spawning multiple threads)
pub fn create_client_voip_manager(
    voip: ServerVoip,
    shutdown_token: CancellationToken,
    key: [u8; 32],
    mut receiver: Receiver<Vec<u8>>,
    #[allow(unused_variables)] listening_to: SocketAddr,
    uuid: String,
)
{
    let message_buffer = voip.message_buffer.clone();

    //Spawn client management thread
    tokio::spawn(async move {
        loop {
            let socket = voip.socket.clone();
            //Clone so we can move the value
            let voip_connected_clients = voip.connected_clients.clone();

            let image_buffer: Arc<
                DashMap<String, IndexMap<String, HashMap<String, Option<Vec<u8>>>>>,
            > = message_buffer.clone();

            select! {
                _ = shutdown_token.cancelled() => {
                    //Shutdown thread by exiting the loop
                    break;
                },

                //receive_message length by reading its first 4 bytes
                received_bytes = receiver.recv() => {
                    let received_bytes = received_bytes.unwrap();

                    //Decrypt message
                    // [. . . . . .4][4 . . . . len - 4][len - 4..]
                    //  PACKET LENGTH       MESSAGE      MSG TYPE
                    let mut decrypted_bytes = decrypt_aes256_bytes(&received_bytes, &key).unwrap();

                    let message_type_bytes: Vec<u8> = decrypted_bytes.drain(decrypted_bytes.len() - 4..).collect();

                    //Get message type by reading last 4 bytes
                    let message_type = UdpMessageType::from_number(u32::from_be_bytes(message_type_bytes.try_into().unwrap()));

                    match message_type {
                        UdpMessageType::Voice => {
                            //Spawn relay thread
                            tokio::spawn(async move {
                                //Relay message to all of the clients
                                for connected_socket_addr in voip_connected_clients.iter().filter(|entry| {
                                    #[allow(unused_variables)]
                                    let socket_addr = entry.value();

                                    //We dont send the user's voice to them in release builds
                                    #[cfg(not(debug_assertions))]
                                    {
                                        *socket_addr != listening_to
                                    }

                                    //We allow voice loopback in debug builds
                                    #[cfg(debug_assertions)]
                                    {
                                        true
                                    }
                                }).map(|entry| *entry.value()) {
                                    let mut decrypted_bytes = decrypted_bytes.clone();

                                    //Append message type, this will be encrypted
                                    decrypted_bytes.append(&mut (message_type.clone() as u32).to_be_bytes().to_vec());

                                    //Encrypt packet
                                    let mut encrypted_packet = encrypt_aes256_bytes(&decrypted_bytes, &key).unwrap();

                                    //Get encrypted packet size
                                    let mut message_length_header = (encrypted_packet.len() as u32).to_be_bytes().to_vec();

                                    //Append message to header
                                    message_length_header.append(&mut encrypted_packet);

                                    //Send the header indicating message length and send the whole message appended to it
                                    socket.send_to(&message_length_header, connected_socket_addr).await.unwrap();
                                }
                            });
                        }
                        UdpMessageType::ImageHeader => {
                            get_image_header(&decrypted_bytes, &image_buffer).unwrap();
                        }
                        UdpMessageType::Image => {
                            // [. . . . . . . . . . . len - 164][len - 164 . . . . . len - 100][len - 100. . . . . len - 64][len - 64 . . . .]
                            //      IMAGE                           HASH                            UUID                      IDENTIFICATOR
                            let message_bytes = decrypted_bytes.to_vec();

                            //Get the identificator of the image part in bytes
                            let indetificator_bytes = message_bytes[message_bytes.len() - IDENTIFICATOR_BYTE_OFFSET..].to_vec();

                            let identificator = String::from_utf8(indetificator_bytes).unwrap();

                            //Get the identificator of the image part in bytes
                            let hash_bytes = message_bytes[message_bytes.len() - HASH_BYTE_OFFSET..message_bytes.len() - UUID_BYTE_OFFSET].to_vec();

                            let hash = String::from_utf8(hash_bytes).unwrap();

                            //Get the image part bytes
                            //We subtract 164 bytes to only get the image part
                            let image = message_bytes[..message_bytes.len() - HASH_BYTE_OFFSET].to_vec();

                            //THIS IS UNUSED AND SHOULD BE REMOVED
                            let _uuid_bytes = message_bytes[message_bytes.len() - UUID_BYTE_OFFSET..message_bytes.len() - IDENTIFICATOR_BYTE_OFFSET].to_vec();

                            if let Some(mut image_header) = image_buffer.get_mut(&uuid) {
                                if let Some((index, _, contents)) = image_header.get_full_mut(&identificator) {

                                    if let Some(byte_pair) = contents.get_mut(&hash) {
                                        *byte_pair = Some(image);
                                    }
                                    else {
                                        tracing::error!("Image part hash not found in the image header: {hash}");
                                    }

                                    //If all the parts of the image header had arrived send the image to all the clients
                                    if contents.iter().all(|(_, value)| value.is_some()) {
                                        let contents_clone = contents.clone();
                                        let image_buffer = image_buffer.clone();

                                        tokio::spawn(async move {
                                            for connected_client in voip_connected_clients.iter() {
                                                let uuid = connected_client.key();

                                                let socket_addr = connected_client.value();

                                                //Combine the image part bytes
                                                let image_bytes: Vec<u8> = contents_clone.iter().flat_map(|(_, value)| {
                                                    <std::option::Option<std::vec::Vec<u8>> as Clone>::clone(value).unwrap()
                                                }).collect();

                                                //Create image parts by splitting it every 60000 bytes
                                                let image_parts_tuple: Vec<(String, &[u8])> = image_bytes
                                                    .chunks(60000)
                                                    .map(|image_part| (sha256::digest(image_part), image_part))
                                                    .collect();

                                                let image_parts = Vec::from_iter(image_parts_tuple.iter().map(|part| part.0.clone()));

                                                let identificator = sha256::digest(
                                                    image_parts
                                                        .iter()
                                                        .flat_map(|hash| hash.as_bytes().to_vec())
                                                        .collect::<Vec<u8>>(),
                                                );

                                                //Create header message
                                                let header_message =
                                                    ImageHeader::new(uuid.clone(), image_parts.clone(), identificator.clone());

                                                if image_bytes == vec![0] {
                                                    println!("asd");
                                                    image_buffer.remove(&uuid.clone());
                                                }

                                                // Send image header
                                                send_bytes(
                                                    serde_json::to_string(&header_message).unwrap().as_bytes().to_vec(),
                                                    &key,
                                                    UdpMessageType::ImageHeader,
                                                    socket.clone(),
                                                    *socket_addr,
                                                )
                                                .await.unwrap();

                                                //Send image parts
                                                //We have already sent the image header
                                                send_image_parts(image_parts_tuple, uuid.clone(), &key, identificator, socket.clone(), *socket_addr)
                                                    .await.unwrap();
                                            }
                                        });

                                        //Drain earlier ImageHeaders (and the current one), because a new one has arrived
                                        image_header.drain(index..=index);
                                    };
                                }
                                else {
                                    tracing::error!("Image header not found: {identificator}");
                                }
                            }
                            else {
                                tracing::error!("User not found in the image header list: {uuid}");
                            };
                        }
                    }
                },
            }
        }
    });
}

async fn send_bytes(
    mut bytes: Vec<u8>,
    encryption_key: &[u8],
    message_type: UdpMessageType,
    socket: Arc<UdpSocket>,
    send_to: SocketAddr,
) -> anyhow::Result<()>
{
    //Append message flag bytes
    bytes.append(&mut (message_type as u32).to_be_bytes().to_vec());

    //Encrypt message
    let mut encrypted_message = encrypt_aes256_bytes(&bytes, encryption_key)?;

    //Get message length
    let mut message_length_in_bytes = (encrypted_message.len() as u32).to_be_bytes().to_vec();

    //Append message to message length
    message_length_in_bytes.append(&mut encrypted_message);

    //Check for packet length overflow
    let bytes_length = message_length_in_bytes.len();

    if bytes_length > 65536 {
        bail!(format!(
            "Udp packet length overflow, with length of {bytes_length}"
        ))
    }

    //Send bytes
    socket.send_to(&message_length_in_bytes, send_to).await?;

    Ok(())
}
/// Send the images specified in the ```image_parts_tuple``` argument
/// __Image message contents:__
/// - ```[len - 64 - 64 - 36..len - 64 - 36]``` = Contains the hash (sha256 hash) of the image part we are sending
/// - ```[len - 64 - 36.. len - 64]``` = Contains the UUID of the author who has sent the message
/// - ```[..len - 64 - 64 - 36]``` = Contains the image part we are sending (JPEG image)
/// - ```[len - 64..]``` = Contains the identificator of the part we are sending
/// - **The hash length is 64 bytes.**
/// - **The identificator is 64 bytes.**
/// - **The uuid is 36 bytes.**
async fn send_image_parts(
    image_parts_tuple: Vec<(String, &[u8])>,
    uuid: String,
    encryption_key: &[u8],
    identificator: String,
    socket: Arc<UdpSocket>,
    send_to: SocketAddr,
) -> Result<(), Error>
{
    for (hash, bytes) in image_parts_tuple {
        //Hash as bytes
        let mut hash = hash.as_bytes().to_vec();

        //Append the hash to the bytes
        let mut bytes = bytes.to_vec();

        //Append hash
        bytes.append(&mut hash);

        //Append uuid to the message
        bytes.append(&mut uuid.as_bytes().to_vec());

        //Append identificator
        bytes.append(&mut identificator.as_bytes().to_vec());

        //Send bytes
        send_bytes(
            bytes,
            encryption_key,
            UdpMessageType::Image,
            socket.clone(),
            send_to,
        )
        .await?;
    }

    Ok(())
}

impl MessageService
{
    /// The result returned by this function may be a real error, or an error constructed on purpose so that the thread call this function gets shut down.
    /// When experiencing errors, make sure to check the error message as it may be on purpose
    #[inline]
    async fn message_main(
        &mut self,
        message: String,
        client_handle: Arc<tokio::sync::Mutex<OwnedWriteHalf>>,
        socket_addr: SocketAddr,
    ) -> Result<()>
    {
        let req_result: Result<ClientMessage, serde_json::Error> = serde_json::from_str(&message);

        let req: ClientMessage = req_result.unwrap();

        //If its a Client reaction or a message edit we shouldnt allocate more MessageReactions, since those are not actual messages
        //HOWEVER, if they're client connection or disconnection messages a reaction should be allocated because people can react to those
        if !(matches!(&req.message_type, ClientReaction(_))
            || matches!(&req.message_type, MessageEdit(_))
            || {
                if let ClientMessageType::SyncMessage(sync_msg) = &req.message_type {
                    //If this is true (if sync_attribute is none) that means the client is syncing its last seen message index, thefor we shouldnt allocate a new reaction
                    sync_msg.sync_attribute.is_none()
                }
                else {
                    false
                }
            })
        {
            //Allocate a reaction after every type of message except a sync message
            match self.reactions.try_lock() {
                Ok(mut ok) => {
                    ok.push(MessageReaction {
                        message_reactions: Vec::new(),
                    });
                },
                Err(err) => {
                    println!("{err}")
                },
            };
        }

        if let ClientMessageType::SyncMessage(sync_msg) = &req.message_type {
            if sync_msg.password == self.passw.trim() {
                //Handle incoming connections and disconnections, if sync_attr is a None then its just a message for syncing
                if let Some(sync_attr) = &sync_msg.sync_attribute {
                    match sync_attr {
                        ConnectionType::Connect(profile) => {
                            //Check if user has been banned
                            if self
                                .shared_fields
                                .lock()
                                .await
                                .banned_uuids
                                .lock()
                                .await
                                .iter()
                                .any(|item| *item == req.uuid)
                            {
                                send_message_to_client(
                                    &mut *client_handle.lock().await,
                                    "You have been banned!".to_string(),
                                )
                                .await?;

                                return Err(Error::msg("Client has been banned!"));
                            }
                            else {
                                let mut clients = self.connected_clients.lock().await;

                                //Check if the client has already been connected once
                                for client in clients.iter() {
                                    //If found, then the client is already connected
                                    if client.uuid == req.uuid {
                                        //This can only happen if the connection closed unexpectedly (If the client was stopped unexpectedly)
                                        send_message_to_client(
                                            &mut *client_handle.lock().await,
                                            hex::encode(self.decryption_key),
                                        )
                                        .await?;

                                        //If found return, and end execution
                                        return Ok(());
                                    }
                                }

                                //When spawning a client reader, we should announce it to the whole chat group (Adding a Server(UserConnect) enum to the messages list)
                                let server_msg = ServerOutput {
                                    replying_to: None,
                                    message_type: ServerMessageType::Server(
                                        super::backend::ServerMessage::Connect(profile.clone()),
                                    ),
                                    author: SERVER_AUTHOR.to_string(),
                                    message_date: {
                                        Utc::now().format("%Y.%m.%d. %H:%M").to_string()
                                    },
                                    uuid: SERVER_UUID.to_string(),
                                };

                                self.messages.lock().await.push(server_msg.clone());

                                //We should sync the connection message with all the clients except the connecting one, therefor we only pus hback the connected client after we have syncted this message with all the clients
                                sync_message_with_clients(
                                    Arc::new(tokio::sync::Mutex::new(clients.clone())),
                                    self.clients_last_seen_index.clone(),
                                    server_msg,
                                    self.decryption_key,
                                )
                                .await?;

                                //If the ip is not found then add it to connected clients
                                clients.push(ConnectedClient::new(
                                    req.uuid.clone(),
                                    profile.username.clone(),
                                    client_handle.clone(),
                                ));

                                //Store connected client's profile
                                self.connected_clients_profile
                                    .lock()
                                    .await
                                    .insert(req.uuid, profile.clone());

                                //Return custom key which the server's text will be encrypted with
                                send_message_to_client(
                                    &mut *client_handle.try_lock()?,
                                    hex::encode(self.decryption_key),
                                )
                                .await?;

                                //Sync all messages, send all of the messages to the client, because we have already provided the decryption key
                                send_message_to_client(
                                    &mut *client_handle.try_lock()?,
                                    self.full_sync_client().await?,
                                )
                                .await?;
                                return Ok(());
                            }
                        },
                        //Handle disconnections
                        ConnectionType::Disconnect => {
                            let mut clients = self.connected_clients.lock().await;
                            //Search for connected ip in all connected ips
                            for (index, client) in clients.clone().iter().enumerate() {
                                //If found, then disconnect the client
                                if client.uuid == req.uuid {
                                    let server_msg = self
                                        .handle_server_disconnect(client, &mut clients, index)
                                        .await?;

                                    sync_message_with_clients(
                                        Arc::new(tokio::sync::Mutex::new(clients.clone())),
                                        self.clients_last_seen_index.clone(),
                                        server_msg,
                                        self.decryption_key,
                                    )
                                    .await?;

                                    return Err(Error::msg("Client disconnected!"));
                                }
                            }
                        },
                    }
                }
            }
            else {
                send_message_to_client(&mut *client_handle.try_lock()?, "Invalid Password!".into())
                    .await?;

                //return an error so the client listener thread stops
                return Err(Error::msg("Invalid password entered by client!"));
            }
        }

        //If a client manages to stay connected after being banned this check should server as protection
        //This will check if the sender's uuid is in the connected client's list, which it should be since the client needs to connect to the server (Sending information), before being allowed to send a message
        if !self
            .connected_clients
            .lock()
            .await
            .iter()
            .any(|client| client.uuid == req.uuid)
        {
            let mut client_handle = &mut *client_handle.try_lock()?;
            //Disconnect from the client for real, and send an error message
            send_message_to_client(&mut client_handle, "Failed to authenticate!".into()).await?;

            client_handle.shutdown().await?;
        }

        //Check if user has been banned
        self.handle_banned_uuid(&req, &client_handle).await?;

        //if the client is not found in the list means we have not established a connection, thus an invalid packet (if the user enters a false password then this will return false because it didnt get added in the first part of this function)
        if self //Check if we have already established a connection with the client, if yes then it doesnt matter what password the user has entered
            .connected_clients
            .try_lock()
            .unwrap()
            .iter()
            .any(|client| client.uuid == req.uuid)
        //Search through the list
        {
            match &req.message_type {
                VoipConnection(request) => {
                    match request {
                        super::backend::ClientVoipRequest::Connect(port) => {
                            let socket_addr = SocketAddr::new(socket_addr.ip(), *port);

                            //Send important info to client (Session ID, etc)
                            send_message_to_client(
                                &mut *client_handle.try_lock()?,
                                encrypt_aes256(
                                    serde_json::to_string(&ServerVoipReply::Success)?,
                                    &self.decryption_key,
                                )?,
                            )
                            .await?;

                            if let Some(ongoing_call) = &self.voip {
                                ongoing_call.connect(req.uuid.clone(), socket_addr)?;
                            }
                            // If there is no ongoing call, we should create it
                            else {
                                let voip_server_instance =
                                    self.create_voip_server(self.opened_on_port.clone()).await?;

                                //Immediately connect the user who has requested the voip call
                                voip_server_instance.connect(req.uuid.clone(), socket_addr)?;

                                //Set voip server
                                self.voip = Some(voip_server_instance);
                            }

                            //We can safely assume its Some(_) here
                            if let Some(voip) = self.voip.as_mut() {
                                //Create handler thread
                                voip.threads.get_or_insert_with(|| {
                                    //Clone so we can move it into the thread
                                    let socket = voip.socket.clone();
                                    let connected_clients = voip.connected_client_thread_channels.clone();
                                    let cancellation_token = voip.thread_cancellation_token.clone();

                                    //Spawn manager thread
                                    tokio::spawn(async move {
                                        loop {
                                            //Create buffer for header, this is the size of the maximum udp packet so no error will appear
                                            let mut header_buf = vec![0; 65536];

                                            //Wait until we get a new message or until the thread token gets cancelled
                                            select! {
                                                //Wait until we receive a new message
                                                //Receive header size
                                                _ = socket.peek_from(&mut header_buf) => {
                                                    //Get message length
                                                    let header_length = u32::from_be_bytes(header_buf[..4].try_into().unwrap());

                                                    //Create body according to message size indicated by the header, make sure to add 4 to the byte length because we peeked the header thus we didnt remove the bytes from the buffer
                                                    let mut body_buf = vec![0; header_length as usize + 4];

                                                    match socket.recv_from(&mut body_buf).await {
                                                        Ok((_, socket_addr)) => {
                                                            match connected_clients.get(&socket_addr) {
                                                                Some(client) => {
                                                                    //We send everything from the 4th byte since that is the part of the header
                                                                    //We dont care about the result since it will panic when the thread is shut down
                                                                    let _ = client.0.send(body_buf[4..].to_vec()).await;
                                                                },
                                                                None => {
                                                                    tracing::error!("Client hasnt been added to the client connected list");
                                                                },
                                                            };
                                                        },

                                                        Err(err) => {
                                                            tracing::error!("{err}");
                                                        },
                                                    }
                                                }
                                                //Wait until the token gets cancelled
                                                _ = cancellation_token.cancelled() => {
                                                    //End loop once the token gets cancelled
                                                    break;
                                                }
                                            }
                                        }
                                    });
                                });

                                //Search if there is a channel for the handler thread of this connecting SocketAddr
                                if voip
                                    .connected_client_thread_channels
                                    .get(&socket_addr)
                                    .is_none()
                                {
                                    let (sender, receiver) = mpsc::channel::<Vec<u8>>(255);

                                    //Create cancellation token for client
                                    let client_manager_cancellation_token =
                                        CancellationToken::new();

                                    //Create voip manager for client
                                    create_client_voip_manager(
                                        voip.clone(),
                                        client_manager_cancellation_token.clone(),
                                        self.decryption_key,
                                        receiver,
                                        socket_addr,
                                        req.uuid.clone(),
                                    );

                                    voip.connected_client_thread_channels.insert(
                                        socket_addr,
                                        (Arc::new(sender), client_manager_cancellation_token),
                                    );
                                }
                            }

                            //Sync connected users with all users
                            sync_message_with_clients(
                                self.connected_clients.clone(),
                                self.clients_last_seen_index.clone(),
                                ServerOutput {
                                    replying_to: None,
                                    message_type: ServerMessageType::VoipState(ServerVoipState {
                                        connected_clients: Some(
                                            self.voip
                                                .as_ref()
                                                .unwrap()
                                                .connected_clients
                                                .iter()
                                                .map(|f| f.key().clone())
                                                .collect(),
                                        ),
                                    }),
                                    message_date: {
                                        Utc::now().format("%Y.%m.%d. %H:%M").to_string()
                                    },
                                    uuid: req.uuid.clone(),
                                    author: self
                                        .connected_clients_profile
                                        .lock()
                                        .await
                                        .get(&req.uuid)
                                        .unwrap()
                                        .username
                                        .clone(),
                                },
                                self.decryption_key,
                            )
                            .await?;
                        },
                        super::backend::ClientVoipRequest::Disconnect => {
                            if let Some(ongoing_voip) = self.voip.clone() {
                                //Get who disconnected
                                let connected_client =
                                    ongoing_voip.connected_clients.get(&req.uuid).ok_or_else(
                                        || Error::msg("Connected client not found based on UUID"),
                                    )?;

                                let socket_addr = connected_client.value();

                                let client_manager_thread = ongoing_voip.connected_client_thread_channels.get(socket_addr).ok_or_else(|| Error::msg("Client not found in connected client list based on SocketAddr"))?;

                                //Cancel client manager thread
                                client_manager_thread.1.cancel();

                                //Make sure to drop the reference so we will not deadlock upon calling ```voip.disconnect```
                                drop(connected_client);

                                //Blocks here
                                ongoing_voip.disconnect(req.uuid.clone())?;

                                if ongoing_voip.connected_clients.is_empty() {
                                    //If the voip has no connected clients we can shut down the whole service
                                    ongoing_voip.thread_cancellation_token.cancel();

                                    //Reset voip's state
                                    self.voip = None;
                                }

                                sync_message_with_clients(
                                    self.connected_clients.clone(),
                                    self.clients_last_seen_index.clone(),
                                    ServerOutput {
                                        replying_to: None,
                                        message_type: ServerMessageType::VoipState(
                                            ServerVoipState {
                                                connected_clients: {
                                                    //Match server Voip state
                                                    self.voip.as_ref().map(|server_voip| {
                                                        server_voip
                                                            .connected_clients
                                                            .iter()
                                                            .map(|entry| entry.key().clone())
                                                            .collect()
                                                    })
                                                },
                                            },
                                        ),
                                        message_date: {
                                            Utc::now().format("%Y.%m.%d. %H:%M").to_string()
                                        },
                                        uuid: req.uuid.clone(),
                                        author: String::new(),
                                    },
                                    self.decryption_key,
                                )
                                .await?;
                            }
                            else {
                                println!("Voip disconnected from an offline server")
                            }
                        },
                    }
                },

                NormalMessage(_msg) => self.normal_message(&req).await,

                SyncMessage(_msg) => {
                    self.sync_message(&req).await;
                },

                FileRequestType(request_type) => {
                    send_message_to_client(
                        &mut *client_handle.try_lock()?,
                        //Encrypt the request reply
                        encrypt_aes256(
                            self.handle_request(request_type).await?,
                            &self.decryption_key,
                        )
                        .unwrap(),
                    )
                    .await?;

                    return Ok(());
                },

                FileUpload(upload_type) => {
                    self.handle_upload(req.clone(), upload_type).await;
                },

                ClientReaction(reaction) => {
                    self.handle_reaction(reaction, &req).await;
                },

                MessageEdit(edit) => {
                    match &mut self.messages.try_lock() {
                        Ok(messages_vec) => {
                            //Server-side uuid check
                            if messages_vec[edit.index].uuid != req.uuid {
                                //Nice try :)
                                return Ok(());
                            }

                            //If its () then we can check for the index, because you can delete all messages, rest is ignored
                            if edit.new_message.is_none() {
                                //Set as `Deleted`
                                messages_vec[edit.index].message_type = ServerMessageType::Deleted;
                            }

                            if let ServerMessageType::Normal(inner_msg) =
                                &mut messages_vec[edit.index].message_type
                            {
                                if let Some(new_msg) = edit.new_message.clone() {
                                    inner_msg.message = new_msg;

                                    inner_msg.has_been_edited = true;
                                }
                            }
                        },
                        Err(err) => println!("{err}"),
                    };
                },
            };

            //We return the syncing function because after we have handled the request we return back the updated messages, which already contain the "side effects" of the client request
            //Please rework this, we should always be sending the latest message to all the clients so we are kept in sync, we only send all of them when we are connecting
            //We should send the incoming message to all of the clients, we are already storing the messages in self.messages
            sync_message_with_clients(
                self.connected_clients.clone(),
                self.clients_last_seen_index.clone(),
                ServerOutput::convert_clientmsg_to_servermsg(
                    req.clone(),
                    //Server file indexing, this is used as a handle for the client to ask files from the server
                    match &req.message_type {
                        VoipConnection(_) => String::new(),

                        //This is unreachable, as requests are handled elsewhere
                        FileRequestType(_) => unreachable!(),

                        FileUpload(inner) => sha256::digest(&inner.bytes),

                        //Some message types may not have a signature, they arent requested the same way as files
                        NormalMessage(_) => String::new(),

                        //Some message types may not have a signature, they arent requested the same way as files
                        SyncMessage(_) => String::new(),

                        //Some message types may not have a signature, they arent requested the same way as files
                        ClientReaction(_) => String::new(),

                        //Some message types may not have a signature, they arent requested the same way as files
                        MessageEdit(_) => String::new(),
                    },
                    //Get message type
                    match &req.message_type {
                        FileRequestType(_) => unreachable!(),
                        FileUpload(inner) => {
                            //We should match the upload type more specifically
                            match inner.extension.clone().unwrap_or_default().as_str() {
                                "png" | "jpeg" | "bmp" | "tiff" | "webp" => Image,
                                "wav" | "mp3" | "m4a" => Audio,
                                _ => Upload,
                            }
                        },
                        NormalMessage(_) => Normal,
                        SyncMessage(_) => Sync,
                        ClientReaction(_) => ServerMessageTypeDiscriminantReaction,
                        MessageEdit(_) => Edit,
                        VoipConnection(_) => Voip,
                    },
                    req.uuid.clone(),
                    self.connected_clients_profile
                        .lock()
                        .await
                        .get(&req.uuid)
                        .unwrap()
                        .clone()
                        .username,
                ),
                self.decryption_key,
            )
            .await
            .expect("Syncing failed");

            Ok(())
        }
        else {
            send_message_to_client(&mut *client_handle.try_lock()?, "Invalid Password!".into())
                .await?;

            Err(Error::msg("Invalid password entered by client!"))
        }
    }

    async fn create_voip_server(&self, port: String) -> anyhow::Result<ServerVoip>
    {
        // Create socket
        let socket = UdpSocket::bind(format!("[::]:{port}")).await?;

        //Return ServerVoip
        Ok(ServerVoip {
            connected_clients: Arc::new(DashMap::new()),
            _established_since: Utc::now(),
            socket: Arc::new(socket),
            thread_cancellation_token: CancellationToken::new(),
            threads: None,
            connected_client_thread_channels: Arc::new(DashMap::new()),
            message_buffer: Arc::new(DashMap::new()),
        })
    }

    async fn handle_server_disconnect(
        &self,
        client: &ConnectedClient,
        clients: &mut tokio::sync::MutexGuard<'_, Vec<ConnectedClient>>,
        index: usize,
    ) -> Result<ServerOutput, Error>
    {
        send_message_to_client(
            &mut *client.handle.clone().unwrap().lock().await,
            "Server disconnecting from client.".to_owned(),
        )
        .await?;

        clients.remove(index);

        let server_msg = ServerOutput {
            replying_to: None,
            message_type: ServerMessageType::Server(super::backend::ServerMessage::Disconnect(
                self.connected_clients_profile
                    .lock()
                    .await
                    .get(&client.uuid)
                    .unwrap()
                    .clone(),
            )),
            author: "Server".to_string(),
            message_date: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
            uuid: String::from("00000000-0000-0000-0000-000000000000"),
        };

        self.messages.lock().await.push(server_msg.clone());

        Ok(server_msg)
    }

    async fn handle_server_ban(
        &self,
        client: &ConnectedClient,
        clients: &mut tokio::sync::MutexGuard<'_, Vec<ConnectedClient>>,
        index: usize,
    ) -> Result<ServerOutput, Error>
    {
        let client_handle_clone = client.handle.clone().unwrap();

        let mut client_handle = &mut *client_handle_clone.lock().await;
        //Send ban message to client
        send_message_to_client(&mut client_handle, "You have been banned!".to_owned()).await?;

        //Signal disconnection
        send_message_to_client(
            &mut client_handle,
            "Server disconnecting from client.".to_owned(),
        )
        .await?;

        //Shutdown client connection
        client_handle.shutdown().await?;

        //Remove client
        clients.remove(index);

        let server_msg = ServerOutput {
            replying_to: None,
            message_type: ServerMessageType::Server(super::backend::ServerMessage::Ban(
                self.connected_clients_profile
                    .lock()
                    .await
                    .get(&client.uuid)
                    .unwrap()
                    .clone(),
            )),
            author: "Server".to_string(),
            message_date: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
            uuid: String::from("00000000-0000-0000-0000-000000000000"),
        };

        self.messages.lock().await.push(server_msg.clone());

        Ok(server_msg)
    }

    async fn handle_banned_uuid(
        &self,
        req: &ClientMessage,
        client_handle: &Arc<tokio::sync::Mutex<OwnedWriteHalf>>,
    ) -> Result<(), Error>
    {
        if let Some(idx) = self
            .shared_fields
            .lock()
            .await
            .banned_uuids
            .lock()
            .await
            .iter()
            .position(|item| *item == req.uuid)
        {
            let mut client_handle = &mut *client_handle.lock().await;

            send_message_to_client(&mut client_handle, "You have been banned!".to_string()).await?;

            self.connected_clients.lock().await.remove(idx);

            //Signal disconnection
            send_message_to_client(
                client_handle,
                "Server disconnecting from client.".to_owned(),
            )
            .await?;

            return Err(Error::msg("Client has been banned!"));
        };
        Ok(())
    }

    /// all the functions the server can do
    async fn normal_message(&self, req: &ClientMessage)
    {
        let mut messages = self.messages.lock().await;
        messages.push(ServerOutput::convert_clientmsg_to_servermsg(
            req.clone(),
            //Signatures for messages may be used later for something more useful
            String::new(),
            Normal,
            req.uuid.clone(),
            self.connected_clients_profile
                .lock()
                .await
                .get(&req.uuid)
                .unwrap()
                .clone()
                .username,
        ));
    }

    /// This function returns a message containing a full sync (all the messages etc)
    /// It returns a ```ServerMaster``` converted to an encrypted string
    async fn full_sync_client(&self) -> anyhow::Result<String>
    {
        //Construct reply
        let server_master = ServerMaster {
            //Return an empty message list
            message_list: self.messages.try_lock().unwrap().clone(),
            user_seen_list: self.clients_last_seen_index.try_lock().unwrap().clone(),
            reaction_list: (*self.reactions.try_lock().unwrap().clone()).to_vec(),
            connected_clients_profile: self.connected_clients_profile.try_lock().unwrap().clone(),
            ongoing_voip_call: {
                if let Some(voip) = &self.voip {
                    ServerVoipState {
                        connected_clients: Some(
                            voip.connected_clients
                                .iter()
                                .map(|entry| entry.key().clone())
                                .collect(),
                        ),
                    }
                }
                else {
                    ServerVoipState {
                        connected_clients: None,
                    }
                }
            },
        };

        //convert reply into string
        let final_msg: String = server_master.struct_into_string();

        //Encrypt string
        let encrypted_msg = encrypt_aes256(final_msg, &self.decryption_key).unwrap();

        //Reply with encrypted string
        Ok(encrypted_msg)
    }

    /// This function has a side effect on the user_seen_list, modifying it according to the client
    async fn sync_message(&self, req: &ClientMessage)
    {
        //Dont ask me why I did it this way
        if let SyncMessage(inner) = &req.message_type {
            //if its Some(_) then modify the list, the whole updated list will get sent back to the client regardless
            if let Some(last_seen_message_index) = inner.last_seen_message_index {
                match self.clients_last_seen_index.try_lock() {
                    Ok(mut client_vec) => {
                        //Iter over the whole list so we can update the user's index if there is one
                        if let Some(client_index_pos) =
                            client_vec.iter().position(|client| client.uuid == req.uuid)
                        {
                            //Update index
                            client_vec[client_index_pos].index = last_seen_message_index;
                        }
                        else {
                            client_vec.push(ClientLastSeenMessage::new(
                                last_seen_message_index,
                                req.uuid.clone(),
                            ));
                        }
                    },
                    Err(err) => {
                        tracing::error!("{}", err);
                    },
                }
            }
        };
    }
    async fn receive_file(&self, request: ClientMessage, req: &ClientFileUploadStruct)
    {
        //We should retrieve the username of the cient who has sent this, we clone it so that the mutex is dropped, thus allowing other threads to lock it
        let file_author = self
            .connected_clients_profile
            .lock()
            .await
            .get(&request.uuid)
            .unwrap()
            .clone()
            .username;

        //500mb limit
        if !req.bytes.len() > 500000000 {
            match env::var("APPDATA") {
                Ok(app_data) => {
                    //Get the signature of the file, and this is going to be the handle for this file
                    let file_hash = sha256::digest(&req.bytes);

                    //create file, add file to its named so it can never be mixed with images
                    match fs::File::create(format!(
                        "{app_data}\\Matthias\\Server\\{}.{}",
                        file_hash,
                        req.extension.clone().unwrap_or_default()
                    )) {
                        Ok(mut created_file) => {
                            if let Err(err) = created_file.write_all(&req.bytes) {
                                println!("[{err}\n{}]", err.kind());
                            };

                            created_file.flush().unwrap();
                            //success

                            self.file_list.insert(
                                file_hash.clone(),
                                PathBuf::from(format!(
                                    "{app_data}\\Matthias\\Server\\{}.{}",
                                    file_hash,
                                    req.extension.clone().unwrap_or_default()
                                )),
                            );

                            let mut messages = self.messages.lock().await;
                            messages.push(ServerOutput::convert_clientmsg_to_servermsg(
                                request.clone(),
                                file_hash,
                                Upload,
                                request.uuid.clone(),
                                file_author,
                            ));
                        },
                        Err(err) => {
                            println!(" [{err}\n{}]", err.kind());
                        },
                    }
                },
                Err(err) => {
                    println!("{err}")
                },
            }
        }
    }
    async fn serve_file(&self, signature: String) -> (Vec<u8>, PathBuf)
    {
        let path = self.file_list.get(&signature).unwrap().clone();
        (fs::read(&path).unwrap_or_default(), path)
    }
    async fn serve_image(&self, signature: String) -> Vec<u8>
    {
        fs::read(&*self.image_list.get(&signature).unwrap()).unwrap_or_default()
    }
    async fn receive_image(&self, req: ClientMessage, img: &ClientFileUploadStruct)
    {
        //We should retrieve the username of the cient who has sent this, we clone it so that the mutex is dropped, thus allowing other threads to lock it
        let file_author = self
            .connected_clients_profile
            .lock()
            .await
            .get(&req.uuid)
            .unwrap()
            .clone()
            .username;

        let file_signature = sha256::digest(img.bytes.clone());

        match env::var("APPDATA") {
            Ok(app_data) => {
                match fs::File::create(format!("{app_data}\\Matthias\\Server\\{}", file_signature))
                {
                    Ok(mut created_file) => {
                        if let Err(err) = created_file.write_all(&img.bytes) {
                            println!("[{err}\n{}]", err.kind());
                        };

                        created_file.flush().unwrap();
                        //success

                        match self.messages.try_lock() {
                            Ok(mut ok) => {
                                ok.push(ServerOutput::convert_clientmsg_to_servermsg(
                                    req.clone(),
                                    file_signature.clone(),
                                    Image,
                                    req.uuid.clone(),
                                    file_author,
                                ));
                            },
                            Err(err) => println!("{err}"),
                        }

                        //Only save as last step to avoid a mismatch + correct indexing :)
                        self.image_list.insert(
                            file_signature.clone(),
                            PathBuf::from(format!(
                                "{app_data}\\Matthias\\Server\\{}",
                                file_signature
                            )),
                        );
                    },
                    Err(err) => {
                        println!(" [{err} {}]", err.kind());
                    },
                }
            },
            Err(err) => {
                println!("{err}")
            },
        }
    }
    async fn receive_audio(&self, req: ClientMessage, audio: &ClientFileUploadStruct)
    {
        //We should retrieve the username of the cient who has sent this, we clone it so that the mutex is dropped, thus allowing other threads to lock it
        let file_author = self
            .connected_clients_profile
            .lock()
            .await
            .get(&req.uuid)
            .unwrap()
            .clone()
            .username;

        let audio_paths = self.audio_list.clone();

        let file_signature = sha256::digest(audio.bytes.clone());

        match fs::File::create(format!(
            "{}\\Matthias\\Server\\{}",
            env!("APPDATA"),
            file_signature
        )) {
            Ok(mut created_file) => {
                if let Err(err) = created_file.write_all(&audio.bytes) {
                    println!("[{err}\n{}]", err.kind());
                };

                created_file.flush().unwrap();
                //success

                match self.messages.try_lock() {
                    Ok(mut ok) => {
                        ok.push(ServerOutput::convert_clientmsg_to_servermsg(
                            req.clone(),
                            file_signature.clone(),
                            Audio,
                            req.uuid.clone(),
                            file_author,
                        ));
                    },
                    Err(err) => println!("{err}"),
                }

                //Only save as last step to avoid a mismatch + correct indexing :)
                audio_paths.insert(
                    file_signature.clone(),
                    PathBuf::from(format!(
                        "{}\\Matthias\\Server\\{}",
                        env!("APPDATA"),
                        file_signature
                    )),
                );

                //consequently save the audio_recording's name
                self.audio_names.insert(file_signature, audio.name.clone());
            },
            Err(err) => {
                println!(" [{err} {}]", err.kind());
            },
        }
    }
    async fn serve_audio(&self, signature: String) -> (Vec<u8>, Option<String>)
    {
        (
            fs::read(&*self.audio_list.get(&signature).unwrap()).unwrap_or_default(),
            self.audio_names.get(&signature).unwrap().clone(),
        )
    }

    /// used to handle all the requests, route the user's request
    #[inline]
    pub async fn handle_request(
        &self,
        request_type: &ClientRequestTypeStruct,
    ) -> anyhow::Result<String>
    {
        let reply = match request_type {
            ClientRequestTypeStruct::ImageRequest(img_request) => {
                let read_file = self.serve_image(img_request.signature.clone()).await;

                serde_json::to_string(&ServerReplyType::Image(ServerImageReply {
                    bytes: read_file,
                    signature: img_request.signature.clone(),
                }))
                .unwrap_or_default()
            },
            ClientRequestTypeStruct::FileRequest(file_request) => {
                let (file_bytes, file_name) =
                    &self.serve_file(file_request.signature.clone()).await;

                serde_json::to_string(&ServerReplyType::File(ServerFileReply {
                    file_name: file_name.clone(),
                    bytes: file_bytes.clone(),
                }))
                .unwrap_or_default()
            },
            ClientRequestTypeStruct::AudioRequest(audio_request) => {
                let (file_bytes, file_name) =
                    self.serve_audio(audio_request.signature.clone()).await;

                serde_json::to_string(&ServerReplyType::Audio(ServerAudioReply {
                    bytes: file_bytes,
                    signature: audio_request.signature.clone(),
                    file_name: file_name.unwrap_or_default(),
                }))
                .unwrap_or_default()
            },
            ClientRequestTypeStruct::ClientRequest(client_request_uuid) => {
                let connected_clients = self.connected_clients_profile.try_lock().unwrap();

                let client = connected_clients.get(client_request_uuid).unwrap();

                serde_json::to_string(&ServerReplyType::Client(ServerClientReply {
                    uuid: client_request_uuid.to_string(),
                    profile: client.clone(),
                }))
                .unwrap_or_default()
            },
        };

        Ok(reply)
    }

    /// handle all the file uploads
    pub async fn handle_upload(&self, req: ClientMessage, upload_type: &ClientFileUploadStruct)
    {
        //Create server folder, so we will have a place to put our uploads
        let _ = fs::create_dir(format!("{}\\matthias\\Server", env!("APPDATA")));

        //Pattern match on upload type so we know how to handle the specific request
        match upload_type.extension.clone().unwrap_or_default().as_str() {
            "png" | "jpeg" | "bmp" | "tiff" | "webp" => self.receive_image(req, upload_type).await,
            "wav" | "mp3" | "m4a" => self.receive_audio(req, upload_type).await,
            //Define file types and how should the server handle them based on extension, NOTICE: ENSURE CLIENT COMPATIBILITY
            _ => self.receive_file(req, upload_type).await,
        }
    }

    /// handle reaction requests
    pub async fn handle_reaction(&self, reaction: &ReactionType, req: &ClientMessage)
    {
        match reaction {
            ReactionType::Add(reaction) => {
                match &mut self.reactions.try_lock() {
                    Ok(reaction_vec) => {
                        //Borrow as mutable so we dont have to clone
                        for item in reaction_vec[reaction.message_index]
                            .message_reactions
                            .iter_mut()
                        {
                            //Check if it has already been reacted before, if yes add one to the counter
                            if item.emoji_name == reaction.emoji_name {
                                item.authors.push(req.uuid.clone());

                                //Quit the function immediately, so we can add the new reaction
                                return;
                            }
                        }

                        //After we have checked all the reactions if there is already one, we can add out *new* one
                        reaction_vec[reaction.message_index]
                            .message_reactions
                            .push(Reaction {
                                emoji_name: reaction.emoji_name.clone(),
                                authors: vec![req.uuid.clone()],
                            });
                    },
                    Err(err) => println!("{err}"),
                }
            },
            ReactionType::Remove(reaction) => {
                match &mut self.reactions.try_lock() {
                    Ok(reaction_vec) => {
                        let mut was_last_rection = false;

                        //Borrow as mutable so we dont have to clone
                        for item in reaction_vec[reaction.message_index]
                            .message_reactions
                            .iter_mut()
                        {
                            //Check if it has already been reacted before, if yes add one to the counter
                            if item.emoji_name == reaction.emoji_name {
                                match item.authors.iter().position(|uuid| **uuid == req.uuid) {
                                    Some(idx) => {
                                        item.authors.remove(idx);
                                    },
                                    None => {
                                        tracing::error!(
                                            "Tried to remove a non-author from the authors list."
                                        );
                                    },
                                }

                                //Check if the item.times is 0 that means we removed the last reaction
                                //If yes, set flag
                                if item.authors.is_empty() {
                                    was_last_rection = true;
                                }
                            }
                        }

                        //Check if we removed the last emoji, if yes remove the whole emoji entry
                        if was_last_rection {
                            match reaction_vec[reaction.message_index]
                                .message_reactions
                                .clone()
                                .get(reaction.message_index)
                            {
                                Some(_) => {
                                    reaction_vec[reaction.message_index]
                                        .message_reactions
                                        .remove(reaction.message_index);
                                },
                                None => {
                                    tracing::error!("The emoji requested to be removed was not in the emoji list");
                                },
                            }
                        }
                    },
                    Err(err) => println!("{err}"),
                }
            },
        }
    }
}
