use chrono::Utc;
use rand::rngs::ThreadRng;

use std::collections::BTreeMap;
use std::fs::{self, File};

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc};

use crate::app::input::Input;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    //fontbook
    pub filter: String,
    pub named_chars: BTreeMap<egui::FontFamily, BTreeMap<char, String>>,

    //login page
    pub login_username: String,
    #[serde(skip)]
    pub login_password: String,
    //server main
    pub ipv4_mode: bool,
    #[serde(skip)]
    pub server_has_started: bool,
    #[serde(skip)]
    pub public_ip: String,
    //server settings
    pub server_req_password: bool,

    pub server_password: String,

    pub open_on_port: String,

    //thread communication for server
    #[serde(skip)]
    pub srx: mpsc::Receiver<String>,
    #[serde(skip)]
    pub stx: mpsc::Sender<String>,

    //child windows
    #[serde(skip)]
    pub settings_window: bool,

    //thread communication for file requesting
    #[serde(skip)]
    pub frx: mpsc::Receiver<String>,
    #[serde(skip)]
    pub ftx: mpsc::Sender<String>,

    //thread communication for image requesting
    #[serde(skip)]
    pub irx: mpsc::Receiver<String>,
    #[serde(skip)]
    pub itx: mpsc::Sender<String>,

    //main
    #[serde(skip)]
    pub emoji_mode: bool,
    #[serde(skip)]
    pub keymap: Input,
    #[serde(skip)]
    pub bookmark_mode: bool,
    #[serde(skip)]
    pub client_mode: bool,
    #[serde(skip)]
    pub server_mode: bool,
    #[serde(skip)]
    pub mode_selector: bool,
    #[serde(skip)]
    pub opened_account_path: PathBuf,
    #[serde(skip)]
    pub opened_account: Option<File>,

    //client main
    #[serde(skip)]
    pub files_to_send: Vec<PathBuf>,
    pub usr_msg_expanded: bool,
    pub send_on_ip: String,
    pub req_passw: bool,
    pub client_password: String,

    //font
    pub font_size: f32,
    pub how_on: f32,
    #[serde(skip)]
    pub drop_file_animation: bool,
    //msg
    #[serde(skip)]
    pub replying_to: Option<usize>,
    #[serde(skip)]
    pub usr_msg: String,
    #[serde(skip)]
    pub incoming_msg_time: Vec<String>,
    #[serde(skip)]
    pub incoming_msg: ServerMaster,
    //emoji fasz
    pub random_emoji: String,
    pub emoji: Vec<String>,
    #[serde(skip)]
    pub rand_eng: ThreadRng,
    pub random_generated: bool,
    //thread communication for client
    #[serde(skip)]
    pub rx: mpsc::Receiver<String>,
    #[serde(skip)]
    pub tx: mpsc::Sender<String>,
    //data sync
    #[serde(skip)]
    pub drx: mpsc::Receiver<String>,
    #[serde(skip)]
    pub dtx: mpsc::Sender<String>,
    #[serde(skip)]
    pub has_init: bool,
    #[serde(skip)]
    pub autosync_sender: Option<mpsc::Receiver<String>>,
    #[serde(skip)]
    pub autosync_should_run: Arc<AtomicBool>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel::<String>();
        let (stx, srx) = mpsc::channel::<String>();
        let (dtx, drx) = mpsc::channel::<String>();
        let (ftx, frx) = mpsc::channel::<String>();
        let (itx, irx) = mpsc::channel::<String>();
        Self {
            //fontbook
            filter: Default::default(),
            named_chars: Default::default(),

            //login page
            login_username: String::new(),
            login_password: String::new(),

            //server_main
            ipv4_mode: true,
            server_has_started: false,
            public_ip: String::new(),

            //server settings
            server_req_password: false,
            server_password: String::default(),
            open_on_port: String::default(),

            //thread communication for server
            srx,
            stx,

            //child windows
            settings_window: false,

            //thread communication for file requesting
            frx,
            ftx,

            //thread communication for image requesting
            irx,
            itx,

            //main
            emoji_mode: false,
            keymap: Input::default(),
            bookmark_mode: false,
            client_mode: false,
            server_mode: false,
            mode_selector: false,
            opened_account: None,
            opened_account_path: PathBuf::default(),

            //client main
            files_to_send: Vec::new(),
            how_on: 0.0,
            drop_file_animation: false,
            usr_msg_expanded: false,
            send_on_ip: String::new(),
            req_passw: false,
            client_password: String::new(),
            //font
            font_size: 20.,
            //emoji button
            emoji: vec![
                "😐", "😍", "😉", "😈", "😇", "😆", "😅", "😄", "😃", "😂", "😁", "😀",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>(),
            random_emoji: "🍑".into(),
            rand_eng: rand::thread_rng(),
            random_generated: false,
            //msg
            usr_msg: String::new(),
            replying_to: None,
            incoming_msg_time: Vec::new(),
            incoming_msg: ServerMaster::default(),
            //thread communication for client
            rx,
            tx,
            //data sync
            drx,
            dtx,
            has_init: false,
            autosync_sender: None,
            autosync_should_run: Arc::new(AtomicBool::new(true)),
        }
    }
}

#[allow(dead_code)]
impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

//When the client is uploading a file, this packet gets sent
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientFileUpload {
    pub extension: String,
    pub name: String,
    pub bytes: Vec<u8>,
}

//Normal message
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientNormalMessage {
    pub message: String,
}

//Empty packet, as described later, only used for syncing
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientSnycMessage {/*Empty packet, only for syncing*/}

//This is used by the client for requesting file
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientFileRequest {
    pub index: i32,
}

//This is used by the client for requesting images
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientImageRequest {
    pub index: i32,
}

//this is used to send out images to the server TODO: IMPROVE IMAGE / FILE SENDING HANDLING BY SERVER __
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientImage {
    pub bytes: Vec<u8>,
}

//Client outgoing message types
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum ClientMessageType {
    //this is when you want to display an image and you have to make a request to the server file
    ClientImageRequest(ClientImageRequest),

    ClientSyncMessage(ClientSnycMessage),
    ClientFileRequest(ClientFileRequest),
    ClientFileUpload(ClientFileUpload),

    //this is when you are sending images to the server TODO: REMOVE THIS WHOLE POOP REF LINE 239
    ClientImage(ClientImage),

    ClientNormalMessage(ClientNormalMessage),
}

//This is what gets to be sent out by the client
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientMessage {
    pub replying_to: Option<usize>,
    pub MessageType: ClientMessageType,
    pub Password: String,
    pub Author: String,
    pub MessageDate: String,
    pub Destination: String,
}

impl ClientMessage {
    //struct into string, it makes sending information easier by putting it all in a string
    pub fn struct_into_string(&self) -> String {
        return serde_json::to_string(self).unwrap_or_default();
    }

    //this is used when sending a normal message
    pub fn construct_normal_msg(
        msg: &str,
        ip: String,
        password: String,
        author: String,
        replying_to: Option<usize>,
    ) -> ClientMessage {
        ClientMessage {
            replying_to: replying_to,
            MessageType: ClientMessageType::ClientNormalMessage(ClientNormalMessage {
                message: msg.trim().to_string(),
            }),
            Password: password,
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
            Destination: ip,
        }
    }

    //this is used when you want to send a file, this contains name, bytes
    pub fn construct_file_msg(
        file_name: PathBuf,
        ip: String,
        password: String,
        author: String,
        replying_to: Option<usize>,
    ) -> ClientMessage {
        ClientMessage {
            replying_to: replying_to,
            //Dont execute me please :3 |
            //                          |
            //                          V
            MessageType: ClientMessageType::ClientFileUpload(ClientFileUpload {
                extension: file_name.extension().unwrap().to_str().unwrap().to_string(),
                name: file_name
                    .file_prefix()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
                bytes: std::fs::read(file_name).unwrap_or_default(),
            }),

            Password: password,
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
            Destination: ip,
        }
    }

    //this is used for constructing a sync msg aka sending an empty packet, so server can reply
    pub fn construct_sync_msg(
        ip: String,
        password: String,
        author: String,
        replying_to: Option<usize>,
    ) -> ClientMessage {
        ClientMessage {
            replying_to: replying_to,
            MessageType: ClientMessageType::ClientSyncMessage(ClientSnycMessage {}),
            Password: password,
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
            Destination: ip,
        }
    }

    //this is used for asking for a file
    pub fn construct_file_request_msg(
        index: i32,
        password: String,
        author: String,
        ip: String,
        replying_to: Option<usize>,
    ) -> ClientMessage {
        ClientMessage {
            replying_to: replying_to,
            MessageType: ClientMessageType::ClientFileRequest(ClientFileRequest { index: index }),
            Password: password,
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
            Destination: ip,
        }
    }

    //this is used for asking for an image
    pub fn construct_image_request_msg(
        index: i32,
        password: String,
        author: String,
        ip: String,
        replying_to: Option<usize>,
    ) -> ClientMessage {
        ClientMessage {
            replying_to: replying_to,
            MessageType: ClientMessageType::ClientImageRequest(ClientImageRequest { index: index }),
            Password: password,
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
            Destination: ip,
        }
    }

    //this is used for SENDING IMAGES SO THE SERVER CAN DECIDE IF ITS A PICTURE
    pub fn construct_image_msg(
        file_path: PathBuf,
        ip: String,
        password: String,
        author: String,
        replying_to: Option<usize>,
    ) -> ClientMessage {
        ClientMessage {
            replying_to: replying_to,
            MessageType: ClientMessageType::ClientImage(ClientImage {
                bytes: fs::read(file_path).unwrap_or_default(),
            }),

            Password: password,
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
            Destination: ip,
        }
    }
}

/*
    Server. . .

    Are used to convert clinet sent messages into a server message, so it can be sent back;
    Therefor theyre smaller in size
*/

/*
        NOTICE:


    .... Upload : is always what the server sends back to the client (so the client knows what to ask about)

    .... Reply : is always what the server send to the client after the client asked.

*/

//This is what the server sends back (pushes to message vector), when reciving a file
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerFileUpload {
    pub file_name: String,
    pub index: i32,
}

//This is what the server sends back, when asked for a file (FIleRequest)
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerFileReply {
    pub bytes: Vec<u8>,
    pub file_name: PathBuf,
}

//This is what gets sent to a client basicly, and they have to ask for the file when the ui containin this gets rendered
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerImageUpload {
    pub index: i32,
}

//When client asks for the image based on the provided index, reply with the image bytes
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerImageReply {
    pub bytes: Vec<u8>,
    pub index: i32,
}

//This is what the server sends back (pushes to message vector), when reciving a normal message
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerNormalMessage {
    pub message: String,
}

//This is what server replies can be
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum ServerMessageType {
    Upload(ServerFileUpload),
    Normal(ServerNormalMessage),

    //Used to send and index to client so it knows which index to ask for VERY IMPORTANT!!!!!!!!!
    Image(ServerImageUpload),
}

//This is one whole server msg (packet), which gets bundled when sending ServerMain
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerOutput {
    pub replying_to: Option<usize>,
    pub MessageType: ServerMessageType,
    pub Author: String,
    pub MessageDate: String,
}
impl ServerOutput {
    pub fn struct_into_string(&self) -> String {
        return serde_json::to_string(self).unwrap_or_default();
    }
    pub fn convert_msg_to_servermsg(normal_msg: ClientMessage) -> ServerOutput {
        //Convert a client output to a server output (ClientMessage -> ServerOutput), trim some useless info
        ServerOutput {
            replying_to: normal_msg.replying_to,
            MessageType: ServerMessageType::Normal(ServerNormalMessage {
                message: match normal_msg.MessageType {
                    ClientMessageType::ClientSyncMessage(_) => todo!(),
                    ClientMessageType::ClientFileRequest(_) => todo!(),
                    ClientMessageType::ClientFileUpload(_) => todo!(),
                    ClientMessageType::ClientImage(_) => todo!(),
                    ClientMessageType::ClientNormalMessage(msg) => msg.message,
                    ClientMessageType::ClientImageRequest(_) => todo!(),
                },
            }),
            Author: normal_msg.Author,
            MessageDate: normal_msg.MessageDate,
        }
    }
    pub fn convert_picture_to_servermsg(normal_msg: ClientMessage, index: i32) -> ServerOutput {
        //Convert a client output to a server output (ClientMessage -> ServerOutput), trim some useless info
        ServerOutput {
            replying_to: normal_msg.replying_to,
            MessageType: ServerMessageType::Image(ServerImageUpload {
                index: match normal_msg.MessageType {
                    ClientMessageType::ClientSyncMessage(_) => todo!(),
                    ClientMessageType::ClientFileRequest(_) => todo!(),
                    ClientMessageType::ClientFileUpload(_) => todo!(),
                    ClientMessageType::ClientImage(_) => index,
                    ClientMessageType::ClientNormalMessage(_) => todo!(),
                    ClientMessageType::ClientImageRequest(_) => todo!(),
                },
            }),
            Author: normal_msg.Author,
            MessageDate: normal_msg.MessageDate,
        }
    }
    pub fn convert_upload_to_servermsg(normal_msg: ClientMessage, index: i32) -> ServerOutput {
        //Convert a client output to a server output (ClientMessage -> ServerOutput), trim some useless info
        ServerOutput {
            replying_to: normal_msg.replying_to,
            MessageType: ServerMessageType::Upload(ServerFileUpload {
                file_name: match normal_msg.MessageType {
                    ClientMessageType::ClientSyncMessage(_) => todo!(),
                    ClientMessageType::ClientFileRequest(_) => todo!(),
                    ClientMessageType::ClientFileUpload(msg) => {
                        format!("{}.{}", msg.name, msg.extension)
                    }
                    ClientMessageType::ClientImage(_) => todo!(),
                    ClientMessageType::ClientNormalMessage(_) => todo!(),
                    ClientMessageType::ClientImageRequest(_) => todo!(),
                },
                index: index,
            }),
            Author: normal_msg.Author,
            MessageDate: normal_msg.MessageDate,
        }
    }
}

//Used to put all the messages into 1 big pack (Bundling All the ServerOutput-s)
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerMaster {
    pub struct_list: Vec<ServerOutput>,
}
impl Default for ServerMaster {
    fn default() -> Self {
        Self {
            struct_list: Vec::new(),
        }
    }
}
impl ServerMaster {
    pub fn struct_into_string(&self) -> String {
        return serde_json::to_string(self).unwrap_or_default();
    }
    pub fn convert_vec_serverout_into_server_master(
        server_output_list: Vec<ServerOutput>,
    ) -> ServerMaster {
        return ServerMaster {
            struct_list: server_output_list,
        };
    }
}
