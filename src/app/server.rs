use std::{
    collections::HashMap, env, fs, io::Write, net::SocketAddr, path::PathBuf, sync::Arc,
    time::Duration,
};

use anyhow::{Error, Result};
use chrono::Utc;
use dashmap::DashMap;
use egui::Context;
use tokio_util::sync::CancellationToken;

use super::backend::{
    encrypt, encrypt_aes256, fetch_incoming_message_lenght, ClientLastSeenMessage,
    ClientMessageType, ClientProfile, ConnectedClient, ConnectionType, MessageReaction, Reaction,
    ServerClientReply, ServerMessageType,
    ServerMessageTypeDiscriminants::{
        Audio, Edit, Image, Normal, Reaction as ServerMessageTypeDiscriminantReaction, Sync,
        Upload, VoipConnection as Voip,
    },
    ServerReplyType, ServerSync, ServerVoip, ServerVoipAuthenticate, ServerVoipReply,
};

use crate::app::backend::ServerMaster;
use crate::app::backend::{
    ClientFileRequestType as ClientRequestTypeStruct, ClientFileUpload as ClientFileUploadStruct,
    ClientMessage,
    ClientMessageType::{
        FileRequestType, FileUpload, MessageEdit, NormalMessage, Reaction as ClientReaction,
        SyncMessage, VoipConnection,
    },
    ClientReaction as ClientReactionStruct, ServerFileReply, ServerImageReply,
};
use tokio::{
    io::AsyncWrite,
    net::{tcp::OwnedReadHalf, UdpSocket},
    select,
    task::JoinHandle,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{self, tcp::OwnedWriteHalf},
};

use super::backend::{ServerAudioReply, ServerOutput};

#[derive(Debug, Default)]
pub struct MessageService {
    /// Contains all the messages
    pub messages: Arc<tokio::sync::Mutex<Vec<ServerOutput>>>,

    /// Contains all of the reactions added to the messages
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
pub struct SharedFields {
    /// This list contains the banned uuids
    pub banned_uuids: Arc<tokio::sync::Mutex<Vec<String>>>,
}

/// Shutting down server also doesnt work we will have to figure a way out on how to stop client readers (probably a broadcast channel)
pub async fn server_main(
    port: String,
    password: String,
    //This signals all the client recivers to be shut down
    cancellation_token: CancellationToken,
    connected_clients_profile_list: Arc<DashMap<String, ClientProfile>>,
    //We pass in ctx so we can request repaint when someone connects
    ctx: Context,
) -> Result<Arc<tokio::sync::Mutex<SharedFields>>, Box<dyn std::error::Error>> {
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

            //split client stream, so we will be able to store these seperately
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
) {
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

                msg = recive_message(reader.clone()) => {
                    msg?
                }
            };

            let mut message_service = msg_service.lock().await;

            match message_service
                .message_main(incoming_message, writer.clone(), socket_addr)
                .await
            {
                Ok(_) => {}
                Err(err) => {
                    println!("Error processing a message: {err}");
                    break;
                }
            }
        }
        Ok(())
    });
}

#[inline]
async fn recive_message(reader: Arc<tokio::sync::Mutex<OwnedReadHalf>>) -> Result<String> {
    let mut reader = reader.lock().await;

    let incoming_message_len = fetch_incoming_message_lenght(&mut *reader).await?;

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
) -> anyhow::Result<()> {
    let mut connected_clients_locked = connected_clients.lock().await;

    let server_master = ServerSync {
        message,
        user_seen_list: user_seen_list.lock().await.to_vec(),
    };

    let server_master_string = server_master.struct_into_string();

    //Encrypt string
    let encrypted_string = encrypt_aes256(server_master_string, &key).unwrap();

    //Send message lenght
    let message_lenght = TryInto::<u32>::try_into(encrypted_string.as_bytes().len())?;

    for client in connected_clients_locked.iter_mut() {
        if let Some(client_handle) = &mut client.handle {
            let mut client_handle = client_handle.lock().await;

            client_handle
                .write_all(&message_lenght.to_be_bytes())
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

    //Send message lenght
    writer
        .write_all(&(message_bytes.len() as u32).to_be_bytes())
        .await?;

    //Send message
    writer.write_all(message_bytes).await?;

    Ok(())
}

impl MessageService {
    /// The result returned by this function may be a real error, or an error constructed on purpose so that the thread call this function gets shut down.
    /// When experiening errors, make sure to check the error message as it may be on purpose
    #[inline]
    async fn message_main(
        &mut self,
        message: String,
        client_handle: Arc<tokio::sync::Mutex<OwnedWriteHalf>>,
        socket_addr: SocketAddr,
    ) -> Result<()> {
        let req_result: Result<ClientMessage, serde_json::Error> = serde_json::from_str(&message);

        let req: ClientMessage = req_result.unwrap();

        //If its a Client reaction or a message edit we shouldnt allocate more MessageReactions, since those are not actual messages
        //HOWEVER, if theyre client connection or disconnection messages a reaction should be allocated because people can react to those
        if !(matches!(&req.message_type, ClientReaction(_))
            || matches!(&req.message_type, MessageEdit(_))
            || {
                if let ClientMessageType::SyncMessage(sync_msg) = &req.message_type {
                    //If this is true (if sync_attribute is none) that means the client is syncing its last seen message index, thefor we shouldnt allocate a new reaction
                    sync_msg.sync_attribute.is_none()
                } else {
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
                }
                Err(err) => {
                    println!("{err}")
                }
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
                            } else {
                                let mut clients = self.connected_clients.lock().await;
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
                                    author: "Server".to_string(),
                                    message_date: {
                                        Utc::now().format("%Y.%m.%d. %H:%M").to_string()
                                    },
                                    uuid: String::from("00000000-0000-0000-0000-000000000000"),
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
                        }
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
                        }
                    }
                }
            } else {
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
                        super::backend::ClientVoipRequest::Connect => {
                            //
                            //Authenticate if needed
                            //

                            if let Some(ongoing_call) = &self.voip {
                                ongoing_call.connect(req.uuid.clone(), socket_addr)?;
                            }
                            // If there is no ongoing call, we should create it
                            else {
                                let voip_server =
                                    self.create_voip_server(self.opened_on_port.clone()).await?;

                                //Immediately connect the user who has requested the voip call
                                voip_server.connect(req.uuid.clone(), socket_addr)?;

                                //Set voip server
                                self.voip = Some(voip_server);
                            }

                            //Send important ifno to client (Session ID, etc)
                            send_message_to_client(
                                &mut *client_handle.try_lock()?,
                                encrypt_aes256(
                                    serde_json::to_string(&ServerVoipReply::Success(
                                        //Create session ID
                                        ServerVoipAuthenticate::new()?,
                                    ))?,
                                    &self.decryption_key,
                                )?,
                            )
                            .await?;
                        }
                        super::backend::ClientVoipRequest::Disconnect => {
                            if let Some(ongoing_voip) = &self.voip {
                                ongoing_voip.disconnect(req.uuid.clone())?;
                            }
                        }
                    }
                }
                NormalMessage(_msg) => self.normal_message(&req).await,

                SyncMessage(_msg) => {
                    self.sync_message(&req).await;
                }

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
                }

                FileUpload(upload_type) => {
                    self.handle_upload(req.clone(), upload_type).await;
                }

                ClientReaction(reaction) => {
                    self.handle_reaction(reaction).await;
                }

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
                        }
                        Err(err) => println!("{err}"),
                    };
                }
            };

            //We return the syncing function because after we have handled the request we return back the updated messages, which already contain the "side effects" of the client request
            //Please rework this, we should always be sending the latest message to all the clients so we are kept in sync, we only send all of them when we are connecting
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
                            //We should match the upload type more specificly
                            match inner.extension.clone().unwrap_or_default().as_str() {
                                "png" | "jpeg" | "bmp" | "tiff" | "webp" => Image,
                                "wav" | "mp3" | "m4a" => Audio,
                                _ => Upload,
                            }
                        }
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

            //We should send the incoming message to all of the clients, we are already storing the messages in self.messages
            Ok(())
        } else {
            send_message_to_client(&mut *client_handle.try_lock()?, "Invalid Password!".into())
                .await?;

            Err(Error::msg("Invalid password entered by client!"))
        }
    }

    async fn create_voip_server(&self, port: String) -> anyhow::Result<ServerVoip> {
        // Create socket
        let socket = UdpSocket::bind(format!("[::1]:{port}")).await?;

        //Return ServerVoip
        Ok(ServerVoip {
            connected_clients: Arc::new(DashMap::new()),
            established_since: Utc::now(),
            socket: Arc::new(socket),
        })
    }

    async fn handle_server_disconnect(
        &self,
        client: &ConnectedClient,
        clients: &mut tokio::sync::MutexGuard<'_, Vec<ConnectedClient>>,
        index: usize,
    ) -> Result<ServerOutput, Error> {
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
    ) -> Result<ServerOutput, Error> {
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
    ) -> Result<(), Error> {
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
    async fn normal_message(&self, req: &ClientMessage) {
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
    /// It reutrns a ```ServerMaster``` converted to an encrypted string
    async fn full_sync_client(&self) -> anyhow::Result<String> {
        //Construct reply
        let server_master = ServerMaster {
            //Return an empty message list
            message_list: self.messages.try_lock().unwrap().clone(),
            user_seen_list: self.clients_last_seen_index.try_lock().unwrap().clone(),
            reaction_list: (*self.reactions.try_lock().unwrap().clone()).to_vec(),
            connected_clients_profile: self.connected_clients_profile.try_lock().unwrap().clone(),
        };

        //convert reply into string
        let final_msg: String = server_master.struct_into_string();

        //Encrypt string
        let encrypted_msg = encrypt_aes256(final_msg, &self.decryption_key).unwrap();

        //Reply with encrypted string
        Ok(encrypted_msg)
    }

    /// This function has a side effect on the user_seen_list, modifying it according to the client
    async fn sync_message(&self, req: &ClientMessage) {
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
                        } else {
                            client_vec.push(ClientLastSeenMessage::new(
                                last_seen_message_index,
                                req.uuid.clone(),
                            ));
                        }
                    }
                    Err(err) => {
                        dbg!(err);
                    }
                }
            }
        };
    }
    async fn recive_file(&self, request: ClientMessage, req: &ClientFileUploadStruct) {
        //We should retrive the username of the cient who has sent this, we clone it so that the mutex is dropped, thus allowing other threads to lock it
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
                        }
                        Err(err) => {
                            println!(" [{err}\n{}]", err.kind());
                        }
                    }
                }
                Err(err) => {
                    println!("{err}")
                }
            }
        }
    }
    async fn serve_file(&self, signature: String) -> (Vec<u8>, PathBuf) {
        let path = self.file_list.get(&signature).unwrap().clone();
        (fs::read(&path).unwrap_or_default(), path)
    }
    async fn serve_image(&self, signature: String) -> Vec<u8> {
        fs::read(&*self.image_list.get(&signature).unwrap()).unwrap_or_default()
    }
    async fn recive_image(&self, req: ClientMessage, img: &ClientFileUploadStruct) {
        //We should retrive the username of the cient who has sent this, we clone it so that the mutex is dropped, thus allowing other threads to lock it
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
                            }
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
                    }
                    Err(err) => {
                        println!(" [{err} {}]", err.kind());
                    }
                }
            }
            Err(err) => {
                println!("{err}")
            }
        }
    }
    async fn recive_audio(&self, req: ClientMessage, audio: &ClientFileUploadStruct) {
        //We should retrive the username of the cient who has sent this, we clone it so that the mutex is dropped, thus allowing other threads to lock it
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
                    }
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
            }
            Err(err) => {
                println!(" [{err} {}]", err.kind());
            }
        }
    }
    async fn serve_audio(&self, signature: String) -> (Vec<u8>, Option<String>) {
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
    ) -> anyhow::Result<String> {
        let reply = match request_type {
            ClientRequestTypeStruct::ImageRequest(img_request) => {
                let read_file = self.serve_image(img_request.signature.clone()).await;

                serde_json::to_string(&ServerReplyType::Image(ServerImageReply {
                    bytes: read_file,
                    signature: img_request.signature.clone(),
                }))
                .unwrap_or_default()
            }
            ClientRequestTypeStruct::FileRequest(file_request) => {
                let (file_bytes, file_name) =
                    &self.serve_file(file_request.signature.clone()).await;

                serde_json::to_string(&ServerReplyType::File(ServerFileReply {
                    file_name: file_name.clone(),
                    bytes: file_bytes.clone(),
                }))
                .unwrap_or_default()
            }
            ClientRequestTypeStruct::AudioRequest(audio_request) => {
                let (file_bytes, file_name) =
                    self.serve_audio(audio_request.signature.clone()).await;

                serde_json::to_string(&ServerReplyType::Audio(ServerAudioReply {
                    bytes: file_bytes,
                    signature: audio_request.signature.clone(),
                    file_name: file_name.unwrap_or_default(),
                }))
                .unwrap_or_default()
            }
            ClientRequestTypeStruct::ClientRequest(client_request_uuid) => {
                let connected_clients = self.connected_clients_profile.try_lock().unwrap();

                let client = connected_clients.get(client_request_uuid).unwrap();

                serde_json::to_string(&ServerReplyType::Client(ServerClientReply {
                    uuid: client_request_uuid.to_string(),
                    profile: client.clone(),
                }))
                .unwrap_or_default()
            }
        };

        Ok(reply)
    }

    /// handle all the file uploads
    pub async fn handle_upload(&self, req: ClientMessage, upload_type: &ClientFileUploadStruct) {
        //Create server folder, so we will have a place to put our uploads
        let _ = fs::create_dir(format!("{}\\matthias\\Server", env!("APPDATA")));

        //Pattern match on upload tpye so we know how to handle the specific request
        match upload_type.extension.clone().unwrap_or_default().as_str() {
            "png" | "jpeg" | "bmp" | "tiff" | "webp" => self.recive_image(req, upload_type).await,
            "wav" | "mp3" | "m4a" => self.recive_audio(req, upload_type).await,
            //Define file types and how should the server handle them based on extension, NOTICE: ENSURE CLIENT COMPATIBILITY
            _ => self.recive_file(req, upload_type).await,
        }
    }

    /// handle reaction requests
    pub async fn handle_reaction(&self, reaction: &ClientReactionStruct) {
        match &mut self.reactions.try_lock() {
            Ok(reaction_vec) => {
                //Borrow as mutable so we dont have to clone
                for item in reaction_vec[reaction.message_index]
                    .message_reactions
                    .iter_mut()
                {
                    //Check if it has already been reacted before, if yes add one to the counter
                    if item.emoji_name == reaction.emoji_name {
                        item.times += 1;

                        //Quit the function immediately, so we can add the new reaction
                        return;
                    }
                }

                //After we have checked all the reactions if there is already one, we can add out *new* one
                reaction_vec[reaction.message_index]
                    .message_reactions
                    .push(Reaction {
                        emoji_name: reaction.emoji_name.clone(),
                        //Set default amount, start from 1
                        times: 1,
                    });
            }
            Err(err) => println!("{err}"),
        }
    }
}
