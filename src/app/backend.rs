use chrono::Utc;
use rand::rngs::ThreadRng;

use std::collections::BTreeMap;
use std::fs::{File, self};

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

//Message Types
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct FileUpload {
    pub extension: String,
    pub name: String,
    pub bytes: Vec<u8>,
}

#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct NormalMessage {
    pub message: String,
}

#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Image {
    pub bytes: Vec<u8>,
}

#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct SnycMessage {/*Empty packet, only for syncing*/}

#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct FileRequest {
    pub index: i32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum MessageType {
    SyncMessage(SnycMessage),
    FileRequest(FileRequest),
    FileUpload(FileUpload),
    Image(Image),
    NormalMessage(NormalMessage),
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Message {
    pub replying_to: Option<usize>,
    pub MessageType: MessageType,
    pub Password: String,
    pub Author: String,
    pub MessageDate: String,
    pub Destination: String,
}

impl Message {
    pub fn struct_into_string(&self) -> String {
        return serde_json::to_string(self).unwrap_or_default();
    }
    pub fn construct_normal_msg(
        msg: &str,
        ip: String,
        password: String,
        author: String,
        replying_to: Option<usize>,
    ) -> Message {
        Message {
            replying_to: replying_to,
            MessageType: MessageType::NormalMessage(NormalMessage {
                message: msg.trim().to_string(),
            }),
            Password: password,
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
            Destination: ip,
        }
    }
    pub fn construct_file_msg(
        file_name: PathBuf,
        ip: String,
        password: String,
        author: String,
        replying_to: Option<usize>,
    ) -> Message {
        Message {
            replying_to: replying_to,
            //Dont execute me please :3 |
            //                          |
            //                          V
            MessageType: MessageType::FileUpload(FileUpload {
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
    pub fn construct_sync_msg(
        ip: String,
        password: String,
        author: String,
        replying_to: Option<usize>,
    ) -> Message {
        Message {
            replying_to: replying_to,
            MessageType: MessageType::SyncMessage(SnycMessage {}),
            Password: password,
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
            Destination: ip,
        }
    }
    pub fn construct_file_request_msg(
        index: i32,
        password: String,
        author: String,
        ip: String,
        replying_to: Option<usize>,
    ) -> Message {
        Message {
            replying_to: replying_to,
            MessageType: MessageType::FileRequest(FileRequest { index: index }),
            Password: password,
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
            Destination: ip,
        }
    }
    pub fn construct_image_msg(
        file_name: PathBuf,
        ip: String,
        password: String,
        author: String,
        replying_to: Option<usize>,
    ) -> Message {
        Message {
            replying_to: replying_to,
            //Dont execute me please :3 |
            //                          |
            //                          V
            MessageType: MessageType::Image(Image { bytes: fs::read(file_name).unwrap_or_default() }),

            Password: password,
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
            Destination: ip,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerFileUpload {
    pub file_name: String,
    pub index: i32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerNormalMessage {
    pub message: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerImage {
    pub bytes: Vec<u8>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum ServerMessageType {
    Upload(ServerFileUpload),
    Normal(ServerNormalMessage),
    Image(ServerImage),
}

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
    pub fn convert_msg_to_servermsg(normal_msg: Message) -> ServerOutput {
        //Convert a client output to a server output (Message -> ServerOutput), trim some useless info
        ServerOutput {
            replying_to: normal_msg.replying_to,
            MessageType: ServerMessageType::Normal(ServerNormalMessage {
                message: match normal_msg.MessageType {
                    MessageType::SyncMessage(_) => todo!(),
                    MessageType::FileRequest(_) => todo!(),
                    MessageType::FileUpload(_) => todo!(),
                    MessageType::Image(_) => todo!(),
                    MessageType::NormalMessage(msg) => msg.message,
                },
            }),
            Author: normal_msg.Author,
            MessageDate: normal_msg.MessageDate,
        }
    }
    pub fn convert_picture_to_servermsg(normal_msg: Message) -> ServerOutput {
        //Convert a client output to a server output (Message -> ServerOutput), trim some useless info
        ServerOutput {
            replying_to: normal_msg.replying_to,
            MessageType: ServerMessageType::Image(ServerImage {
                bytes: match normal_msg.MessageType {
                    MessageType::SyncMessage(_) => todo!(),
                    MessageType::FileRequest(_) => todo!(),
                    MessageType::FileUpload(_) => todo!(),
                    MessageType::Image(img) => img.bytes,
                    MessageType::NormalMessage(_) => todo!(),
                },
            }),
            Author: normal_msg.Author,
            MessageDate: normal_msg.MessageDate,
        }
    }
    pub fn convert_upload_to_servermsg(normal_msg: Message, index: i32) -> ServerOutput {
        //Convert a client output to a server output (Message -> ServerOutput), trim some useless info
        ServerOutput {
            replying_to: normal_msg.replying_to,
            MessageType: ServerMessageType::Upload(ServerFileUpload {
                file_name: match normal_msg.MessageType {
                    MessageType::SyncMessage(_) => todo!(),
                    MessageType::FileRequest(_) => todo!(),
                    MessageType::FileUpload(msg) => {
                        format!("{}.{}", msg.name, msg.extension)
                    }
                    MessageType::Image(_) => todo!(),
                    MessageType::NormalMessage(_) => todo!(),
                },
                index: index,
            }),
            Author: normal_msg.Author,
            MessageDate: normal_msg.MessageDate,
        }
    }
}

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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct FileServe {
    pub bytes: Vec<u8>,
    pub file_name: PathBuf,
}
impl Default for FileServe {
    fn default() -> Self {
        Self {
            bytes: Vec::new(),
            file_name: PathBuf::new(),
        }
    }
}
