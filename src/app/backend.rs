use chrono::{DateTime, Utc};
use egui::Color32;
use rand::rngs::ThreadRng;

use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key,
};
use anyhow::{ensure, Context, Result};
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
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Mutex};
use tonic::transport::{Channel, Endpoint};
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

    ///thread communication for file requesting
    #[serde(skip)]
    pub frx: mpsc::Receiver<String>,
    #[serde(skip)]
    pub ftx: mpsc::Sender<String>,

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
    pub audio_save_rx: mpsc::Receiver<String>,
    #[serde(skip)]
    pub audio_save_tx: mpsc::Sender<String>,

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
    #[serde(skip)]
    pub connection_reciver: mpsc::Receiver<Option<ClientConnection>>,
    #[serde(skip)]
    pub connection_sender: mpsc::Sender<Option<ClientConnection>>,

    ///Server - client syncing thread
    #[serde(skip)]
    pub autosync_sender: Option<mpsc::Receiver<String>>,

    ///Server - client sync worker should run
    #[serde(skip)]
    pub autosync_should_run: Arc<AtomicBool>,

    #[serde(skip)]
    pub audio_file: Arc<Mutex<PathBuf>>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel::<String>();
        let (stx, srx) = mpsc::channel::<String>();
        let (dtx, drx) = mpsc::channel::<String>();
        let (ftx, frx) = mpsc::channel::<String>();
        let (itx, irx) = mpsc::channel::<String>();
        let (audio_save_tx, audio_save_rx) = mpsc::channel::<String>();
        let (connection_sender, connection_reciver) = mpsc::channel::<Option<ClientConnection>>();
        Self {
            audio_file: Arc::new(Mutex::new(PathBuf::from(format!(
                "{}\\Matthias\\Client\\voice_record.wav",
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

            //thread communication for file requesting
            frx,
            ftx,

            //thread communication for image requesting
            irx,
            itx,

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

/*Children structs*/
///Children struct
/// Client Ui
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Client {
    ///Search parameters set by user, to chose what to search for obviously
    pub search_parameters: SearchParameters,

    ///Check if search panel settings panel (xd) is open
    #[serde(skip)]
    pub search_settings_panel: bool,

    ///Search buffer
    #[serde(skip)]
    pub search_buffer: String,

    ///Check if search panel is open
    #[serde(skip)]
    pub search_mode: bool,

    ///Message highlighting function
    #[serde(skip)]
    pub message_highlight_color: Color32,

    ///emoji tray is hovered
    #[serde(skip)]
    pub emoji_tray_is_hovered: bool,

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

    ////This indexes the user's selected messages for replying
    #[serde(skip)]
    pub replying_to: Option<usize>,

    ///Input (Múlt idő) user's message, this is what gets modified in the text editor
    #[serde(skip)]
    pub usr_msg: String,

    ///Incoming messages, this is the whole packet which get sent to all the clients, this cointains all the messages, and the info about them
    #[serde(skip)]
    pub incoming_msg: ServerMaster,

    ///emoji fasz
    pub random_emoji: String,
    pub emoji: Vec<String>,

    ///Random engine
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
            search_parameters: SearchParameters::default(),
            search_settings_panel: false,
            search_buffer: String::new(),
            search_mode: false,

            message_highlight_color: Color32::WHITE,
            //audio playback
            audio_playback: AudioPlayback::default(),
            emoji_tray_is_hovered: false,
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
            replying_to: None,
            incoming_msg: ServerMaster::default(),

            voice_recording_start: None,
        }
    }
}

///Main, Global stuff
#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct Main {
    ///Checks if windwos needs to be set up
    #[serde(skip)]
    pub setup: Option<()>,

    ///Checks if the emoji tray is on
    #[serde(skip)]
    pub emoji_mode: bool,

    ///Checks if bookmark mode is turned on
    #[serde(skip)]
    pub bookmark_mode: bool,

    ///Client mode main switch
    #[serde(skip)]
    pub client_mode: bool,

    ///Server mode main switch
    #[serde(skip)]
    pub server_mode: bool,

    ///Mode selector mode main switch
    #[serde(skip)]
    pub mode_selector: bool,

    ///Opened account's file pathbuf
    #[serde(skip)]
    pub opened_account_path: PathBuf,
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

///Empty packet, as described later, only used for syncing
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientSnycMessage {
    /*Used for syncing or connecting & disconnecting*/
    /// If its None its used for syncing, false: disconnecting, true: connecting
    /// If you have already registered the client with the server then the true value will be ignored
    pub sync_attribute: Option<bool>,
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
}

///This is what gets to be sent out by the client
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientMessage {
    pub replying_to: Option<usize>,
    pub MessageType: ClientMessageType,
    pub Password: String,
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
        password: Option<&str>,
        author: &str,
        replying_to: Option<usize>,
    ) -> ClientMessage {
        ClientMessage {
            replying_to,
            MessageType: ClientMessageType::ClientNormalMessage(ClientNormalMessage {
                message: msg.trim().to_string(),
            }),
            //If the password is set as None (Meaning the user didnt enter any password) just send the message with an empty string
            Password: password.unwrap_or("").to_string(),
            Author: author.to_string(),
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    ///this is used when you want to send a file, this contains name, bytes
    pub fn construct_file_msg(
        file_path: PathBuf,
        password: Option<&str>,
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

            Password: password.unwrap_or("").to_string(),
            Author: author.to_string(),
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    pub fn construct_reaction_msg(
        char: char,
        index: usize,
        author: &str,
        password: Option<&str>,
    ) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            MessageType: ClientMessageType::ClientReaction(ClientReaction {
                char,
                message_index: index,
            }),
            Password: password.unwrap_or("").to_string(),
            Author: author.to_string(),
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    /// this is used for constructing a sync msg aka sending an empty packet, so server can reply
    /// If its None its used for syncing, false: disconnecting, true: connecting
    pub fn construct_sync_msg(password: String, author: String) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            MessageType: ClientMessageType::ClientSyncMessage(ClientSnycMessage {sync_attribute: None}),
            Password: password,
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    /// If its None its used for syncing, false: disconnecting, true: connecting
    pub fn construct_connection_msg(password: String, author: String) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            MessageType: ClientMessageType::ClientSyncMessage(ClientSnycMessage {sync_attribute: Some(true)}),
            Password: password,
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    /// If its None its used for syncing, false: disconnecting, true: connecting
    /// Please note that its doesnt really matter what we pass in the author becuase the server identifies us based on our ip address TODO: Just switch to uuid's
    pub fn construct_disconnection_msg(password: String, author: String) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            MessageType: ClientMessageType::ClientSyncMessage(ClientSnycMessage {sync_attribute: Some(false)}),
            Password: password,
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    ///this is used for asking for a file
    pub fn construct_file_request_msg(
        index: i32,
        password: String,
        author: String,
    ) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            MessageType: ClientMessageType::ClientFileRequestType(
                ClientFileRequestType::ClientFileRequest(ClientFileRequest { index }),
            ),
            Password: password,
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    ///this is used for asking for an image
    pub fn construct_image_request_msg(
        index: i32,
        password: String,
        author: String,
    ) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            MessageType: ClientMessageType::ClientFileRequestType(
                ClientFileRequestType::ClientImageRequest(ClientImageRequest { index }),
            ),
            Password: password,
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    ///this is used for asking for an image
    pub fn construct_audio_request_msg(
        index: i32,
        password: String,
        author: String,
    ) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            MessageType: ClientMessageType::ClientFileRequestType(
                ClientFileRequestType::ClientAudioRequest(ClientAudioRequest { index }),
            ),
            Password: password,
            Author: author,
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
    pub client: Option<MessageClient<Channel>>,
    #[serde(skip)]
    pub state: ConnectionState,
}

impl ClientConnection {
    ///Ip arg to know where to connect, username so we can register with the sever, used to spawn a valid ClientConnection instance
    pub async fn connect(ip: String, author: String, password: String) -> anyhow::Result<Self> {
        Ok(Self {
            client: {
                //Ping server to recive custom uuid, and to also get if server ip is valid
                let client = MessageClient::new(Endpoint::from_shared(ip.clone())?.connect_lazy());

                let mut client_clone = client.clone();

                match client_clone
                    .message_main(tonic::Request::new(MessageRequest {
                        message: ClientMessage::construct_connection_msg(password, author)
                        .struct_into_string(),
                    }))
                    .await
                {
                    /*We could return this, this is what the server is supposed to return, when a new user is connected */
                    Ok(server_reply) => {
                        Some(client_clone)
                    },
                    Err(error) => {
                        std::thread::spawn(move || unsafe {
                            MessageBoxW(
                                0,
                                str::encode_utf16(error.to_string().as_str())
                                    .chain(std::iter::once(0))
                                    .collect::<Vec<_>>()
                                    .as_ptr(),
                                w!("Error"),
                                MB_ICONERROR,
                            );
                        });
                        None
                    }
                }
            },
            state: ConnectionState::Connected,
        })
    }

    ///Used to destroy a current ClientConnection instance does not matter if the instance is invalid
    pub async fn disconnect(&mut self, author: String, password: String) -> anyhow::Result<()> {
        
        //De-register with the server
        let client = self.client.as_mut().ok_or(anyhow::Error::msg("Invalid ClientConnection instance (Client is None)"))?;

        client.message_main(
            tonic::Request::new(MessageRequest {
                message: ClientMessage::construct_disconnection_msg(password, author)
                .struct_into_string(),
            })
        ).await?;

        Ok(())
    }
}

///Used to show state of the connection
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Connecting,
    Error,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self::Disconnected
    }
}

impl Debug for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ConnectionState::Connected => "Connected",
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
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerFileUpload {
    pub file_name: String,
    pub index: i32,
}

///This is what the server sends back, when asked for a file (FIleRequest)
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerFileReply {
    pub bytes: Vec<u8>,
    pub file_name: PathBuf,
}

///This is what gets sent to a client basicly, and they have to ask for the file when the ui containin this gets rendered
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerImageUpload {
    pub index: i32,
}

///When client asks for the image based on the provided index, reply with the image bytes
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerImageReply {
    pub bytes: Vec<u8>,
    pub index: i32,
}

///This is what the server sends back (pushes to message vector), when reciving a normal message
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerNormalMessage {
    pub message: String,
}

///REFER TO -> ServerImageUpload; logic      ||      same thing but with audio files
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerAudioUpload {
    pub index: i32,
    pub file_name: String,
}

///When client asks for the image based on the provided index, reply with the audio bytes, which gets written so it can be opened by a readbuf
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerAudioReply {
    pub bytes: Vec<u8>,
    pub index: i32,
    pub file_name: String,
}

use strum::{EnumDiscriminants, EnumMessage};
use strum_macros::EnumString;

use super::client::messages::message_client::MessageClient;
use super::client::messages::MessageRequest;

///This is what server replies can be
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, EnumDiscriminants)]
#[strum_discriminants(derive(EnumString, EnumMessage))]
pub enum ServerMessageType {
    #[strum_discriminants(strum(message = "Upload"))]
    Upload(ServerFileUpload),
    #[strum_discriminants(strum(message = "Normal"))]
    Normal(ServerNormalMessage),

    ///Used to send and index to client so it knows which index to ask for VERY IMPORTANT!!!!!!!!!
    #[strum_discriminants(strum(message = "Image"))]
    Image(ServerImageUpload),
    #[strum_discriminants(strum(message = "Audio"))]
    Audio(ServerAudioUpload),
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

///This is one whole server msg (packet), which gets bundled when sending ServerMain
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerOutput {
    pub replying_to: Option<usize>,
    pub MessageType: ServerMessageType,
    pub Author: String,
    pub MessageDate: String,
    pub reactions: MessageReaction,
}
impl ServerOutput {
    pub fn _struct_into_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn convert_type_to_servermsg(
        normal_msg: ClientMessage,
        index: i32,
        //Automaticly generated enum by strum
        upload_type: ServerMessageTypeDiscriminants,
        reactions: MessageReaction,
    ) -> ServerOutput {
        ServerOutput {
            replying_to: normal_msg.replying_to,
            MessageType:
                match normal_msg.MessageType {
                    ClientMessageType::ClientFileRequestType(_) => unimplemented!("Converting Sync packets isnt implemented, because they shouldnt be displayed to the client"),
                    ClientMessageType::ClientFileUpload(upload) => {
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
                            ServerMessageTypeDiscriminants::Normal => unreachable!(),
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
                        }
                    },
                    ClientMessageType::ClientNormalMessage(message) => {
                        ServerMessageType::Normal(
                            ServerNormalMessage {
                                message: message.message,
                            }
                        )
                    },
                    ClientMessageType::ClientSyncMessage(_) => unimplemented!("Converting Sync packets isnt implemented, because they shouldnt be displayed to the client"),
                    ClientMessageType::ClientReaction(_) => todo!(),
                },
            Author: normal_msg.Author,
            MessageDate: normal_msg.MessageDate,
            reactions,
        }
    }
}

///Used to put all the messages into 1 big pack (Bundling All the ServerOutput-s), Main packet, this gets to all the clients
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct ServerMaster {
    pub struct_list: Vec<ServerOutput>,
}
impl ServerMaster {
    pub fn struct_into_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
    pub fn convert_vec_serverout_into_server_master(
        server_output_list: Vec<ServerOutput>,
    ) -> ServerMaster {
        ServerMaster {
            struct_list: server_output_list,
        }
    }
}

/*
 Client backend
*/

///Struct for audio playback
pub struct AudioPlayback {
    pub stream: OutputStream,
    pub stream_handle: OutputStreamHandle,
    pub sink_list: Vec<Option<Sink>>,
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
    pub volume: f32,
    pub speed: f32,
    pub cursor: PlaybackCursor,
    pub cursor_offset: u64,
}

///Initialize default values
impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            volume: 0.8,
            speed: 1.,
            cursor: PlaybackCursor::new([0].to_vec()),
            cursor_offset: 0,
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

///Impl new so It can probe a file (in vec<u8> format)
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
        cursor.read(buf)
    }
}

///Implement the Seek trait
impl Seek for PlaybackCursor {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let mut cursor = self.cursor.lock().unwrap();
        cursor.seek(pos)
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
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum SearchType {
    Date,
    File,
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

///Main searchparameter struct contains everyting the client needs for searching
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct SearchParameters {
    pub search_type: SearchType,
}

impl Default for SearchParameters {
    fn default() -> Self {
        Self {
            search_type: SearchType::Message,
        }
    }
}

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

//Account management
pub fn encrypt_aes256(string_to_be_encrypted: String) -> aes_gcm::aead::Result<String> {
    let key: &[u8] = &[42; 32];

    let key = Key::<Aes256Gcm>::from_slice(key);

    let cipher = Aes256Gcm::new(key);
    let nonce = GenericArray::from([69u8; 12]); // funny encryption key hehehe

    let ciphertext = cipher.encrypt(&nonce, string_to_be_encrypted.as_bytes().as_ref())?;
    let ciphertext = hex::encode(ciphertext);

    Ok(ciphertext)
}

#[inline]
pub fn decrypt_aes256(string_to_be_decrypted: String) -> Result<String, FromUtf8Error> {
    let ciphertext = hex::decode(string_to_be_decrypted).unwrap();
    let key: &[u8] = &[42; 32];

    let key = Key::<Aes256Gcm>::from_slice(key);

    let cipher = Aes256Gcm::new(key);
    let nonce = GenericArray::from([69u8; 12]); // funny encryption key hehehe

    let plaintext = cipher.decrypt(&nonce, ciphertext.as_ref()).unwrap();
    String::from_utf8(plaintext)
}

#[inline]
pub fn pass_encrypt(string_to_be_encrypted: String) -> String {
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
pub fn pass_hash_match(to_be_verified: String, file_ln: String) -> bool {
    argon2::verify_encoded(&file_ln, to_be_verified.as_bytes()).unwrap()
}

pub fn login(username: String, passw: String) -> Result<PathBuf> {
    let app_data = env::var("APPDATA")?;

    let path = PathBuf::from(format!("{app_data}\\Matthias\\{username}.szch"));

    let file_contents = fs::read_to_string(&path)?;

    let mut file_lines = file_contents.lines();

    let usr_check = username
        == file_lines
            .next()
            .context("Corrupted Matthias file at username")?;

    let pwd_check = pass_hash_match(
        passw,
        file_lines
            .next()
            .context("Corrupted Matthias file at password")?
            .into(),
    );

    ensure!(usr_check && pwd_check, "Invalid Password");
    Ok(path)
}

pub fn register(username: String, passw: String) -> Result<()> {
    if username.contains(' ') || username.contains('@') || username.contains(' ') {
        return Err(anyhow::Error::msg("Cant use special characters in name"));
    }

    let app_data = env::var("APPDATA")?;

    //always atleast try to make the folder
    let _ = fs::create_dir_all(format!("{app_data}\\Matthias"));

    let user_path = PathBuf::from(format!("{app_data}\\Matthias\\{username}.szch"));

    //user check
    if std::fs::metadata(&user_path).is_ok() {
        return Err(anyhow::Error::msg("User already exists"));
    }

    let hex_encrypted = username + "\n" + &pass_encrypt(passw);

    let mut file = fs::File::create(&user_path)?;

    file.write_all(hex_encrypted.as_bytes())?;

    file.flush()?;

    Ok(())
}

pub fn append_to_file(path: PathBuf, write: String) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(false)
        .append(true)
        .open(path)?;

    match encrypt_aes256(write) {
        Ok(write) => {
            file.write_all((format!("\n{write}")).as_bytes())?;

            file.flush()?;

            Ok(())
        }
        Err(_) => Err(anyhow::Error::msg(
            "Error occured when trying to encrypt with sha256",
        )),
    }
}

pub fn decrypt_lines_from_vec(mut file_ln: Vec<String>) -> Result<Vec<String>> {
    //remove pass and user
    file_ln.remove(0);
    file_ln.remove(0);

    let mut output_vec: Vec<String> = Vec::new();

    if !file_ln.is_empty() {
        for item in file_ln {
            //skip invalid lines
            if !item.trim().is_empty() {
                let decrypted_line = decrypt_aes256(item.to_string())?;

                output_vec.push(decrypted_line)
            }
        }
    }

    Ok(output_vec)
}

pub fn read_from_file(path: PathBuf) -> Result<Vec<String>> {
    let file: Vec<String> = fs::read_to_string(path)?
        .lines()
        .map(|f| f.to_string())
        .collect();

    Ok(file)
}

pub fn delete_line_from_file(line_number: usize, path: PathBuf) -> Result<()> {
    //copy everything from orignal convert to vec representing lines delete line rewrite file
    let mut file = std::fs::OpenOptions::new().read(true).open(path.clone())?;

    let mut buf: String = Default::default();
    file.read_to_string(&mut buf)?;

    let final_vec: Vec<&str> = buf.lines().collect();

    let mut final_vec: Vec<String> = final_vec.iter().map(|s| s.to_string()).collect();
    final_vec.remove(line_number);

    file.flush()?;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)?;

    let buf = final_vec.join("\n");

    file.write_all(buf.as_bytes())?;

    file.flush()?;

    Ok(())
}

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

#[inline]
pub fn write_image(file_response: &ServerImageReply, ip: String) -> Result<()> {
    //secondly create the folder labeled with the specified server ip

    let path = format!(
        "{}\\Matthias\\Client\\{}\\Images\\{}",
        env!("APPDATA"),
        general_purpose::URL_SAFE_NO_PAD.encode(ip),
        file_response.index
    );

    fs::write(path, &file_response.bytes)?;

    Ok(())
}

#[inline]
pub fn write_audio(file_response: ServerAudioReply, ip: String) -> Result<()> {
    //secondly create the folder labeled with the specified server ip
    let path = format!(
        "{}\\Matthias\\Client\\{}\\Audios\\{}",
        env!("APPDATA"),
        general_purpose::URL_SAFE_NO_PAD.encode(ip),
        file_response.index
    );

    fs::write(path, file_response.bytes)?;

    Ok(())
}

pub fn generate_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}
