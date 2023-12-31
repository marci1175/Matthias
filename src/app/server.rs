use std::{env, fs, io::Write, path::PathBuf};

use rand::Rng;
use std::sync::Mutex;
use tonic::{transport::Server, Request, Response, Status};
use super::backend::ServerMessageTypeDiscriminants::{Normal, Image, Audio, Upload};
/*
use std::{io, time::Duration};
use clap::Parser;
use rcgen::{Certificate, CertificateParams, DistinguishedName};
use tokio::time::sleep;
use log::{info, error};
use clap_derive::Parser;
use instant_acme::{
    Account, AuthorizationStatus, ChallengeType, Identifier, LetsEncrypt, NewAccount, NewOrder,
    OrderStatus,
};

*/

use messages::{
    message_server::{Message as ServerMessage, MessageServer},
    MessageRequest, MessageResponse,
};

use crate::app::backend::ServerMaster;
use crate::app::backend::{
    ClientFileRequestType as ClientRequestTypeStruct, ClientFileUpload as ClientFileUploadStruct,
    ClientMessage,
    ClientMessageType::{
        ClientFileRequestType, ClientFileUpload, ClientNormalMessage, ClientSyncMessage,
    },
    ServerFileReply, ServerImageReply,
};

use super::backend::{ServerAudioReply, ServerOutput};

pub mod messages {
    tonic::include_proto!("messages");
}

#[derive(Debug, Default)]
pub struct MessageService {
    pub messages: Mutex<Vec<ServerOutput>>,
    pub passw: String,

    //files
    pub generated_file_paths: Mutex<Vec<PathBuf>>,

    //file_names
    pub original_file_paths: Mutex<Vec<PathBuf>>,

    //images
    pub image_paths: Mutex<Vec<PathBuf>>,

    //audio list
    pub audio_list: Mutex<Vec<PathBuf>>,

    //audio name list
    pub audio_names: Mutex<Vec<Option<String>>>,
}
#[tonic::async_trait]
impl ServerMessage for MessageService {
    #[inline]
    async fn message_main(
        &self,
        request: Request<MessageRequest>,
    ) -> Result<Response<MessageResponse>, Status> {
        let req_result: Result<ClientMessage, serde_json::Error> =
            serde_json::from_str(&request.into_inner().message);
        let req: ClientMessage = req_result.unwrap();

        if &req.Password == self.passw.trim() {
            match &req.MessageType {
                ClientNormalMessage(_msg) => self.NormalMessage(req).await,

                ClientSyncMessage(_msg) => { /*Dont do anything we will always reply with the list of msgs*/
                }

                ClientFileRequestType(request_type) => {
                    return self.handle_request(request_type).await;
                }

                ClientFileUpload(upload_type) => {
                    self.handle_upload(req.clone(), upload_type).await;
                }
            };

            return self.sync_message().await;
        } else {
            return Ok(Response::new(MessageResponse {
                message: "Invalid Password!".into(),
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

pub async fn server_main(
    port: String,
    password: String,
    ip_v4: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    //apad().await;
    let mut addr: std::net::SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    if !ip_v4 {
        addr = format!("[::]:{}", port).parse()?;
    }

    let msg_service = MessageService {
        passw: password,
        ..Default::default()
    };

    let messages = &msg_service.messages.lock().unwrap().to_vec();
    
    Server::builder()
        .add_service(MessageServer::new(msg_service))
        .serve(addr)
        .await?;

    let reply: String = messages.iter().map(|f| f.struct_into_string()).collect();
    Ok(reply)
}

impl MessageService {
    pub async fn NormalMessage(&self, req: ClientMessage) {
        match self.messages.lock() {
            Ok(mut ok) => {
                ok.push(ServerOutput::convert_type_to_servermsg(req, -1, Normal));
            }
            Err(err) => {
                println!("{err}")
            }
        };
    }
    pub async fn sync_message(&self) -> Result<Response<MessageResponse>, Status> {
        //pass matching p100 technology
        let shared_messages = self.messages.lock().unwrap().clone();

        let server_master = ServerMaster::convert_vec_serverout_into_server_master(shared_messages);

        let final_msg: String = server_master.struct_into_string();

        // Wait for the spawned thread to finish

        let reply = MessageResponse { message: final_msg };
        Ok(Response::new(reply))
    }
    pub async fn recive_file(&self, request: ClientMessage, req: &ClientFileUploadStruct) {
        /*

            DEPRICATED

        error -> 0 success
        error -> 1 Server : failed to get APPDATA arg
        error -> 2 Server : failed to create file

        */

        //500mb limit
        if req.bytes.len() > 500000000 {
            //Dont allow the upload
        } else {
            match env::var("APPDATA") {
                Ok(app_data) => {
                    //generat a random number to avoid file overwrites, cuz of same name files
                    let random_generated_number = rand::thread_rng().gen_range(-i64::MAX..i64::MAX);

                    //create file, add file to its named so it can never be mixed with images
                    match fs::File::create(format!(
                        "{app_data}\\szeChat\\Server\\{}file.{}",
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
                                        "{app_data}\\szeChat\\Server\\{}file.{}",
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
                                        "{app_data}\\szeChat\\Server\\{}.{}",
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
                                        request,
                                        self.original_file_paths.lock().unwrap().len() as i32 - 1,
                                        Upload
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
    pub async fn serve_file(&self, index: i32) -> (Vec<u8>, PathBuf) {
        let path = &self.generated_file_paths.lock().unwrap()[index as usize];
        (fs::read(path).unwrap_or_default(), path.clone())
    }
    pub async fn serve_image(&self, index: i32) -> Vec<u8> {
        fs::read(&self.image_paths.lock().unwrap()[index as usize]).unwrap_or_default()
    }
    pub async fn recive_image(&self, req: ClientMessage, img: &ClientFileUploadStruct) {
        match env::var("APPDATA") {
            Ok(app_data) => {
                let mut image_path = self.image_paths.lock().unwrap();

                let image_path_lenght = image_path.len();

                match fs::File::create(format!(
                    "{app_data}\\szeChat\\Server\\{}",
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
                                    Image
                                ));
                            }
                            Err(err) => println!("{err}"),
                        }

                        //Only save as last step to avoid a mismatch + correct indexing :)
                        image_path.push(PathBuf::from(format!(
                            "{app_data}\\szeChat\\Server\\{}",
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
    pub async fn recive_audio(&self, req: ClientMessage, audio: &ClientFileUploadStruct) {
        let mut audio_paths = self.audio_list.lock().unwrap();

        let audio_paths_lenght = audio_paths.len();
        
        match fs::File::create(format!(
            "{}\\szeChat\\Server\\{}",
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
                        ));
                    }
                    Err(err) => println!("{err}"),
                }

                //Only save as last step to avoid a mismatch + correct indexing :)
                audio_paths.push(PathBuf::from(format!(
                    "{}\\szeChat\\Server\\{}",
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
    pub async fn serve_audio(&self, index: i32) -> (Vec<u8>, Option<String>) {
        (
            fs::read(&self.audio_list.lock().unwrap()[index as usize]).unwrap_or_default(),
            self.audio_names.lock().unwrap()[index as usize].clone(),
        )
    }

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

    pub async fn handle_upload(&self, req: ClientMessage, upload_type: &ClientFileUploadStruct) {
        //Pattern match on upload tpye so we know how to handle the specific request
        match upload_type.extension.clone().unwrap_or_default().as_str() {
            "png" | "jpeg" | "bmp" | "tiff" | "webp" => self.recive_image(req, upload_type).await,
            "wav" | "mp3" | "m4a" => self.recive_audio(req, upload_type).await,
            //Define file types and how should the server handle them based on extension, NOTICE: ENSURE CLIENT COMPATIBILITY
            _ => self.recive_file(req, upload_type).await,
        }
    }
}
