use std::{env, fs, io::Write, net::SocketAddr, path::PathBuf, sync::Arc};

use super::backend::{
    encrypt_aes256, ClientLastSeenMessage, ClientMessageType, ConnectedClient, MessageReaction,
    Reaction, ServerMessageType,
    ServerMessageTypeDiscriminants::{Audio, Image, Normal, Upload},
};

use messages::{
    message_server::{Message as ServerMessage, MessageServer},
    MessageRequest, MessageResponse,
};
use rand::Rng;
use std::sync::Mutex;
use tokio::sync::mpsc::Receiver;
use tonic::{transport::Server, Request, Response, Status};

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

use super::backend::{ServerAudioReply, ServerOutput};

pub mod messages {
    tonic::include_proto!("messages");
}

#[derive(Debug, Default)]
pub struct MessageService {
    ///Contains all the messages
    pub messages: Mutex<Vec<ServerOutput>>,

    ///Contains all of the reactions added to the messages
    pub reactions: Mutex<Vec<MessageReaction>>,

    ///This is the required password by the server
    pub passw: String,

    ///files
    pub generated_file_paths: Mutex<Vec<PathBuf>>,

    ///file_names
    pub original_file_paths: Mutex<Vec<PathBuf>>,

    ///images
    pub image_paths: Mutex<Vec<PathBuf>>,

    ///audio list
    pub audio_list: Mutex<Vec<PathBuf>>,

    ///audio name list
    pub audio_names: Mutex<Vec<Option<String>>>,

    ///connected clients
    pub connected_clients: Mutex<Vec<ConnectedClient>>,

    ///Client secret
    pub decryption_key: [u8; 32],

    ///Client last seen message
    pub clients_last_seen_index: Mutex<Vec<ClientLastSeenMessage>>,
}

#[tonic::async_trait]
impl ServerMessage for MessageService {
    ///MessageResponse contains the serialized info (String)
    #[inline]
    async fn message_main(
        &self,
        request: Request<MessageRequest>,
    ) -> Result<Response<MessageResponse>, Status> {
        let inbound_connection_address = request.remote_addr();

        let req_result: Result<ClientMessage, serde_json::Error> =
            serde_json::from_str(&request.into_inner().message);

        let req: ClientMessage = req_result.unwrap();

        if let Some(remote_address) = inbound_connection_address {
            if let ClientMessageType::ClientSyncMessage(sync_msg) = &req.MessageType {
                if sync_msg.password == self.passw.trim() {
                    //Handle incoming connections and disconnections, if inbound_connection_address is some, else we assume its for syncing *ONLY*
                    if let Some(sync_attr) = sync_msg.sync_attribute {
                        //Incoming connection, we should be returning temp uuid
                        if sync_attr {
                            match self.connected_clients.lock() {
                                Ok(mut clients) => {
                                    //Search for connected ip in all connected ips
                                    for client in clients.iter() {
                                        //If found, then the client is already connected
                                        if client.address == remote_address {
                                            //This can only happen if the connection closed unexpectedly (If the client was stopped unexpectedly)
                                            return Ok(Response::new(MessageResponse {
                                                message: hex::encode(self.decryption_key),
                                            }));
                                        }
                                    }

                                    //If the ip is not found then add it to connected clients
                                    clients.push(ConnectedClient::new(
                                        remote_address,
                                        req.Uuid,
                                        req.Author,
                                    ));

                                    //Return custom which will the server's text will be encrypted with
                                    return Ok(Response::new(MessageResponse {
                                        message: hex::encode(self.decryption_key),
                                    }));
                                }
                                Err(err) => {
                                    dbg!(err);
                                }
                            }
                        }
                        //Handle disconnections
                        else {
                            match self.connected_clients.lock() {
                                Ok(mut clients) => {
                                    //Search for connected ip in all connected ips
                                    for (index, client) in clients.clone().iter().enumerate() {
                                        //If found, then disconnect the client
                                        if client.address == remote_address {
                                            clients.remove(index);

                                            //Stop the for loop, for safety
                                            break;
                                        }
                                    }
                                }
                                Err(err) => {
                                    dbg!(err);
                                }
                            }
                        }
                    }

                    //else: we dont do anything because we return the updated message list in the end
                }

                //Sync all messages
                return self.sync_message(&req).await;
            }

            if self //Check if we have already established a connection with the client, if yes then it doesnt matter what password the user has entered
                .connected_clients
                .lock()
                .unwrap()
                .iter()
                .any(|client| client.address == remote_address)
            //Search through the list
            {
                match &req.MessageType {
                    ClientNormalMessage(_msg) => self.NormalMessage(&req).await,

                    ClientSyncMessage(_msg) => {
                        unimplemented!("How the fuck did you get here?");
                    }

                    ClientFileRequestType(request_type) => {
                        return self.handle_request(request_type).await;
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
                    match self.reactions.lock() {
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
                return self.sync_message(&req).await;
            } else {
                return Ok(Response::new(MessageResponse {
                    message: "Invalid Password!".into(),
                }));
            }
        } else {
            return Ok(Response::new(MessageResponse {
                message: "Invalid Client!".into(),
            }));
        }
    }
}

/*
async fn apad() -> anyhow::Result<()> {
    println!("asd");
    let opts = "kurvaanyad.hu";
    println!("asd");
    // Create a new account. This will generate a fresh ECDSA key for you.
    // Alternatively, restore an account from serialized credentials by
    // using `Account::from_credentials()`.

    let (account, credentials) = Account::create(
        &NewAccount {
            contact: &[],
            terms_of_service_agreed: true,
            only_return_existing: false,
        },
        LetsEncrypt::Staging.url(),
        None,
    )
    .await?;
    info!(
        "account credentials:\n\n{}",
        serde_json::to_string_pretty(&credentials).unwrap()
    );
    println!("asd");
    // Create the ACME order based on the given domain names.
    // Note that this only needs an `&Account`, so the library will let you
    // process multiple orders in parallel for a single account.

    let identifier = Identifier::Dns(opts.to_string());
    let mut order = account
        .new_order(&NewOrder {
            identifiers: &[identifier],
        })
        .await
        .unwrap();

    let state = order.state();
    info!("order state: {:#?}", state);
    assert!(matches!(state.status, OrderStatus::Pending));

    // Pick the desired challenge type and prepare the response.

    let authorizations = order.authorizations().await.unwrap();
    let mut challenges = Vec::with_capacity(authorizations.len());
    for authz in &authorizations {
        match authz.status {
            AuthorizationStatus::Pending => {}
            AuthorizationStatus::Valid => continue,
            _ => todo!(),
        }

        // We'll use the DNS challenges for this example, but you could
        // pick something else to use here.

        let challenge = authz
            .challenges
            .iter()
            .find(|c| c.r#type == ChallengeType::Dns01)
            .ok_or_else(|| anyhow::anyhow!("no dns01 challenge found"))?;

        let Identifier::Dns(identifier) = &authz.identifier;

        println!("Please set the following DNS record then press the Return key:");
        println!(
            "_acme-challenge.{} IN TXT {}",
            identifier,
            order.key_authorization(challenge).dns_value()
        );
        io::stdin().read_line(&mut String::new()).unwrap();

        challenges.push((identifier, &challenge.url));
    }

    // Let the server know we're ready to accept the challenges.

    for (_, url) in &challenges {
        order.set_challenge_ready(url).await.unwrap();
    }

    // Exponentially back off until the order becomes ready or invalid.

    let mut tries = 1u8;
    let mut delay = Duration::from_millis(250);
    loop {
        sleep(delay).await;
        let state = order.refresh().await.unwrap();
        if let OrderStatus::Ready | OrderStatus::Invalid = state.status {
            info!("order state: {:#?}", state);
            break;
        }

        delay *= 2;
        tries += 1;
        match tries < 5 {
            true => info!("order is not ready, waiting {delay:?}, {:?} {}", state, tries),
            false => {
                error!("order is not ready: {state:#?}, {}", tries);
                return Err(anyhow::anyhow!("order is not ready"));
            }
        }
    }

    let state = order.state();
    if state.status != OrderStatus::Ready {
        return Err(anyhow::anyhow!(
            "unexpected order status: {:?}",
            state.status
        ));
    }

    let mut names = Vec::with_capacity(challenges.len());
    for (identifier, _) in challenges {
        names.push(identifier.to_owned());
    }

    // If the order is ready, we can provision the certificate.
    // Use the rcgen library to create a Certificate Signing Request.

    let mut params = CertificateParams::new(names.clone());
    params.distinguished_name = DistinguishedName::new();
    let cert = Certificate::from_params(params).unwrap();
    let csr = cert.serialize_request_der()?;

    // Finalize the order and print certificate chain, private key and account credentials.

    order.finalize(&csr).await.unwrap();
    let cert_chain_pem = loop {
        match order.certificate().await.unwrap() {
            Some(cert_chain_pem) => break cert_chain_pem,
            None => sleep(Duration::from_secs(1)).await,
        }
    };

    info!("certficate chain:\n\n{}", cert_chain_pem);
    info!("private key:\n\n{}", cert.serialize_private_key_pem());
    Ok(())
}

#[derive(Parser)]
struct Options {
    #[clap(long)]
    name: String,
}*/

async fn shutdown_signal(mut signal: Receiver<()>) {
    signal.recv().await;
}

fn interceptor_fn(request: Request<()>) -> Result<Request<()>, Status> {
    Ok(request)
}

pub async fn server_main(
    port: String,
    password: String,
    signal: Receiver<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("[::]:{}", port).parse()?;

    let msg_service = MessageService {
        passw: password,
        decryption_key: rand::random::<[u8; 32]>(),
        ..Default::default()
    };

    Server::builder()
        .add_service(MessageServer::with_interceptor(msg_service, interceptor_fn))
        .serve_with_shutdown(addr, shutdown_signal(signal))
        .await?;

    //Shutdown gracefully
    Ok(())
}

impl MessageService {
    /// all the functions the server can do
    async fn NormalMessage(&self, req: &ClientMessage) {
        match self.messages.lock() {
            Ok(mut ok) => {
                ok.push(ServerOutput::convert_type_to_servermsg(
                    req.clone(),
                    //Im not sure why I did that, Tf is this?
                    -1,
                    Normal,
                    MessageReaction::default(),
                    req.Uuid.clone(),
                ));
            }
            Err(err) => {
                println!("{err}")
            }
        };
    }
    async fn sync_message(&self, req: &ClientMessage) -> Result<Response<MessageResponse>, Status> {
        let all_messages = &mut self.messages.lock().unwrap().clone();

        let all_messages_len = all_messages.len();

        //Dont ask me why I did it this way
        let selected_messages_part = if let ClientSyncMessage(inner) = &req.MessageType {
            //if its Some(_) then modify the list, the whole updated list will get sent back to the client regardless
            if let Some(last_seen_message_index) = inner.last_seen_message_index {
                match &mut self.clients_last_seen_index.lock() {
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
                &all_messages
            }
        } else {
            &all_messages
        };

        //Construct reply
        let server_master = ServerMaster::convert_vec_serverout_into_server_master(
            selected_messages_part.to_vec(),
            (*self.reactions.lock().unwrap().clone()).to_vec(),
            self.clients_last_seen_index.lock().unwrap().clone(),
        );

        //convert reply into string
        let final_msg: String = server_master.struct_into_string();

        //Encrypt string
        let encrypted_msg = encrypt_aes256(final_msg, &self.decryption_key).unwrap();

        //Wrap final reply
        let reply = MessageResponse {
            message: encrypted_msg,
        };

        //Reply with encrypted string
        Ok(Response::new(reply))
    }
    async fn recive_file(&self, request: ClientMessage, req: &ClientFileUploadStruct) {
        //500mb limit
        if !req.bytes.len() > 500000000 {
            match env::var("APPDATA") {
                Ok(app_data) => {
                    //generat a random number to avoid file overwrites, cuz of same name files
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

                            match self.original_file_paths.lock() {
                                Ok(mut ok) => {
                                    ok.push(PathBuf::from(format!(
                                        "{app_data}\\Matthias\\Server\\{}.{}",
                                        req.name.clone().unwrap_or_default(),
                                        req.extension.clone().unwrap_or_default()
                                    )));
                                }
                                Err(err) => {
                                    println!("{err}")
                                }
                            };

                            match self.messages.lock() {
                                Ok(mut ok) => {
                                    ok.push(ServerOutput::convert_type_to_servermsg(
                                        request.clone(),
                                        self.original_file_paths.lock().unwrap().len() as i32 - 1,
                                        Upload,
                                        MessageReaction::default(),
                                        request.Uuid.clone(),
                                    ));
                                }
                                Err(err) => println!("{err}"),
                            }
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
        fs::read(&self.image_paths.lock().unwrap()[index as usize]).unwrap_or_default()
    }
    async fn recive_image(&self, req: ClientMessage, img: &ClientFileUploadStruct) {
        match env::var("APPDATA") {
            Ok(app_data) => {
                let mut image_path = self.image_paths.lock().unwrap();

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
                                    MessageReaction::default(),
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
                            MessageReaction::default(),
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
    ) -> Result<Response<MessageResponse>, Status> {
        match request_type {
            ClientRequestTypeStruct::ClientImageRequest(img_request) => {
                let read_file = self.serve_image(img_request.index).await;

                let output = serde_json::to_string(&ServerImageReply {
                    bytes: read_file,
                    index: img_request.index,
                })
                .unwrap_or_default();

                Ok(Response::new(MessageResponse { message: output }))
            }
            ClientRequestTypeStruct::ClientFileRequest(file_request) => {
                let (file_bytes, file_name) = &self.serve_file(file_request.index).await;

                let output = serde_json::to_string(&ServerFileReply {
                    file_name: file_name.clone(),
                    bytes: file_bytes.clone(),
                })
                .unwrap_or_default();

                Ok(Response::new(MessageResponse { message: output }))
            }
            ClientRequestTypeStruct::ClientAudioRequest(audio_request) => {
                let (file_bytes, file_name) = self.serve_audio(audio_request.index).await;

                let output = serde_json::to_string(&ServerAudioReply {
                    bytes: file_bytes,
                    index: audio_request.index,
                    file_name: file_name.unwrap_or_default(),
                })
                .unwrap_or_default();

                Ok(Response::new(MessageResponse { message: output }))
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

                //If its none then we can check for the index, because you can delete all messages, rest is ignored
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
