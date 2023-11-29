use chrono::{format::StrftimeItems, Local};

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
    message_server::{Message, MessageServer},
    FileRequest, FileResponse, FileSend, FileStatus, MessageRequest, MessageResponse, MessageSync,
};

pub mod messages {
    tonic::include_proto!("messages");
}

#[derive(Debug, Default)]
pub struct MessageService {
    pub messages: Mutex<Vec<String>>,
    pub passw: String,

    //files
    pub file_paths: Mutex<Vec<PathBuf>>,
}
#[tonic::async_trait]
impl Message for MessageService {
    async fn send_message(
        &self,
        request: Request<MessageRequest>,
    ) -> Result<Response<MessageResponse>, Status> {
        println!("Got a request: {:?}", request);

        let req = request.into_inner();

        let current_datetime = Local::now();
        let format = StrftimeItems::new("%Y.%m.%d. %H:%M");
        let formatted_datetime = current_datetime.format_with_items(format);

        if req.password.trim() == self.passw.trim() {
            match self.messages.lock() {
                Ok(mut ok) => {
                    ok.push(
                        format!("{formatted_datetime} $ {} | {} ", req.author, req.message) + "\n",
                    );
                }
                Err(err) => {
                    println!("{err}")
                }
            };
        }
        let shared_messages = self.messages.lock().unwrap().clone();

        let handle = std::thread::spawn(move || {
            let final_msg: String = shared_messages
                .iter()
                .map(|s| s.to_string())
                .collect::<String>();

            final_msg
        });

        // Wait for the spawned thread to finish
        let final_msg = handle.join().unwrap();
        if req.password.trim() == self.passw.trim() {
            let reply = MessageResponse {
                message: final_msg.to_string(),
            };

            Ok(Response::new(reply))
        }
        //invalid passw
        else {
            let reply = MessageResponse {
                message: "Invalid Password!".to_string(),
            };
            Ok(Response::new(reply))
        }
    }
    async fn sync_message(
        &self,
        request: Request<MessageSync>,
    ) -> Result<Response<MessageResponse>, Status> {
        //pass matching p100 technology
        if request.into_inner().password == self.passw {
            let shared_messages = self.messages.lock().unwrap().clone();

            let handle = std::thread::spawn(move || {
                let final_msg: String = shared_messages
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<String>();

                final_msg
            });

            // Wait for the spawned thread to finish
            let final_msg = handle.join().unwrap();

            let reply = MessageResponse { message: final_msg };
            Ok(Response::new(reply))
        } else {
            let reply = MessageResponse {
                message: "Invalid password!".into(),
            };
            Ok(Response::new(reply))
        }
    }
    async fn recive_file(
        &self,
        request: Request<FileSend>,
    ) -> Result<Response<FileStatus>, Status> {
        /*

        error -> 0 success
        error -> 1 Server : failed to get APPDATA arg
        error -> 2 Server : failed to create file

        */
        let req = request.into_inner().clone();

        if req.passw == self.passw {
            let mut error_code: i32 = 0;

            //500mb limit
            if req.file.len() > 500000000 {
                error_code = -1;
            } else {
                match env::var("APPDATA") {
                    Ok(app_data) => {
                        let _create_dir = fs::create_dir(format!("{}\\szeChat\\Server", app_data));

                        match fs::File::create(format!("{app_data}\\szeChat\\Server\\{}", req.name))
                        {
                            Ok(mut created_file) => {
                                if let Err(err) = created_file.write_all(&req.file) {
                                    println!("[{err}\n{}]", err.kind());
                                };

                                created_file.flush().unwrap();
                                //success

                                match self.file_paths.lock() {
                                    Ok(mut ok) => {
                                        ok.push(PathBuf::from(format!(
                                            "{app_data}\\szeChat\\Server\\{}",
                                            req.name
                                        )));
                                    }
                                    Err(err) => {
                                        println!("{err}")
                                    }
                                };
                                let current_datetime = Local::now();
                                let format = StrftimeItems::new("%Y.%m.%d. %H:%M");
                                let formatted_datetime = current_datetime.format_with_items(format);
                                match self.messages.lock() {
                                    Ok(mut ok) => {
                                        ok.push(format!(
                                            //use a character before file upload which cannot be set as a file name
                                            "{formatted_datetime} $ {} | >file_upload '{}' '{}'\n",
                                            req.author,
                                            req.name,
                                            self.file_paths.lock().unwrap().len() - 1
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
                        error_code = 1;
                        println!("{err}")
                    }
                }
            }

            dbg!(&self.file_paths);

            let reply = FileStatus { error: error_code };

            Ok(Response::new(reply))
        } else {
            let reply = FileStatus { error: -2 };

            Ok(Response::new(reply))
        }
    }

    async fn serve_file(
        &self,
        request: Request<FileRequest>,
    ) -> Result<Response<FileResponse>, Status> {
        let req = request.into_inner().clone();

        let file_path_vec = self.file_paths.lock().unwrap();

        let apad = &file_path_vec[req.index as usize];

        let file = fs::read(apad).unwrap();

        let file_name = apad.file_name().unwrap().to_string_lossy().to_string();

        let reply = FileResponse {
            file,
            name: file_name,
        };

        Ok(Response::new(reply))
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
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
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

    Ok(messages.to_vec())
}
