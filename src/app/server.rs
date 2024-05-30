use std::{env, fs, io::Write, path::PathBuf, sync::Arc};

use anyhow::{Error, Result};

use super::backend::{
    encrypt_aes256, fetch_incoming_message_lenght, ClientLastSeenMessage, ClientMessageType,
    ConnectedClient, MessageReaction, Reaction, ServerMessageType,
    ServerMessageTypeDiscriminants::{
        self, Audio, Edit, Image, Normal, Reaction as ServerMessageTypeDiscriminantReaction, Upload,
    },
};

use rand::Rng;
use std::sync::Mutex;
use tokio::{
    io::AsyncWrite,
    net::tcp::OwnedReadHalf,
    sync::{broadcast, mpsc::Receiver},
};

use crate::app::backend::ServerMaster;
use crate::app::backend::{
    ClientFileRequestType as ClientRequestTypeStruct, ClientFileUpload as ClientFileUploadStruct,
    ClientMessage, ClientMessageEdit as ClientMessageEditStruct,
    ClientMessageType::{
        ClientFileRequestType, ClientFileUpload, ClientMessageEdit, ClientNormalMessage,
        ClientReaction, ClientSyncMessage,
    },
    ClientReaction as ClientReactionStruct, ServerFileReply, ServerImageReply,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{self, tcp::OwnedWriteHalf},
};

use super::backend::{ServerAudioReply, ServerOutput};

#[derive(Debug, Default)]
pub struct MessageService {
    ///Contains all the messages
    pub messages: Arc<tokio::sync::Mutex<Vec<ServerOutput>>>,

    ///Contains all of the reactions added to the messages
    pub reactions: Arc<tokio::sync::Mutex<Vec<MessageReaction>>>,

    ///This is the required password by the server
    pub passw: String,

    /// This is the list, which we will send the files from, these are generated file names, so names will rarely ever match (1 / 1.8446744e+19) chance
    /// The names are not relevant since when downloading them the client will always ask for a new name
    pub generated_file_paths: Mutex<Vec<PathBuf>>,

    /// This list contains a list of the path to the stored images
    /// When the client is asking for a file, they provide an index (which we provided originally when syncing, aka sending the latest message to all the clients)
    pub image_list: Mutex<Vec<PathBuf>>,

    ///This list contains a list of the path to the stored audio files
    /// When the client is asking for a file, they provide an index (which we provided originally when syncing, aka sending the latest message to all the clients)
    pub audio_list: Mutex<Vec<PathBuf>>,

    /// This list contains the names of the saved audios, since we generate a random name for the files we want to store
    /// We also dont ask the user the provide a name whenever playing an audio (requesting it from the server)
    pub audio_names: Mutex<Vec<Option<String>>>,

    ///connected clients
    pub connected_clients: Arc<tokio::sync::Mutex<Vec<ConnectedClient>>>,

    ///Client secret
    pub decryption_key: [u8; 32],

    ///Client last seen message
    pub clients_last_seen_index: Arc<tokio::sync::Mutex<Vec<ClientLastSeenMessage>>>,
}

/// Shutting down server also doesnt work we will have to figure a way out on how to stop client readers (probably a broadcast channel)
pub async fn server_main(
    port: String,
    password: String,
    mut signal: Receiver<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    //Start listening
    let tcp_listener = net::TcpListener::bind(format!("[::]:{}", port)).await?;

    //Server default information
    let msg_service = Arc::new(tokio::sync::Mutex::new(MessageService {
        passw: password,
        decryption_key: rand::random::<[u8; 32]>(),
        ..Default::default()
    }));

    let (thread_sender, thread_reciver) = broadcast::channel::<()>(1);

    //Server thread
    let _: tokio::task::JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        loop {
            //check if the server is supposed to run
            if signal.try_recv().is_ok() {
                //send a shutdown signal to all of the client recivers using the broadcast channel
                thread_sender.send(())?;

                //shutdown server
                break;
            }

            //accept connection
            let (stream, _address) = tcp_listener.accept().await?;

            //split client stream, so we will be able to store these seperately
            let (reader, writer) = stream.into_split();

            let msg_service_clone = msg_service.clone();

            //Listen for future client messages (IF the client stays connected)
            spawn_client_reader(
                Arc::new(tokio::sync::Mutex::new(reader)),
                Arc::new(tokio::sync::Mutex::new(writer)),
                msg_service_clone,
                thread_reciver.resubscribe(),
            );
        }
        Ok(())
    });

    Ok(())
}

/// This function does not need to be async since it spawn an async thread anyway
/// Spawn reader thread, this will constantly listen to the client which was connected, this thread will only finish if the client disconnects
#[inline]
fn spawn_client_reader(
    reader: Arc<tokio::sync::Mutex<OwnedReadHalf>>,
    writer: Arc<tokio::sync::Mutex<OwnedWriteHalf>>,
    msg_service: Arc<tokio::sync::Mutex<MessageService>>,
    mut thread_reciver: broadcast::Receiver<()>,
) {
    let _: tokio::task::JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        loop {
            //Check if the thread needs to be shut down
            if thread_reciver.try_recv().is_ok() {
                //MAybe we should implement sending a disconnection msg to all of the clients
                break;
            }

            let message_service = msg_service.lock().await;

            //the thread will block here waiting for client message, problem appears here
            let incoming_message = recive_message(reader.clone()).await?;

            message_service
                .message_main(incoming_message, writer.clone())
                .await?;

            // Send it to all the other clients,
            // the message_main function modifes the messages list
            // let mut messages = message_service.messages.lock().await;
            //This will block until it could reply to all fot he clients
            //If there is an incoming message we should reply to all of the clients, after processing said message
            // sync_all_messages_with_all_clients(
            //     message_service.connected_clients.clone(),
            //     message_service.messages.clone(),
            //     message_service.reactions.clone(),
            //     message_service.clients_last_seen_index.clone(),
            // )
            // .await?;
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
    connected_clients: Arc<tokio::sync::Mutex<Vec<ConnectedClient>>>,
    message: Vec<ServerOutput>,
) -> anyhow::Result<()> {
    let mut connected_clients = connected_clients.try_lock()?;

    let server_master = ServerMaster {
        struct_list: message,
        user_seen_list: vec![],
        reaction_list: vec![],
        auto_sync_attributes: None
    };

    let server_master_string = server_master.struct_into_string();

    //Send message lenght
    let message_lenght = TryInto::<u32>::try_into(server_master_string.as_bytes().len())?;

    for client in connected_clients.iter_mut() {
        if let Some(client_handle) = &mut client.handle {
            let mut client_handle = client_handle.lock().await;

            client_handle
                .write_all(&message_lenght.to_be_bytes())
                .await?;

            //Send actual message
            client_handle
                .write_all(server_master_string.as_bytes())
                .await?;

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
        &self,
        message: String,
        client_handle: Arc<tokio::sync::Mutex<OwnedWriteHalf>>,
    ) -> Result<()> {
        let req_result: Result<ClientMessage, serde_json::Error> =
            dbg!(serde_json::from_str(&message));
        let mut client_buffer = client_handle.try_lock()?;

        let req: ClientMessage = req_result.unwrap();

        if let ClientMessageType::ClientSyncMessage(sync_msg) = &req.MessageType {
            if sync_msg.password == self.passw.trim() {
                //Handle incoming connections and disconnections, if inbound_connection_address is some, else we assume its for syncing *ONLY*
                if let Some(sync_attr) = sync_msg.sync_attribute {
                    //sync attr is true if its a connection message i.e a licnet is trying to connect to us
                    if sync_attr {
                        match self.connected_clients.try_lock() {
                            Ok(mut clients) => {
                                //Search for connected ip in all connected ips
                                for client in clients.iter() {
                                    //If found, then the client is already connected
                                    if client.uuid == req.Uuid {
                                        //This can only happen if the connection closed unexpectedly (If the client was stopped unexpectedly)
                                        send_message_to_client(
                                            &mut *client_buffer,
                                            hex::encode(self.decryption_key),
                                        )
                                        .await?;

                                        //If found return, and end execution
                                        return Ok(());
                                    }
                                }

                                //If the ip is not found then add it to connected clients
                                clients.push(ConnectedClient::new(
                                    req.Uuid.clone(),
                                    req.Author.clone(),
                                    client_handle.clone(),
                                ));

                                //Return custom key which the server's text will be encrypted with
                                send_message_to_client(
                                    &mut *client_buffer,
                                    hex::encode(self.decryption_key),
                                )
                                .await?;

                                return Ok(());
                            }
                            Err(err) => {
                                dbg!(err);
                            }
                        }
                    }
                    //Handle disconnections
                    else {
                        match self.connected_clients.try_lock() {
                            Ok(mut clients) => {
                                //Search for connected ip in all connected ips
                                for (index, client) in clients.clone().iter().enumerate() {
                                    //If found, then disconnect the client
                                    if client.uuid == req.Uuid {
                                        clients.remove(index);

                                        //Break out of the loop, return an error so the client listener thread stops
                                        return Err(Error::msg("Client disconnected!"));
                                    }
                                }
                            }
                            Err(err) => {
                                dbg!(err);
                            }
                        }
                    }
                }

                //Sync all messages, send all of the messages to the client
                // send_message_to_client(&mut *client_buffer, self.sync_message(&req).await?).await?;
                return Ok(());
            } else {
                send_message_to_client(&mut *client_buffer, "Invalid Password!".into()).await?;

                //return an error so the client listener thread stops
                return Err(Error::msg("Invalid password entered by client!"));
            }
        }

        //if the client is not found in the list means we have not established a connection, thus an invalid packet (if the user enters a false password then this will return false because it didnt get added in the first part of this function)
        if self //Check if we have already established a connection with the client, if yes then it doesnt matter what password the user has entered
            .connected_clients
            .try_lock()
            .unwrap()
            .iter()
            .any(|client| client.uuid == req.Uuid)
        //Search through the list
        {
            match &req.MessageType {
                ClientNormalMessage(_msg) => self.NormalMessage(&req).await,

                ClientSyncMessage(_msg) => {
                    unimplemented!("How the fuck did you get here?");
                }

                ClientFileRequestType(request_type) => {
                    send_message_to_client(
                        &mut *client_buffer,
                        self.handle_request(request_type).await?,
                    )
                    .await?;

                    return Ok(());
                }

                ClientFileUpload(upload_type) => {
                    self.handle_upload(req.clone(), upload_type).await;
                }

                ClientReaction(reaction) => {
                    self.handle_reaction(reaction).await;
                }

                ClientMessageEdit(edit) => {
                    self.handle_message_edit(edit, &req).await;
                }
            };

            //If its a Client reaction or a message edit we shouldnt allocate more MessageReactions, since those are not actually messages
            if !(matches!(&req.MessageType, ClientReaction(_))
                || matches!(&req.MessageType, ClientMessageEdit(_)))
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

            //We return the syncing function because after we have handled the request we return back the updated messages, which already contain the "side effects" of the client request
            //Please rework this, we should always be sending the latest message to all the clients so we are kept in sync, we only send all of them when we are connecting
            sync_message_with_clients(
                self.connected_clients.clone(),
                vec![ServerOutput::convert_type_to_servermsg(
                    req.clone(),
                    //Server file indexing, this is used as a handle for the client to ask files from the server
                    match &req.MessageType {
                        //This is unreachable, as requests are handled elsewhere
                        ClientFileRequestType(_) => unreachable!(),

                        ClientFileUpload(_) => {
                            (self.generated_file_paths.lock().unwrap().len() - 1) as i32
                        }

                        ClientNormalMessage(_) => -1,

                        ClientSyncMessage(_) => panic!("What the fuck"),

                        //The client will update their own message
                        ClientReaction(_) => -1,
                        //The client will update their own message
                        ClientMessageEdit(_) => -1,
                    },
                    //Get message type
                    match &req.MessageType {
                        ClientFileRequestType(_) => unreachable!(),
                        ClientFileUpload(_) => Upload,
                        ClientNormalMessage(_) => Normal,
                        ClientSyncMessage(_) => unreachable!("Ezt gondold át geci"),
                        ClientReaction(_) => ServerMessageTypeDiscriminantReaction,
                        ClientMessageEdit(_) => Edit,
                    },
                    req.Uuid,
                )],
            );

            //We should send the incoming message to all of the clients, we are already storing the messages in self.messages
            Ok(())
        } else {
            send_message_to_client(&mut *client_buffer, "Invalid Password!".into()).await?;

            Err(Error::msg("Invalid password entered by client!"))
        }
    }

    /// all the functions the server can do
    async fn NormalMessage(&self, req: &ClientMessage) {
        let mut messages = self.messages.lock().await;
        messages.push(ServerOutput::convert_type_to_servermsg(
            req.clone(),
            //Im not sure why I did that, Tf is this?
            -1,
            Normal,
            req.Uuid.clone(),
        ));
    }
    async fn sync_message(&self, req: &ClientMessage) -> anyhow::Result<String> {
        let all_messages = &mut self.messages.lock().await.clone();

        let all_messages_len = all_messages.len();

        //Dont ask me why I did it this way
        let selected_messages_part = if let ClientSyncMessage(inner) = &req.MessageType {
            //if its Some(_) then modify the list, the whole updated list will get sent back to the client regardless
            if let Some(last_seen_message_index) = inner.last_seen_message_index {
                match &mut self.clients_last_seen_index.try_lock() {
                    Ok(client_vec) => {
                        //Iter over the whole list so we can update the user's index if there is one
                        if let Some(client_index_pos) =
                            client_vec.iter().position(|client| client.uuid == req.Uuid)
                        {
                            //Update index
                            client_vec[client_index_pos].index = last_seen_message_index;
                        } else {
                            client_vec.push(ClientLastSeenMessage::new(
                                last_seen_message_index,
                                req.Uuid.clone(),
                                req.Author.clone(),
                            ));
                        }
                    }
                    Err(err) => {
                        dbg!(err);
                    }
                }
            }

            //client_message_counter is how many messages does the client have
            if let Some(counter) = inner.client_message_counter {
                //Check if user already has all the messages
                if !counter >= all_messages_len {
                    &all_messages[counter..all_messages_len]
                } else {
                    //Return empty vector
                    &[]
                }
            } else {
                all_messages
            }
        } else {
            all_messages
        };

        //Construct reply
        let server_master = ServerMaster {
            struct_list: selected_messages_part.to_vec(),
            user_seen_list: self.clients_last_seen_index.try_lock().unwrap().clone(),
            reaction_list: (*self.reactions.try_lock().unwrap().clone()).to_vec(),
            auto_sync_attributes: None
        };

        //convert reply into string
        let final_msg: String = server_master.struct_into_string();

        //Encrypt string
        let encrypted_msg = encrypt_aes256(final_msg, &self.decryption_key).unwrap();

        //Reply with encrypted string
        Ok(encrypted_msg)
    }
    async fn recive_file(&self, request: ClientMessage, req: &ClientFileUploadStruct) {
        //500mb limit
        if !req.bytes.len() > 500000000 {
            match env::var("APPDATA") {
                Ok(app_data) => {
                    //generate a random number to avoid file overwrites, cuz of same name files
                    let random_generated_number = rand::thread_rng().gen_range(-i64::MAX..i64::MAX);

                    //create file, add file to its named so it can never be mixed with images
                    match fs::File::create(format!(
                        "{app_data}\\Matthias\\Server\\{}file.{}",
                        random_generated_number,
                        req.extension.clone().unwrap_or_default()
                    )) {
                        Ok(mut created_file) => {
                            if let Err(err) = created_file.write_all(&req.bytes) {
                                println!("[{err}\n{}]", err.kind());
                            };

                            created_file.flush().unwrap();
                            //success

                            match self.generated_file_paths.lock() {
                                Ok(mut ok) => {
                                    ok.push(PathBuf::from(format!(
                                        "{app_data}\\Matthias\\Server\\{}file.{}",
                                        random_generated_number,
                                        req.extension.clone().unwrap_or_default()
                                    )));
                                }
                                Err(err) => {
                                    println!("{err}")
                                }
                            };

                            let mut messages = self.messages.lock().await;
                            messages.push(ServerOutput::convert_type_to_servermsg(
                                request.clone(),
                                self.generated_file_paths.lock().unwrap().len() as i32,
                                Upload,
                                request.Uuid.clone(),
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
    async fn serve_file(&self, index: i32) -> (Vec<u8>, PathBuf) {
        let path = &self.generated_file_paths.lock().unwrap()[index as usize];
        (fs::read(path).unwrap_or_default(), path.clone())
    }
    async fn serve_image(&self, index: i32) -> Vec<u8> {
        fs::read(&self.image_list.lock().unwrap()[index as usize]).unwrap_or_default()
    }
    async fn recive_image(&self, req: ClientMessage, img: &ClientFileUploadStruct) {
        match env::var("APPDATA") {
            Ok(app_data) => {
                let mut image_path = self.image_list.lock().unwrap();

                let image_path_lenght = image_path.len();

                match fs::File::create(format!(
                    "{app_data}\\Matthias\\Server\\{}",
                    image_path_lenght
                )) {
                    Ok(mut created_file) => {
                        if let Err(err) = created_file.write_all(&img.bytes) {
                            println!("[{err}\n{}]", err.kind());
                        };

                        created_file.flush().unwrap();
                        //success

                        match self.messages.try_lock() {
                            Ok(mut ok) => {
                                ok.push(ServerOutput::convert_type_to_servermsg(
                                    req.clone(),
                                    image_path_lenght as i32,
                                    Image,
                                    req.Uuid.clone(),
                                ));
                            }
                            Err(err) => println!("{err}"),
                        }

                        //Only save as last step to avoid a mismatch + correct indexing :)
                        image_path.push(PathBuf::from(format!(
                            "{app_data}\\Matthias\\Server\\{}",
                            image_path_lenght
                        )));
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
        let mut audio_paths = self.audio_list.lock().unwrap();

        let audio_paths_lenght = audio_paths.len();

        match fs::File::create(format!(
            "{}\\Matthias\\Server\\{}",
            env!("APPDATA"),
            audio_paths_lenght
        )) {
            Ok(mut created_file) => {
                if let Err(err) = created_file.write_all(&audio.bytes) {
                    println!("[{err}\n{}]", err.kind());
                };

                created_file.flush().unwrap();
                //success

                match self.messages.try_lock() {
                    Ok(mut ok) => {
                        ok.push(ServerOutput::convert_type_to_servermsg(
                            req.clone(),
                            audio_paths_lenght as i32,
                            Audio,
                            req.Uuid.clone(),
                        ));
                    }
                    Err(err) => println!("{err}"),
                }

                //Only save as last step to avoid a mismatch + correct indexing :)
                audio_paths.push(PathBuf::from(format!(
                    "{}\\Matthias\\Server\\{}",
                    env!("APPDATA"),
                    audio_paths_lenght
                )));

                //consequently save the audio_recording's name
                match self.audio_names.try_lock() {
                    Ok(mut vec) => vec.push(audio.name.clone()),
                    Err(err) => println!("{err}"),
                }
            }
            Err(err) => {
                println!(" [{err} {}]", err.kind());
            }
        }
    }
    async fn serve_audio(&self, index: i32) -> (Vec<u8>, Option<String>) {
        (
            fs::read(&self.audio_list.lock().unwrap()[index as usize]).unwrap_or_default(),
            self.audio_names.lock().unwrap()[index as usize].clone(),
        )
    }

    /// used to handle all the requests, route the user's request
    #[inline]
    pub async fn handle_request(
        &self,
        request_type: &ClientRequestTypeStruct,
    ) -> anyhow::Result<String> {
        match request_type {
            ClientRequestTypeStruct::ClientImageRequest(img_request) => {
                let read_file = self.serve_image(img_request.index).await;

                let output = serde_json::to_string(&ServerImageReply {
                    bytes: read_file,
                    index: img_request.index,
                })
                .unwrap_or_default();

                Ok(output)
            }
            ClientRequestTypeStruct::ClientFileRequest(file_request) => {
                let (file_bytes, file_name) = &self.serve_file(file_request.index).await;

                let output = serde_json::to_string(&ServerFileReply {
                    file_name: file_name.clone(),
                    bytes: file_bytes.clone(),
                })
                .unwrap_or_default();

                Ok(output)
            }
            ClientRequestTypeStruct::ClientAudioRequest(audio_request) => {
                let (file_bytes, file_name) = self.serve_audio(audio_request.index).await;

                let output = serde_json::to_string(&ServerAudioReply {
                    bytes: file_bytes,
                    index: audio_request.index,
                    file_name: file_name.unwrap_or_default(),
                })
                .unwrap_or_default();

                Ok(output)
            }
        }
    }

    /// handle all the file uploads
    pub async fn handle_upload(&self, req: ClientMessage, upload_type: &ClientFileUploadStruct) {
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
                    if item.char == reaction.char {
                        item.times += 1;

                        //Quit the function immediately, so we can add the new reaction
                        return;
                    }
                }

                //After we have checked all the reactions if there is already one, we can add out *new* one
                reaction_vec[reaction.message_index]
                    .message_reactions
                    .push(Reaction {
                        char: reaction.char,
                        //Set default amount, start from 1
                        times: 1,
                    });
            }
            Err(err) => println!("{err}"),
        }
    }

    async fn handle_message_edit(&self, edit: &ClientMessageEditStruct, req: &ClientMessage) {
        match &mut self.messages.try_lock() {
            Ok(messages_vec) => {
                //Server-side uuid check
                if messages_vec[edit.index].uuid != req.Uuid {
                    return;
                }

                //If its () then we can check for the index, because you can delete all messages, rest is ignored
                if edit.new_message.is_none() {
                    //Set as `Deleted`
                    messages_vec[edit.index].MessageType = ServerMessageType::Deleted;
                }

                if let ServerMessageType::Normal(inner_msg) =
                    &mut messages_vec[edit.index].MessageType
                {
                    if let Some(new_msg) = edit.new_message.clone() {
                        inner_msg.message = new_msg;

                        inner_msg.has_been_edited = true;
                    }
                }
            }
            Err(err) => println!("{err}"),
        }
    }
}
