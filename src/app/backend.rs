use chrono::{DateTime, Utc};
use rand::rngs::ThreadRng;
use tokio_util::sync::CancellationToken;

use super::client::{connect_to_server, ServerReply};
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key,
};
use anyhow::{bail, ensure, Result};
use argon2::{Config, Variant, Version};
use base64::engine::general_purpose;
use base64::Engine;
use rfd::FileDialog;
use rodio::{OutputStream, OutputStreamHandle, Sink};
use std::collections::BTreeMap;
use std::env;
use std::fmt::{Debug, Display};
use std::fs;
use std::io;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::string::FromUtf8Error;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use strum::{EnumDiscriminants, EnumMessage};
use strum_macros::EnumString;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use windows_sys::w;
use windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW;
use windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONERROR;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    /*
        Font
    */
    ///fontbook
    pub filter: String,
    pub named_chars: BTreeMap<egui::FontFamily, BTreeMap<char, String>>,
    ///font
    pub font_size: f32,

    /*
    login page
    */
    ///the string entered to the username field on the login page
    pub login_username: String,

    #[serde(skip)]
    ///the string entered to the password field on the login page, dont save this one... obviously :)
    pub login_password: String,

    /*
        server main
    */
    ///SChecks whether server is already started TODO: FIX DUMB STUFF LIKE THIS, INSTEAD USE AN OPTION
    #[serde(skip)]
    pub server_has_started: bool,

    ///Public ip address, checked by pinging external website
    #[serde(skip)]
    pub public_ip: String,

    ///server settings
    pub server_req_password: bool,

    ///Server shutdown handler channel
    #[serde(skip)]
    pub server_shutdown_token: CancellationToken,

    ///What is the server's password set to
    pub server_password: String,

    ///Which port is the server open on
    pub open_on_port: String,

    ///thread communication for server
    #[serde(skip)]
    pub srx: mpsc::Receiver<String>,
    #[serde(skip)]
    pub stx: mpsc::Sender<String>,

    ///child windows
    #[serde(skip)]
    pub settings_window: bool,

    ///thread communication for image requesting
    #[serde(skip)]
    pub irx: mpsc::Receiver<String>,
    #[serde(skip)]
    pub itx: mpsc::Sender<String>,

    ///thread communication for audio recording
    #[serde(skip)]
    pub atx: Option<mpsc::Sender<bool>>,

    ///thread communication for audio ! SAVING !
    #[serde(skip)]
    pub audio_save_rx: mpsc::Receiver<(Option<Sink>, PlaybackCursor, usize, PathBuf)>,
    #[serde(skip)]
    pub audio_save_tx: mpsc::Sender<(Option<Sink>, PlaybackCursor, usize, PathBuf)>,

    /*
        main
    */
    pub main: Main,

    /*
        client main
    */
    pub client_ui: Client,

    #[serde(skip)]
    pub client_connection: ClientConnection,

    ///thread communication for client
    #[serde(skip)]
    pub rx: mpsc::Receiver<String>,
    #[serde(skip)]
    pub tx: mpsc::Sender<String>,

    ///data sync
    #[serde(skip)]
    pub drx: mpsc::Receiver<String>,
    #[serde(skip)]
    pub dtx: mpsc::Sender<String>,

    ///Server connection
    /// This channel hosts a Client connection and the sync message sent by the server in a String format
    #[serde(skip)]
    pub connection_reciver: mpsc::Receiver<Option<(ClientConnection, String)>>,
    #[serde(skip)]
    pub connection_sender: mpsc::Sender<Option<(ClientConnection, String)>>,

    ///Server - client syncing thread
    #[serde(skip)]
    pub server_sender_thread: Option<()>,

    #[serde(skip)]
    /// This is what the main thread uses to recive messages from the sync thread
    pub server_output_reciver: Receiver<Option<String>>,
    #[serde(skip)]
    /// This is what the sync thread uses to send messages to the main thread
    pub server_output_sender: Sender<Option<String>>,

    #[serde(skip)]
    /// This is what the sync thread thread uses to recive a shutdown message from the main
    pub autosync_input_reciver: tokio::sync::broadcast::Receiver<()>,
    #[serde(skip)]
    /// This is what the main thread uses to send the shutdown message to the sync thread
    pub autosync_input_sender: tokio::sync::broadcast::Sender<()>,

    #[serde(skip)]
    pub audio_file: Arc<Mutex<PathBuf>>,

    #[serde(skip)]
    pub opened_account: OpenedAccount,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel::<String>();
        let (stx, srx) = mpsc::channel::<String>();
        let (dtx, drx) = mpsc::channel::<String>();
        let (itx, irx) = mpsc::channel::<String>();
        let (audio_save_tx, audio_save_rx) =
            mpsc::channel::<(Option<Sink>, PlaybackCursor, usize, PathBuf)>();

        let (connection_sender, connection_reciver) =
            mpsc::channel::<Option<(ClientConnection, String)>>();

        let (server_output_sender, server_output_reciver) = mpsc::channel::<Option<String>>();

        let (autosync_input_sender, autosync_input_reciver) = tokio::sync::broadcast::channel(1);

        Self {
            audio_file: Arc::new(Mutex::new(PathBuf::from(format!(
                "{}\\Matthias\\Client\\voice_recording.wav",
                env!("APPDATA")
            )))),

            //fontbook
            filter: Default::default(),
            named_chars: Default::default(),

            //login page
            login_username: String::new(),
            login_password: String::new(),

            //server_main
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

            //thread communication for image requesting
            irx,
            itx,

            //This default value will get overwritten when crating the new server, so we can pass in the token to the thread
            //Also, the shutdown reciver is unnecessary in this context because we never use it, I too lazy to delete a few lines instead of writing this whole paragraph >:D
            server_shutdown_token: CancellationToken::new(),

            //thread communication for audio recording
            atx: None,

            //thread communication for audio saving
            audio_save_rx,
            audio_save_tx,

            //main
            main: Main::default(),

            //client main
            client_ui: Client::default(),

            client_connection: ClientConnection::default(),

            //font
            font_size: 20.,

            //emoji button

            //thread communication for client
            rx,
            tx,

            //Server connection
            connection_sender,
            connection_reciver,

            //data sync
            drx,
            dtx,
            server_sender_thread: None,

            server_output_reciver,
            server_output_sender,

            autosync_input_reciver,
            autosync_input_sender,

            opened_account: OpenedAccount::default(),
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

/*Children structs*/
///Children struct
/// Client Ui
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Client {
    ///When a text_edit_cursor move has been requested this value is a Some
    #[serde(skip)]
    pub text_edit_cursor: Option<usize>,

    ///The rect of the connected users list (which gets displayed when pressing the @)
    #[serde(skip)]
    pub connected_users_display_rect: Option<egui::Rect>,

    ///After pressing @ and the user list pops out, the code logs the up arrow and down arroy actions and increments/ decreases the value, resets after pressing @ again
    #[serde(skip)]
    pub user_selector_index: i32,

    #[serde(skip)]
    pub display_user_list: bool,

    ///Search parameters set by user, to chose what to search for obviously
    pub search_parameter: SearchType,

    ///Search buffer
    #[serde(skip)]
    pub search_buffer: String,

    ///Check if search panel is open
    #[serde(skip)]
    pub search_mode: bool,

    ///audio playback
    #[serde(skip)]
    pub audio_playback: AudioPlayback,

    ///this doesnt really matter if we save or no so whatever, implements scrolling to message element
    #[serde(skip)]
    pub scroll_to_message: Option<ScrollToMessage>,

    ///index of the reply the user clicked on
    #[serde(skip)]
    pub scroll_to_message_index: Option<usize>,

    ///Selected port on sending
    pub send_on_port: String,

    ///Selected ip address (without port as seen above)
    pub send_on_address: String,

    ///This is used when the client entered a false password to connect with to the server
    #[serde(skip)]
    pub invalid_password: bool,

    ///This is set to on when an image is enlarged
    #[serde(skip)]
    pub image_overlay: bool,

    ///Scroll widget rect, text editor's rect
    pub scroll_widget_rect: egui::Rect,

    ///This decides how wide the text editor should be, ensure it doesnt overlap with "msg_action_tray" (the action buttons :) )
    pub text_widget_offset: f32,

    ///A vector of all the added files to the buffer, these are the PathBufs which get read, then their bytes get sent
    #[serde(skip)]
    pub files_to_send: Vec<PathBuf>,

    ///This checks if the text editor is open or not
    pub usr_msg_expanded: bool,

    ///This is the full address of the destionation a message is supposed to be sent to
    pub send_on_ip: String,

    ///self.send_on_ip encoded into base64, this is supposedly for ease of use, I dont know why its even here
    pub send_on_ip_base64_encoded: String,

    ///Does client have the password required checkbox ticked
    pub req_passw: bool,

    ///The password the user has entered for server auth
    pub client_password: String,

    ///This gem of a variable is used to contain animation's state
    pub animation_state: f32,

    ///This checks if a file is dragged above Matthias, so it knows when to display the cool animation 8)
    #[serde(skip)]
    pub drop_file_animation: bool,

    /// This field sets the message edit mode
    /// 3 Enums:
    /// Normal
    /// Reply(. . .)
    /// Edit(. . .)
    #[serde(skip)]
    pub messaging_mode: MessagingMode,

    ///Input (Múlt idő) user's message, this is what gets modified in the text editor
    #[serde(skip)]
    pub usr_msg: String,

    ///Incoming messages, this is the whole packet which get sent to all the clients, this cointains all the messages, and the info about them
    #[serde(skip)]
    pub incoming_msg: ServerMaster,

    /// Last seen message's index, this will get sent
    #[serde(skip)]
    pub last_seen_msg_index: Arc<Mutex<usize>>,

    ///emoji fasz
    pub random_emoji: String,
    pub emoji: Vec<String>,

    ///Random generating engine
    #[serde(skip)]
    pub rand_eng: ThreadRng,

    ///Used to decide whether the reactive emoji button should switch emojis (Like discords implementation)
    pub random_generated: bool,

    ///Log when the voice recording has been started so we know how long the recording is
    #[serde(skip)]
    pub voice_recording_start: Option<DateTime<Utc>>,
}
impl Default for Client {
    fn default() -> Self {
        Self {
            text_edit_cursor: None,
            messaging_mode: MessagingMode::Normal,
            connected_users_display_rect: None,

            user_selector_index: 0,
            display_user_list: false,

            search_parameter: SearchType::default(),
            search_buffer: String::new(),
            search_mode: false,
            //audio playback
            audio_playback: AudioPlayback::default(),
            scroll_widget_rect: egui::Rect::NAN,
            text_widget_offset: 0.0,
            scroll_to_message_index: None,
            scroll_to_message: None,
            send_on_port: String::new(),
            send_on_address: String::new(),
            invalid_password: false,
            image_overlay: false,
            files_to_send: Vec::new(),
            animation_state: 0.0,
            drop_file_animation: false,
            usr_msg_expanded: false,
            send_on_ip: String::new(),
            send_on_ip_base64_encoded: String::new(),
            req_passw: false,
            client_password: String::new(),
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
            incoming_msg: ServerMaster::default(),

            voice_recording_start: None,
            last_seen_msg_index: Arc::new(Mutex::new(0)),
        }
    }
}

///Main, Global stuff
#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct Main {
    ///Checks if the emoji tray is on
    #[serde(skip)]
    pub emoji_mode: bool,

    ///Checks if bookmark mode is turned on
    #[serde(skip)]
    pub bookmark_mode: bool,

    ///Client mode main switch
    #[serde(skip)]
    pub client_mode: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone, Debug)]
pub enum MessagingMode {
    #[default]
    Normal,
    /// The inner value of this enum holds the index which this message is editing
    Edit(usize),
    /// The inner value of this enum holds the index which this message is replying to
    Reply(usize),
}

impl MessagingMode {
    pub fn get_reply_index(&self) -> Option<usize> {
        match self {
            MessagingMode::Reply(i) => Some(*i),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
///Opened account attributes, doesnt contain anything which might change at runtime
pub struct OpenedAccount {
    pub uuid: String,
    pub username: String,
    pub path: PathBuf,
}

impl OpenedAccount {
    pub fn new(uuid: String, username: String, path: PathBuf) -> Self {
        Self {
            uuid,
            username,
            path,
        }
    }
}

///When the client is uploading a file, this packet gets sent
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientFileUpload {
    pub extension: Option<String>,
    pub name: Option<String>,
    pub bytes: Vec<u8>,
}

///Normal message
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientNormalMessage {
    pub message: String,
}

// Used for syncing or connecting & disconnecting
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientSnycMessage {
    /// If its None its used for syncing, false: disconnecting, true: connecting
    /// If you have already registered the client with the server then the true value will be ignored
    pub sync_attribute: Option<bool>,

    ///This is used to tell the server how many messages it has to send, if its a None it will automaticly sync all messages
    /// This value is ignored if the `sync_attribute` field is Some(_)
    pub client_message_counter: Option<usize>,

    /// The index of the last seen message by the user, this is sent so we can display which was the last message the user has seen, if its None we ignore the value
    pub last_seen_message_index: Option<usize>,

    ///Contain password in the sync message, so we will send the password when authenticating
    pub password: String,
}

///This is used by the client for requesting file
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientFileRequest {
    pub index: i32,
}

///This is used by the client for requesting images
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientImageRequest {
    pub index: i32,
}

///Client requests audio file in server
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientAudioRequest {
    pub index: i32,
}

///Reaction packet, defines which message its reacting to and with which char
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientReaction {
    pub char: char,
    pub message_index: usize,
}

///Lets the client edit their *OWN* message, a client check is implemented TODO: please write a server check for this
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientMessageEdit {
    ///The message which is edited
    pub index: usize,
    ///The new message
    pub new_message: Option<String>,
}

///These are the types of requests the client can ask
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum ClientFileRequestType {
    ///this is when you want to display an image and you have to make a request to the server file
    ClientImageRequest(ClientImageRequest),
    ClientFileRequest(ClientFileRequest),
    ClientAudioRequest(ClientAudioRequest),
}

///Client outgoing message types
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum ClientMessageType {
    ClientFileRequestType(ClientFileRequestType),

    ClientFileUpload(ClientFileUpload),

    ///Normal msg
    ClientNormalMessage(ClientNormalMessage),

    ///Used for syncing with client and server
    ClientSyncMessage(ClientSnycMessage),

    ClientReaction(ClientReaction),

    ClientMessageEdit(ClientMessageEdit),
}

///This is what gets to be sent out by the client
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientMessage {
    pub replying_to: Option<usize>,
    pub MessageType: ClientMessageType,
    pub Uuid: String,
    pub Author: String,
    pub MessageDate: String,
}

impl ClientMessage {
    ///struct into string, it makes sending information easier by putting it all in a string
    pub fn struct_into_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    ///this is used when sending a normal message
    pub fn construct_normal_msg(
        msg: &str,
        uuid: &str,
        author: &str,
        replying_to: Option<usize>,
    ) -> ClientMessage {
        ClientMessage {
            replying_to,
            MessageType: ClientMessageType::ClientNormalMessage(ClientNormalMessage {
                message: msg.trim().to_string(),
            }),
            //If the password is set as None (Meaning the user didnt enter any password) just send the message with an empty string
            Uuid: uuid.to_string(),
            Author: author.to_string(),
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    ///this is used when you want to send a file, this contains name, bytes
    pub fn construct_file_msg(
        file_path: PathBuf,
        uuid: &str,
        author: &str,
        replying_to: Option<usize>,
    ) -> ClientMessage {
        ClientMessage {
            replying_to,
            //Dont execute me please :3 |
            //                          |
            //                          V
            MessageType: ClientMessageType::ClientFileUpload(ClientFileUpload {
                extension: Some(file_path.extension().unwrap().to_str().unwrap().to_string()),
                name: Some(
                    file_path
                        .file_prefix()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string(),
                ),
                bytes: std::fs::read(file_path).unwrap_or_default(),
            }),

            Uuid: uuid.to_string(),
            Author: author.to_string(),
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    pub fn construct_reaction_msg(
        char: char,
        index: usize,
        author: &str,
        uuid: &str,
    ) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            MessageType: ClientMessageType::ClientReaction(ClientReaction {
                char,
                message_index: index,
            }),
            Uuid: uuid.to_string(),
            Author: author.to_string(),
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    /// this is used for constructing a sync msg aka sending an empty packet, so server can reply
    /// If its None its used for syncing, false: disconnecting, true: connecting
    pub fn construct_sync_msg(
        password: &str,
        author: &str,
        uuid: &str,
        client_message_counter: usize,
        last_seen_message_index: Option<usize>,
    ) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            MessageType: ClientMessageType::ClientSyncMessage(ClientSnycMessage {
                sync_attribute: None,
                password: password.to_string(),
                //This value is not ignored in this context
                client_message_counter: Some(client_message_counter),
                last_seen_message_index,
            }),
            Uuid: uuid.to_string(),
            Author: author.to_string(),
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    /// If its None its used for syncing, false: disconnecting, true: connecting
    pub fn construct_connection_msg(
        password: String,
        author: String,
        uuid: &str,
        last_seen_message_index: Option<usize>,
    ) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            MessageType: ClientMessageType::ClientSyncMessage(ClientSnycMessage {
                sync_attribute: Some(true),
                password,
                //If its used for connecting / disconnecting this value is ignored
                client_message_counter: None,
                last_seen_message_index,
            }),
            Uuid: uuid.to_string(),
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    /// If its None its used for syncing, false: disconnecting, true: connecting
    /// Please note that its doesnt really matter what we pass in the author becuase the server identifies us based on our ip address
    pub fn construct_disconnection_msg(
        password: String,
        author: String,
        uuid: String,
    ) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            MessageType: ClientMessageType::ClientSyncMessage(ClientSnycMessage {
                sync_attribute: Some(false),
                password,
                //If its used for connecting / disconnecting this value is ignored
                client_message_counter: None,
                last_seen_message_index: None,
            }),
            Uuid: uuid,
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    ///this is used for asking for a file
    pub fn construct_file_request_msg(index: i32, uuid: &str, author: String) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            MessageType: ClientMessageType::ClientFileRequestType(
                ClientFileRequestType::ClientFileRequest(ClientFileRequest { index }),
            ),
            Uuid: uuid.to_string(),
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    ///this is used for asking for an image
    pub fn construct_image_request_msg(index: i32, uuid: &str, author: String) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            MessageType: ClientMessageType::ClientFileRequestType(
                ClientFileRequestType::ClientImageRequest(ClientImageRequest { index }),
            ),
            Uuid: uuid.to_string(),
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    ///this is used for asking for an image
    pub fn construct_audio_request_msg(index: i32, uuid: &str, author: String) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            MessageType: ClientMessageType::ClientFileRequestType(
                ClientFileRequestType::ClientAudioRequest(ClientAudioRequest { index }),
            ),
            Uuid: uuid.to_string(),
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    pub fn construct_client_message_edit(
        index: usize,
        new_message: Option<String>,
        uuid: &str,
        author: &str,
    ) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            MessageType: ClientMessageType::ClientMessageEdit(ClientMessageEdit {
                index,
                new_message,
            }),
            Uuid: uuid.to_string(),
            Author: author.to_string(),
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    //this is used for SENDING IMAGES SO THE SERVER CAN DECIDE IF ITS A PICTURE
    //NOTICE: ALL THE AUDIO UPLOAD TYPES HAVE BEEN CONVERTED INTO ONE => "ClientFileUpload" this ensures that the client doesnt handle any backend stuff
}

///This manages all the settings and variables for maintaining a connection with the server (from client)
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct ClientConnection {
    #[serde(skip)]
    pub client_secret: Vec<u8>,

    #[serde(skip)]
    ///This enum wraps the server handle ```Connected(_)```, it also functions as a Sort of Option wrapper
    pub state: ConnectionState,

    #[serde(skip)]
    //Password which was used to connect (and could connect with, it has been password matched with the server)
    pub password: String,
}

impl ClientConnection {
    /// This is a wrapper function for ```client::send_message```
    pub async fn send_message(self, message: ClientMessage) -> anyhow::Result<ServerReply> {
        if let ConnectionState::Connected(connection) = &self.state {
            #[allow(unused_must_use)]
            {
                Ok(connection.send_message(message).await?)
            }
        } else {
            bail!("There is no active connection to send the message on.")
        }
    }

    /// Ip arg to know where to connect, username so we can register with the sever, used to spawn a valid ClientConnection instance
    /// This function blocks (time depends on the connection speed)
    /// This function also hashes the password argument which it sends, and then if the connection was successful the returned struct's password field will contain the already hashed password
    pub async fn connect(
        ip: String,
        author: String,
        password: Option<String>,
        uuid: &str,
    ) -> anyhow::Result<(Self, String)> {
        let hashed_password = encrypt(password.clone().unwrap_or(String::from("")));
        let connection_msg = ClientMessage::construct_connection_msg(
            hashed_password.clone(),
            author.clone(),
            uuid,
            None,
        );

        //Ping server to recive custom uuid, and to also get if server ip is valid
        let client_handle = tokio::net::TcpStream::connect(ip).await?;

        /*We could return this, this is what the server is supposed to return, when a new user is connected */
        let (server_reply, server_handle) =
            connect_to_server(client_handle, connection_msg).await?;

        ensure!(server_reply != "Invalid Password!", "Invalid password!");
        ensure!(
            server_reply != "Invalid Client!",
            "Outdated client or connection!"
        );

        //This the key the server replied, and this is what well need to decrypt the messages, overwrite the client_secret variable
        let client_secret = hex::decode(server_reply)?;

        //Create connection pair
        let (reader, writer) = server_handle.into_split();

        let connection_pair = ConnectionPair::new(writer, reader);

        //Sync with the server
        let sync_message =
            ClientMessage::construct_sync_msg(&hashed_password, &author, uuid, 0, None);

        let server_response = connection_pair
            .send_message(sync_message)
            .await?
            .wait_for_response()
            .await?;

        //This contains the sync string
        let server_reply = decrypt_aes256(&server_response, &client_secret)
            .expect("Failed to decrypt server sync packet");

        Ok((
            Self {
                client_secret,
                state: ConnectionState::Connected(connection_pair),
                password: hashed_password,
            },
            server_reply,
        ))
    }

    pub fn reset_state(&mut self) {
        self.client_secret = Vec::new();
        self.state = ConnectionState::default();
    }

    /// This function is used to __DISCONNECT__ from a server, with this the ```ClientConnection``` instance is destoryed (reset to its default values)
    pub async fn disconnect(
        &mut self,
        author: String,
        password: String,
        uuid: String,
    ) -> anyhow::Result<()> {
        if let ConnectionState::Connected(connection) = &self.state {
            //We pray it doesnt deadlock, amen
            connection
                .send_message(ClientMessage::construct_disconnection_msg(
                    password, author, uuid,
                ))
                .await?;

            //Shutdown connection from the client side
            connection.writer.lock().await.shutdown().await?;
        } else {
            bail!("There is no active connection to send the message on.")
        }

        //Reset state
        self.reset_state();

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ConnectionPair {
    pub writer: Arc<tokio::sync::Mutex<OwnedWriteHalf>>,
    pub reader: Arc<tokio::sync::Mutex<OwnedReadHalf>>,
}

impl ConnectionPair {
    pub fn new(writer: OwnedWriteHalf, reader: OwnedReadHalf) -> Self {
        Self {
            writer: Arc::new(tokio::sync::Mutex::new(writer)),
            reader: Arc::new(tokio::sync::Mutex::new(reader)),
        }
    }

    pub async fn send_message(&self, message: ClientMessage) -> anyhow::Result<ServerReply> {
        let mut writer: tokio::sync::MutexGuard<'_, OwnedWriteHalf> = self.writer.lock().await;

        let message_string = message.struct_into_string();

        let message_bytes = message_string.as_bytes();

        //Send message lenght to server
        writer
            .write_all(&(message_bytes.len() as u32).to_be_bytes())
            .await?;

        //Send message to server
        writer.write_all(message_bytes).await?;

        writer.flush().await?;

        Ok(ServerReply::new(self.reader.clone()))
    }
}

///Used to show state of the connection
#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub enum ConnectionState {
    #[serde(skip)]
    Connected(ConnectionPair),

    #[default]
    Disconnected,
    Connecting,
    Error,
}

impl Debug for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ConnectionState::Connected(_) => "Connected",
            ConnectionState::Disconnected => "Disconnected",
            ConnectionState::Connecting => "Connecting",
            ConnectionState::Error => "Error",
        })
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

///This is what the server sends back (pushes to message vector), when reciving a file
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct ServerFileUpload {
    pub file_name: String,
    pub index: i32,
}

/// This enum holds all the Server reply types so it can be decoded more easily on the client side
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum ServerReplyType {
    FileReply(ServerFileReply),
    ImageReply(ServerImageReply),
    AudioReply(ServerAudioReply),
}

///When client asks for the image based on the provided index, reply with the image bytes
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerImageReply {
    pub bytes: Vec<u8>,
    pub index: i32,
}

///This is what the server sends back, when asked for a file (FIleRequest)
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerFileReply {
    pub bytes: Vec<u8>,
    pub file_name: PathBuf,
}

///When client asks for the image based on the provided index, reply with the audio bytes, which gets written so it can be opened by a readbuf
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerAudioReply {
    pub bytes: Vec<u8>,
    pub index: i32,
    pub file_name: String,
}

///This is what the server sends back (pushes to message vector), when reciving a normal message
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct ServerNormalMessage {
    pub has_been_edited: bool,
    pub message: String,
}

///REFER TO -> ServerImageUpload; logic      ||      same thing but with audio files
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct ServerAudioUpload {
    pub index: i32,
    pub file_name: String,
}

///This is what gets sent to a client basicly, and they have to ask for the file when the ui containin this gets rendered
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct ServerImageUpload {
    pub index: i32,
}

/// This struct contains all the important information for the client to edit / update its own message list
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct ServerMessageEdit {
    /// The message's index it belongs to
    pub index: i32,

    /// None indicates a deleted message, rest is self explanatory
    pub new_message: Option<String>,
}

/// This struct contains all the necesarily information for the client to update its own message list's reactions
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct ServerMessageReaction {
    /// The message's index it belongs to
    pub index: i32,

    /// The char added to the message specified by the index field
    pub char: char,
}

/// This struct is empty as its just a placeholder, because the info is provided in the struct which this message is wrapped in, and is provided directly when sending a message from the server to the client
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct ServerMessageSync {}

/// These are the possible server replies
/// Why do we have to implement PartialEq for all of the structs? This is so funny
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, EnumDiscriminants, PartialEq)]
#[strum_discriminants(derive(EnumString, EnumMessage))]
pub enum ServerMessageType {
    #[strum_discriminants(strum(message = "Upload"))]
    Upload(ServerFileUpload),
    #[strum_discriminants(strum(message = "Normal"))]
    Normal(ServerNormalMessage),

    ///Used to send and index to client so it knows which index to ask for
    ///The index provided by this enum
    #[strum_discriminants(strum(message = "Image"))]
    Image(ServerImageUpload),
    #[strum_discriminants(strum(message = "Audio"))]
    Audio(ServerAudioUpload),

    ///When a message is deleted this is what gets displayed
    #[strum_discriminants(strum(message = "Deleted"))]
    Deleted,

    ///This message indicates an edit in the server's message list, therefor we need to send a message to the client so that the client will update its own list
    #[strum_discriminants(strum(message = "Edit"))]
    Edit(ServerMessageEdit),

    ///This message indicates an edit in the server's message list, therefor we need to send a message to the client so that the client will update its own list
    #[strum_discriminants(strum(message = "Reaction"))]
    Reaction(ServerMessageReaction),

    /// This message is used to "sync" with the client, it provides useful information to the client about other clients (like last viewed message)
    #[strum_discriminants(strum(message = "Sync"))]
    Sync(ServerMessageSync),
}

///This struct contains all the reactions of one message
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct MessageReaction {
    pub message_reactions: Vec<Reaction>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Reaction {
    pub char: char,
    pub times: i64,
}

///This is one msg (packet), which gets bundled when sending ServerMain
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerOutput {
    /// Which message is this a reply to?
    /// The server stores all messages in a vector so this index shows which message its a reply to (if it is)
    pub replying_to: Option<usize>,
    /// Inner message which is *wrapped* in the ServerOutput
    pub MessageType: ServerMessageType,
    /// The account's name who sent the message
    pub Author: String,
    /// The date when this message was sent
    pub MessageDate: String,
    /// The user who sent this message's uuid
    pub uuid: String,
}

impl ServerOutput {
    pub fn _struct_into_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// This function convert a client message to a ServerOutput, which gets sent to all the clients (Its basicly a simplified client message)
    pub fn convert_type_to_servermsg(
        normal_msg: ClientMessage,
        // The index is used to ask bytes from the server, for example in a image message this index will be used to get the image's byte
        index: i32,
        //Automaticly generated enum by strum
        upload_type: ServerMessageTypeDiscriminants,
        uuid: String,
    ) -> ServerOutput {
        ServerOutput {
            replying_to: normal_msg.replying_to,
            MessageType:
                match normal_msg.MessageType {
                    ClientMessageType::ClientFileRequestType(_) => unimplemented!("Converting request packets isnt implemented, because they shouldnt be displayed by the client"),
                    ClientMessageType::ClientFileUpload(upload) => {
                        //The reason it doesnt panic if for example it a normal message because, an Upload can never be a:
                        //  Normal message
                        //  Message edit
                        //  Reaction to a message

                        match upload_type {
                            ServerMessageTypeDiscriminants::Upload => {
                                ServerMessageType::Upload(
                                    ServerFileUpload {
                                        file_name: format!(
                                            "{}.{}",
                                            upload.name.unwrap_or_default(),
                                            upload.extension.unwrap_or_default()
                                        ),
                                        index,
                                    }
                                )
                            },
                            ServerMessageTypeDiscriminants::Image => {
                                ServerMessageType::Image(
                                    ServerImageUpload {
                                        index,
                                    }
                                )
                            },
                            ServerMessageTypeDiscriminants::Audio => {
                                ServerMessageType::Audio(
                                    ServerAudioUpload {
                                        index,
                                        file_name: format!(
                                            "{}.{}",
                                            upload.name.unwrap_or_default(),
                                            upload.extension.unwrap_or_default()
                                        ),
                                    }
                                )
                            },
                            ServerMessageTypeDiscriminants::Deleted => unreachable!(),
                            ServerMessageTypeDiscriminants::Sync => unreachable!(),
                            ServerMessageTypeDiscriminants::Normal => unreachable!(),
                            ServerMessageTypeDiscriminants::Edit => unreachable!(),
                            ServerMessageTypeDiscriminants::Reaction => unreachable!(),
                        }
                    },
                    ClientMessageType::ClientNormalMessage(message) => {
                        ServerMessageType::Normal(
                            ServerNormalMessage {
                                message: message.message,
                                //Set default value for incoming messages
                                has_been_edited: false,
                            }
                        )
                    },
                    ClientMessageType::ClientSyncMessage(_) => {
                        ServerMessageType::Sync(ServerMessageSync {  })
                    },
                    //These messages also have a side effect on the server's list of the messages
                    //The client will interpret these messages and modify its own message list
                    ClientMessageType::ClientReaction(message) => {
                        ServerMessageType::Reaction(ServerMessageReaction { index: message.message_index as i32, char: message.char })
                    },
                    ClientMessageType::ClientMessageEdit(message) => {
                        ServerMessageType::Edit(ServerMessageEdit { index: message.index as i32, new_message: message.new_message })
                    },
                },
            Author: normal_msg.Author,
            MessageDate: normal_msg.MessageDate,
            uuid,
        }
    }
}

///Used to put all the messages into 1 big pack (Bundling All the ServerOutput-s), Main packet, this gets to all the clients
/// This message type is only used when a client is connecting an has to do a full sync (sending everything to the client all the messages reactions, etc)
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct ServerMaster {
    ///All of the messages recived from the server
    pub struct_list: Vec<ServerOutput>,

    ///All of the messages' reactions are
    pub reaction_list: Vec<MessageReaction>,

    ///Users last seen message index
    pub user_seen_list: Vec<ClientLastSeenMessage>,
}

impl ServerMaster {
    pub fn struct_into_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

///This struct provides all the necessary information to keep the client and the server in sync
/// Its struct contains ```Vec<ClientLastSeenMessage>``` which is for displaying which message has the user seen
/// And the message the client has sent
/// We dont need to provide any other information since, the ```ServerMaster``` struct ensures all the clients have the same field when connecting
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerSync {
    ///Users last seen message index
    pub user_seen_list: Vec<ClientLastSeenMessage>,

    pub message: ServerOutput,
}

impl ServerSync {
    pub fn struct_into_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

//When a client is connected this is where the client gets saved
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ConnectedClient {
    ///This handle wouldnt have to be sent so its all okay, its only present on the server's side
    #[serde(skip)]
    pub handle: Option<Arc<tokio::sync::Mutex<OwnedWriteHalf>>>,
    pub uuid: String,
    pub username: String,
}

impl ConnectedClient {
    pub fn new(
        uuid: String,
        username: String,
        handle: Arc<tokio::sync::Mutex<OwnedWriteHalf>>,
    ) -> Self {
        Self {
            uuid,
            username,
            handle: Some(handle),
        }
    }
}

//This contains the client's name and their last seen message's index
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientLastSeenMessage {
    pub index: usize,
    pub username: String,
    pub uuid: String,
}

impl ClientLastSeenMessage {
    pub fn new(index: usize, uuid: String, username: String) -> Self {
        Self {
            index,
            username,
            uuid,
        }
    }
}

/*
 Client backend
*/

///Struct for global audio playback
pub struct AudioPlayback {
    ///Output stream
    pub stream: OutputStream,
    ///Output stream handle
    pub stream_handle: OutputStreamHandle,
    ///Audio sinks, these are the audios played
    pub sink_list: Vec<Option<Sink>>,
    ///Settings list for the sink_list (The audios being played)
    pub settings_list: Vec<AudioSettings>,
}

impl Default for AudioPlayback {
    fn default() -> Self {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        Self {
            stream,
            stream_handle,
            sink_list: Vec::new(),
            settings_list: Vec::new(),
        }
    }
}

///This is used by the audio player, this is where you can set the speed and volume etc
pub struct AudioSettings {
    ///Volume for audio stream
    pub volume: f32,
    ///Speed for audio stream
    pub speed: f32,
    ///Reader cursor, for reading the sound file
    pub cursor: PlaybackCursor,

    ///This is only for ui usage
    pub is_loading: bool,

    ///Path to audio file
    pub path_to_audio: PathBuf,
}

///Initialize default values
impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            volume: 0.8,
            speed: 1.,
            cursor: PlaybackCursor::new(Vec::new()),
            is_loading: false,
            path_to_audio: PathBuf::new(),
        }
    }
}

/*
Maunally create a struct which implements the following traits:
                                                            Read
                                                            Seek

So it can be used as a Arc<Mutex<()>>
*/
#[derive(Clone)]
pub struct PlaybackCursor {
    pub cursor: Arc<Mutex<io::Cursor<Vec<u8>>>>,
}

///Impl new so It can probe a file (in `vec<u8>` format)
impl PlaybackCursor {
    pub fn new(data: Vec<u8>) -> Self {
        let cursor = Arc::new(Mutex::new(io::Cursor::new(data)));
        PlaybackCursor { cursor }
    }
}

///Implement the Read trait
impl Read for PlaybackCursor {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut cursor = self.cursor.lock().unwrap();
        std::io::Read::read(&mut *cursor, buf)
    }
}

///Implement the Seek trait
impl Seek for PlaybackCursor {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let mut cursor = self.cursor.lock().unwrap();
        std::io::Seek::seek(&mut *cursor, pos)
    }
}

pub struct ScrollToMessage {
    pub messages: Vec<egui::Response>,
    pub index: usize,
}

impl ScrollToMessage {
    pub fn new(messages: Vec<egui::Response>, index: usize) -> ScrollToMessage {
        ScrollToMessage { messages, index }
    }
}

/*
    Client
*/
///Used to decide what to search for (In the Message search bar), defined by the user
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
pub enum SearchType {
    Date,
    File,
    #[default]
    Message,
    Name,
    Reply,
}

///Implement display for SearchType so its easier to display
impl Display for SearchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SearchType::Name => "Name",
            SearchType::Message => "Message",
            SearchType::Date => "Date",
            SearchType::Reply => "Replies",
            SearchType::File => "File",
        })
    }
}

///Get ipv4 ip address from an external website
pub fn ipv4_get() -> Result<String, std::io::Error> {
    // Send an HTTP GET request to a service that returns your public IPv4 address
    let response = reqwest::blocking::get("https://ipv4.icanhazip.com/");
    // Check if the request was successful
    if response.is_ok() {
        let public_ipv4 = response.unwrap().text();

        Ok(public_ipv4.unwrap())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "Failed to fetch ip address",
        ))
    }
}

///Get ipv6 ip address from an external website
pub fn ipv6_get() -> Result<String, std::io::Error> {
    // Send an HTTP GET request to a service that returns your public IPv4 address
    let response = reqwest::blocking::get("https://ipv6.icanhazip.com/");
    // Check if the request was successful
    if response.is_ok() {
        let public_ipv4 = response.unwrap().text();

        Ok(public_ipv4.unwrap())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "Failed to fetch ip address",
        ))
    }
}

/// Account management
/// struct containing a new user's info, when serialized / deserialized it gets encrypted or decrypted
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct UserInformation {
    /// username
    pub username: String,
    /// IMPORTANT: PASSWORD *IS* ENCRYPTED BY FUNCTIONS IMPLEMENTED BY THIS TYPE
    pub password: String,
    /// uuids are encrypted by the new function
    pub uuid: String,
    /// bookmarked ips are empty by default, IMPORTANT: THESE ARE *NOT* ENCRYPTED BY DEFAULT
    pub bookmarked_ips: Vec<String>,
}

impl UserInformation {
    ///All of the args are encrypted
    pub fn new(username: String, password: String, uuid: String) -> Self {
        Self {
            username,
            password: encrypt(password),
            uuid,
            bookmarked_ips: Vec::new(),
        }
    }

    /// Automaticly check hash with argon2 encrypted password (from the file)
    pub fn verify_password(&self, password: String) -> bool {
        pass_hash_match(password, self.password.clone())
    }

    /// This serializer function automaticly encrypts the struct with the *encrypt_aes256* fn to string
    pub fn serialize(&self) -> anyhow::Result<String> {
        Ok(encrypt_aes256(serde_json::to_string(&self)?, &[42; 32]).unwrap())
    }

    /// This deserializer function automaticly decrypts the string the *encrypt_aes256* fn to Self
    pub fn deserialize(serialized_struct: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str::<Self>(
            &decrypt_aes256(serialized_struct, &[42; 32]).unwrap(),
        )?)
    }

    /// Write file to the specified path
    pub fn write_file(&self, user_path: PathBuf) -> anyhow::Result<()> {
        let serialized_self = self.serialize()?;

        let mut file = fs::File::create(user_path)?;

        file.write_all(serialized_self.as_bytes())?;

        file.flush()?;

        Ok(())
    }

    /// Add a bookmark entry which can be converted to a string
    pub fn add_bookmark_entry<T>(&mut self, item: T)
    where
        T: ToString,
    {
        self.bookmarked_ips.push(item.to_string());
    }

    /// Remove bookmark at index from the list, this can panic if the wrong index is passed in
    pub fn delete_bookmark_entry(&mut self, index: usize) {
        self.bookmarked_ips.remove(index);
    }
}

#[inline]
/// aes256 is decrypted by this function by a fixed key
pub fn decrypt_aes256(string_to_be_decrypted: &str, key: &[u8]) -> Result<String, FromUtf8Error> {
    let ciphertext = hex::decode(string_to_be_decrypted).unwrap();

    let key = Key::<Aes256Gcm>::from_slice(key);

    let cipher = Aes256Gcm::new(key);
    let nonce = GenericArray::from([69u8; 12]); // funny encryption key hehehe

    let plaintext = cipher.decrypt(&nonce, ciphertext.as_ref()).unwrap();
    String::from_utf8(plaintext)
}

/// aes256 is encrypted by this function by a fixed key
pub fn encrypt_aes256(string_to_be_encrypted: String, key: &[u8]) -> aes_gcm::aead::Result<String> {
    let key = Key::<Aes256Gcm>::from_slice(key);

    let cipher = Aes256Gcm::new(key);
    let nonce = GenericArray::from([69u8; 12]); // funny encryption key hehehe

    let ciphertext = cipher.encrypt(&nonce, string_to_be_encrypted.as_bytes().as_ref())?;
    let ciphertext = hex::encode(ciphertext);

    Ok(ciphertext)
}

#[inline]
/// Argon is used to encrypt this
pub fn encrypt(string_to_be_encrypted: String) -> String {
    let password = string_to_be_encrypted.as_bytes();
    let salt = b"c1eaa94ec38ab7aa16e9c41d029256d3e423f01defb0a2760b27117ad513ccd2";
    let config = Config {
        variant: Variant::Argon2i,
        version: Version::Version13,
        mem_cost: 65536,
        time_cost: 12,
        lanes: 5,
        secret: &[],
        ad: &[],
        hash_length: 64,
    };

    argon2::hash_encoded(password, salt, &config).unwrap()
}

#[inline]
fn pass_hash_match(to_be_verified: String, encoded: String) -> bool {
    argon2::verify_encoded(&encoded, to_be_verified.as_bytes()).unwrap()
}

///Check login
pub fn login(username: String, password: String) -> Result<PathBuf> {
    let app_data = env::var("APPDATA")?;

    let path = PathBuf::from(format!("{app_data}\\Matthias\\{username}.szch"));

    let file_contents: UserInformation = UserInformation::deserialize(&fs::read_to_string(&path)?)?;

    let user_check = username == file_contents.username;

    ensure!(user_check, "File corrupted at the username entry");

    let password_check = file_contents.verify_password(password);

    ensure!(password_check, "Invalid password");

    Ok(path)
}

///Register a new profile
pub fn register(username: String, passw: String) -> anyhow::Result<()> {
    if username.contains(' ') || username.contains('@') || username.contains(' ') {
        return Err(anyhow::Error::msg("Cant use special characters in name"));
    }

    let app_data = env::var("APPDATA")?;

    let user_path = PathBuf::from(format!("{app_data}\\Matthias\\{username}.szch"));

    //Check if user already exists
    if std::fs::metadata(&user_path).is_ok() {
        return Err(anyhow::Error::msg("User already exists"));
    }

    //Construct user info struct then write it to the appdata matthias folder
    UserInformation::new(
        username,
        passw,
        encrypt_aes256(generate_uuid(), &[42; 32]).unwrap(),
    )
    .write_file(user_path)?;

    Ok(())
}

///Write general file, this function takes in a custom path
pub fn write_file(file_response: ServerFileReply) -> Result<()> {
    let files = FileDialog::new()
        .set_title("Save to")
        .set_directory("/")
        .add_filter(
            file_response
                .file_name
                .extension()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            &[file_response
                .file_name
                .extension()
                .unwrap()
                .to_string_lossy()
                .to_string()],
        )
        .save_file();

    if let Some(file) = files {
        fs::write(file, file_response.bytes)?;
    }

    Ok(())
}

///Write an image file to the appdata folder
#[inline]
pub fn write_image(file_response: &ServerImageReply, ip: String) -> Result<()> {
    //secondly create the folder labeled with the specified server ip

    let path = format!(
        "{}\\matthias\\Client\\{}\\Images\\{}",
        env!("APPDATA"),
        general_purpose::URL_SAFE_NO_PAD.encode(ip),
        file_response.index
    );

    let _ = fs::create_dir(&path).inspect_err(|err| {
        dbg!(err);
    });

    fs::write(path, &file_response.bytes)?;

    Ok(())
}

///Write an audio file to the appdata folder
#[inline]
pub fn write_audio(file_response: ServerAudioReply, ip: String) -> Result<()> {
    //secondly create the folder labeled with the specified server ip
    let path = format!(
        "{}\\matthias\\Client\\{}\\Audios\\{}",
        env!("APPDATA"),
        general_purpose::URL_SAFE_NO_PAD.encode(ip),
        file_response.index
    );

    let _ = fs::create_dir(&path).inspect_err(|err| {
        dbg!(err);
    });

    fs::write(path, file_response.bytes)?;

    Ok(())
}

///Generate uuid
pub fn generate_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

///Display Error message with a messagebox
pub fn display_error_message<T>(display: T)
where
    T: ToString + std::marker::Send + 'static,
{
    std::thread::spawn(move || unsafe {
        MessageBoxW(
            0,
            str::encode_utf16(display.to_string().as_str())
                .chain(std::iter::once(0))
                .collect::<Vec<_>>()
                .as_ptr(),
            w!("Error"),
            MB_ICONERROR,
        );
    });
}

/// This function fetches the incoming full message's lenght (it reads the 4 bytes and creates an u32 number from them, which it returns)
/// afaik this function blocks until it can read the first 4 bytes out of the ```reader```
pub async fn fetch_incoming_message_lenght<T>(reader: &mut T) -> anyhow::Result<u32>
where
    T: AsyncReadExt + Unpin + AsyncRead,
{
    let mut buf: Vec<u8> = vec![0; 4];

    reader.read_exact(&mut buf).await?;

    Ok(u32::from_be_bytes(buf[..4].try_into()?))
}
