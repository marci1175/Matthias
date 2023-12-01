use chrono::Utc;
use rand::rngs::ThreadRng;

use std::collections::BTreeMap;
use std::fs::File;

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
    pub usr_msg_expanded: bool,
    pub send_on_ip: String,
    pub req_passw: bool,
    pub client_password: String,
    //font
    pub font_size: f32,

    //msg
    #[serde(skip)]
    pub usr_msg: String,
    #[serde(skip)]
    pub incoming_msg_time: Vec<String>,
    #[serde(skip)]
    pub incoming_msg: String,
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
            incoming_msg_time: Vec::new(),
            incoming_msg: String::new(),
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
#[derive(Default, serde::Serialize, serde::Deserialize, Debug)]
pub struct FileUpload {
    pub name: String,
    pub bytes: Vec<u8>,
}

#[derive(Default, serde::Serialize, serde::Deserialize, Debug)]
pub struct NormalMessage {
    pub message: String,
}

#[derive(Default, serde::Serialize, serde::Deserialize, Debug)]
pub struct Image {
    pub bytes: Vec<u8>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum MessageType {
    FileUpload(FileUpload),
    Image(Image),
    NormalMessage(NormalMessage),
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Message {
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
    ) -> Message {
        Message {
            MessageType: MessageType::NormalMessage(NormalMessage { message: msg.trim().to_string() }),
            Password: password,
            Author: author,
            MessageDate: {
                Utc::now().format("%Y.%m.%d. %H:%M").to_string()
            },
            Destination: ip,
        }
    }
}
