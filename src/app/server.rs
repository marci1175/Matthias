use std::{env, fs, io::Write, path::PathBuf};

use std::sync::Mutex;
use tonic::{transport::Server, Request, Response, Status};

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
    FileRequest as FileRequestStruct, FileServe, FileUpload as FileUploadStruct, Message,
    MessageType::{FileRequest, FileUpload, Image, NormalMessage, SyncMessage},
};

use super::backend::ServerOutput;

pub mod messages {
    tonic::include_proto!("messages");
}

#[derive(Debug, Default)]
pub struct MessageService {
    pub messages: Mutex<Vec<ServerOutput>>,
    pub passw: String,

    //files
    pub file_paths: Mutex<Vec<PathBuf>>,
}
#[tonic::async_trait]
impl ServerMessage for MessageService {
    async fn message_main(
        &self,
        request: Request<MessageRequest>,
    ) -> Result<Response<MessageResponse>, Status> {
        let req_result: Result<Message, serde_json::Error> =
            serde_json::from_str(&request.into_inner().message);
        let req: Message = req_result.unwrap();

        if &req.Password == self.passw.trim() {
            match &req.MessageType {
                NormalMessage(_msg) => self.NormalMessage(req).await,
                SyncMessage(_msg) => { /*Dont do anything we will always reply with the list of msgs*/}
                Image(_) => {
                    self.ImageMessage(req).await;
                }
                FileRequest(msg) => {
                    let (file_bytes, file_name) = &self.serve_file(msg.index).await;
                    let output = serde_json::to_string(&FileServe {
                        file_name: file_name.clone(),
                        bytes: file_bytes.clone(),
                    })
                    .unwrap_or_default();
                    return Ok(Response::new(MessageResponse { message: output }));
                }
                FileUpload(_) => {
                    self.recive_file(req.clone()).await;
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
    let btc_service = MessageService {
        passw: password,
        ..Default::default()
    };

    let messages = &btc_service.messages.lock().unwrap().to_vec();

    Server::builder()
        //.tls_config(ServerTlsConfig::new().client_ca_root(Certificate::from_pem(pem)))
        .add_service(MessageServer::new(btc_service))
        .serve(addr)
        .await?;

    let reply: String = messages.iter().map(|f| f.struct_into_string()).collect();
    Ok(reply)
}

impl MessageService {
    pub async fn NormalMessage(&self, req: Message) {
        match self.messages.lock() {
            Ok(mut ok) => {
                ok.push(ServerOutput::convert_msg_to_servermsg(req));
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
    pub async fn ImageMessage(&self, req: Message) {
        match self.messages.lock() {
            Ok(mut ok) => {
                ok.push(ServerOutput::convert_picture_to_servermsg(req));
            }
            Err(err) => {
                println!("{err}")
            }
        };
    }
    pub async fn recive_file(&self, request: Message) {
        /*

        error -> 0 success
        error -> 1 Server : failed to get APPDATA arg
        error -> 2 Server : failed to create file

        */
        let mut error_code: i32 = 0;

        if let FileUpload(req) = request.clone().MessageType {
            //500mb limit
            if req.bytes.len() > 500000000 {
                error_code = -1;
            } else {
                match env::var("APPDATA") {
                    Ok(app_data) => {
                        let _create_dir = fs::create_dir(format!("{}\\szeChat\\Server", app_data));

                        match fs::File::create(format!(
                            "{app_data}\\szeChat\\Server\\{}.{}",
                            req.name, req.extension
                        )) {
                            Ok(mut created_file) => {
                                if let Err(err) = created_file.write_all(&req.bytes) {
                                    println!("[{err}\n{}]", err.kind());
                                };

                                created_file.flush().unwrap();
                                //success

                                match self.file_paths.lock() {
                                    Ok(mut ok) => {
                                        ok.push(PathBuf::from(format!(
                                            "{app_data}\\szeChat\\Server\\{}.{}",
                                            req.name, req.extension
                                        )));
                                    }
                                    Err(err) => {
                                        println!("{err}")
                                    }
                                };
                                match self.messages.lock() {
                                    Ok(mut ok) => {
                                        ok.push(ServerOutput::convert_upload_to_servermsg(
                                            request,
                                            self.file_paths.lock().unwrap().len() as i32 - 1,
                                        ));
                                    }
                                    Err(err) => println!("{err}"),
                                }
                            }
                            Err(err) => {
                                println!(" [{err}\n{}]", err.kind());
                                error_code = 2;
                            }
                        }
                    }
                    Err(err) => {
                        println!("{err}")
                    }
                }
            }
        }

        dbg!(error_code);
    }
    pub async fn serve_file(&self, index: i32) -> (Vec<u8>, PathBuf) {
        let path = &self.file_paths.lock().unwrap()[index as usize];
        (fs::read(path).unwrap_or_default(), path.clone())
    }
}
