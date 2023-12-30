use chrono::Utc;
use rand::rngs::ThreadRng;

use rodio::{OutputStream, OutputStreamHandle, Sink};
use std::collections::BTreeMap;

use std::io;
use std::io::{Read, Seek, SeekFrom};

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Mutex};

use crate::app::input::Input;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    /*
        Font
    */
    //fontbook
    pub filter: String,
    pub named_chars: BTreeMap<egui::FontFamily, BTreeMap<char, String>>,
    //font
    pub font_size: f32,

    /*
    login page
    */
    //the string entered to the username field on the login page
    pub login_username: String,

    #[serde(skip)]
    //the string entered to the password field on the login page, dont save this one... obviously :)
    pub login_password: String,

    /*
        server main
    */
    //DEPRICATED, ipv4 mode
    pub ipv4_mode: bool,

    //SChecks whether server is already started TODO: FIX DUMB STUFF LIKE THIS, INSTEAD USE AN OPTION
    #[serde(skip)]
    pub server_has_started: bool,

    //Public ip address, checked by pinging external website
    #[serde(skip)]
    pub public_ip: String,

    //server settings
    pub server_req_password: bool,

    //What is the server's password set to
    pub server_password: String,

    //Which port is the server open on
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

    //thread communication for audio recording
    #[serde(skip)]
    pub atx: Option<mpsc::Sender<bool>>,

    //thread communication for audio ! SAVING !
    #[serde(skip)]
    pub audio_save_rx: mpsc::Receiver<String>,
    #[serde(skip)]
    pub audio_save_tx: mpsc::Sender<String>,

    /*
        main
    */
    //Checks if the emoji tray is on
    #[serde(skip)]
    pub emoji_mode: bool,

    //This struct is used for keymapping, TODO: just switch to ctx.input
    #[serde(skip)]
    pub keymap: Input,

    //Checks if bookmark mode is turned on
    #[serde(skip)]
    pub bookmark_mode: bool,

    //Client mode main switch
    #[serde(skip)]
    pub client_mode: bool,

    //Server mode main switch
    #[serde(skip)]
    pub server_mode: bool,

    //Mode selector mode main switch
    #[serde(skip)]
    pub mode_selector: bool,

    //Opened account's file pathbuf
    #[serde(skip)]
    pub opened_account_path: PathBuf,

    /*
        client main
    */
    //audio playback
    #[serde(skip)]
    pub audio_playback: AudioPlayback,

    //this doesnt really matter if we save or no so whatever
    #[serde(skip)]
    pub should_scroll_to_reply: bool,

    //Selected port on sending
    pub send_on_port: String,

    //Selected ip address (without port as seen above)
    pub send_on_address: String,

    //This is used when the client entered a false password to connect with to the server
    #[serde(skip)]
    pub invalid_password: bool,

    //This is set to on when an image is enlarged
    #[serde(skip)]
    pub image_overlay: bool,

    //Scroll widget rect, text editor's rect
    pub scroll_widget_rect: egui::Rect,

    //This decides how wide the text editor should be, ensure it doesnt overlap with "msg_action_tray" (the action buttons :) )
    pub text_widget_offset: f32,

    //A vector of all the added files to the buffer, these are the PathBufs which get read, then their bytes get sent
    #[serde(skip)]
    pub files_to_send: Vec<PathBuf>,

    //This checks if the text editor is open or not
    pub usr_msg_expanded: bool,

    //This is the full address of the destionation a message is supposed to be sent to
    pub send_on_ip: String,

    pub send_on_ip_base64_encoded: String,
    pub req_passw: bool,
    pub client_password: String,

    //This gem of a variable is used to contain animation's state
    pub how_on: f32,

    //This checks if a file is dragged above szeChat, so it knows when to display the cool animation 8)
    #[serde(skip)]
    pub drop_file_animation: bool,

    //Message Settings
    #[serde(skip)]
    pub replying_to: Option<usize>,

    //Input (Múlt idő) user's message, this is what gets modified in the text editor
    #[serde(skip)]
    pub usr_msg: String,

    //Incoming messages, this is the whole packet which get sent to all the clients, this cointains all the messages, and the info about them
    #[serde(skip)]
    pub incoming_msg: ServerMaster,

    //emoji fasz
    pub random_emoji: String,
    pub emoji: Vec<String>,

    //Random engine
    #[serde(skip)]
    pub rand_eng: ThreadRng,

    //Used to decide whether the reactive emoji button should switch emojis (Like discords implementation)
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

    //Server - client syncing thread
    #[serde(skip)]
    pub autosync_sender: Option<mpsc::Receiver<String>>,

    //Server - client sync worker should run
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
        let (audio_save_tx, audio_save_rx) = mpsc::channel::<String>();
        Self {
            //audio playback
            audio_playback: AudioPlayback::default(),

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

            //thread communication for audio recording
            atx: None,

            //thread communication for audio saving
            audio_save_rx,
            audio_save_tx,

            //main
            scroll_widget_rect: egui::Rect::NAN,
            text_widget_offset: 0.0,
            emoji_mode: false,
            keymap: Input::default(),
            bookmark_mode: false,
            client_mode: false,
            server_mode: false,
            mode_selector: false,
            opened_account_path: PathBuf::default(),

            //client main
            should_scroll_to_reply: false,
            send_on_port: String::new(),
            send_on_address: String::new(),
            invalid_password: false,
            image_overlay: false,
            files_to_send: Vec::new(),
            how_on: 0.0,
            drop_file_animation: false,
            usr_msg_expanded: false,
            send_on_ip: String::new(),
            send_on_ip_base64_encoded: String::new(),
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
            incoming_msg: ServerMaster::default(),

            //thread communication for client
            rx,
            tx,

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

//When the client is uploading a file, this packet gets sent
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientFileUpload {
    pub extension: Option<String>,
    pub name: Option<String>,
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

//Client requests audio file in server
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientAudioRequest {
    pub index: i32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum ClientFileRequestType {
    //this is when you want to display an image and you have to make a request to the server file
    ClientImageRequest(ClientImageRequest),
    ClientFileRequest(ClientFileRequest),
    ClientAudioRequest(ClientAudioRequest),
}

//Client outgoing message types
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum ClientMessageType {
    ClientFileRequestType(ClientFileRequestType),

    ClientFileUpload(ClientFileUpload),

    //Normal msg
    ClientNormalMessage(ClientNormalMessage),

    //Used for syncing with client and server
    ClientSyncMessage(ClientSnycMessage),
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
        serde_json::to_string(self).unwrap_or_default()
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
            replying_to,
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
            replying_to,
            //Dont execute me please :3 |
            //                          |
            //                          V
            MessageType: ClientMessageType::ClientFileUpload(ClientFileUpload {
                extension: Some(file_name.extension().unwrap().to_str().unwrap().to_string()),
                name: Some(
                    file_name
                        .file_prefix()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string(),
                ),
                bytes: std::fs::read(file_name).unwrap_or_default(),
            }),

            Password: password,
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
            Destination: ip,
        }
    }

    //this is used for constructing a sync msg aka sending an empty packet, so server can reply
    pub fn construct_sync_msg(ip: String, password: String, author: String) -> ClientMessage {
        ClientMessage {
            replying_to: None,
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
    ) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            MessageType: ClientMessageType::ClientFileRequestType(
                ClientFileRequestType::ClientFileRequest(ClientFileRequest { index }),
            ),
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
    ) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            MessageType: ClientMessageType::ClientFileRequestType(
                ClientFileRequestType::ClientImageRequest(ClientImageRequest { index }),
            ),
            Password: password,
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
            Destination: ip,
        }
    }

    //this is used for asking for an image
    pub fn construct_audio_request_msg(
        index: i32,
        password: String,
        author: String,
        ip: String,
    ) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            MessageType: ClientMessageType::ClientFileRequestType(
                ClientFileRequestType::ClientAudioRequest(ClientAudioRequest { index }),
            ),
            Password: password,
            Author: author,
            MessageDate: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
            Destination: ip,
        }
    }

    //this is used for SENDING IMAGES SO THE SERVER CAN DECIDE IF ITS A PICTURE
    //NOTICE: ALL THE AUDIO UPLOAD TYPES HAVE BEEN CONVERTED INTO ONE => "ClientFileUpload" this ensures that the client doesnt handle any backend stuff
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

//REFER TO -> ServerImageUpload; logic      ||      same thing but with audio files
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerAudioUpload {
    pub index: i32,
    pub file_name: String,
}

//When client asks for the image based on the provided index, reply with the audio bytes, which gets written so it can be opened by a readbuf
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerAudioReply {
    pub bytes: Vec<u8>,
    pub index: i32,
    pub file_name: String,
}

//This is what server replies can be
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum ServerMessageType {
    Upload(ServerFileUpload),
    Normal(ServerNormalMessage),

    //Used to send and index to client so it knows which index to ask for VERY IMPORTANT!!!!!!!!!
    Image(ServerImageUpload),
    Audio(ServerAudioUpload),
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
        serde_json::to_string(self).unwrap_or_default()
    }
    pub fn convert_audio_to_servermsg(normal_msg: ClientMessage, index: i32) -> ServerOutput {
        ServerOutput {
            replying_to: normal_msg.replying_to,
            MessageType: ServerMessageType::Audio(ServerAudioUpload {
                file_name: match normal_msg.MessageType {
                    ClientMessageType::ClientSyncMessage(_) => todo!(),
                    ClientMessageType::ClientFileUpload(upload) => upload.name.unwrap_or_default(),
                    ClientMessageType::ClientNormalMessage(_) => todo!(),
                    ClientMessageType::ClientFileRequestType(_) => todo!(),
                },
                index,
            }),
            Author: normal_msg.Author,
            MessageDate: normal_msg.MessageDate,
        }
    }

    pub fn convert_msg_to_servermsg(normal_msg: ClientMessage) -> ServerOutput {
        //Convert a client output to a server output (ClientMessage -> ServerOutput), trim some useless info
        ServerOutput {
            replying_to: normal_msg.replying_to,
            MessageType: ServerMessageType::Normal(ServerNormalMessage {
                message: match normal_msg.MessageType {
                    ClientMessageType::ClientSyncMessage(_) => todo!(),
                    ClientMessageType::ClientFileUpload(_) => todo!(),
                    ClientMessageType::ClientNormalMessage(msg) => msg.message,
                    ClientMessageType::ClientFileRequestType(_) => todo!(),
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
                    ClientMessageType::ClientFileUpload(_upload) => index,
                    ClientMessageType::ClientNormalMessage(_) => todo!(),
                    ClientMessageType::ClientFileRequestType(_) => todo!(),
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
                    ClientMessageType::ClientFileUpload(msg) => {
                        format!(
                            "{}.{}",
                            msg.name.unwrap_or_default(),
                            msg.extension.unwrap_or_default()
                        )
                    }
                    ClientMessageType::ClientNormalMessage(_) => todo!(),
                    ClientMessageType::ClientFileRequestType(_) => todo!(),
                },
                index,
            }),
            Author: normal_msg.Author,
            MessageDate: normal_msg.MessageDate,
        }
    }
}

//Used to put all the messages into 1 big pack (Bundling All the ServerOutput-s), Main packet, this gets to all the clients
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

//Struct for audio playback
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

//This is used by the audio player, this is where you can set the speed and volume etc
pub struct AudioSettings {
    pub volume: f32,
    pub speed: f32,
    pub cursor: PlaybackCursor,
    pub cursor_offset: u64,
}

//Initialize default values
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

//Impl new so It can probe a file (in vec<u8> format)
impl PlaybackCursor {
    pub fn new(data: Vec<u8>) -> Self {
        let cursor = Arc::new(Mutex::new(io::Cursor::new(data)));
        PlaybackCursor { cursor }
    }
}

//Implement the Read trait
impl Read for PlaybackCursor {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut cursor = self.cursor.lock().unwrap();
        cursor.read(buf)
    }
}

//Implement the Seek trait
impl Seek for PlaybackCursor {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let mut cursor = self.cursor.lock().unwrap();
        cursor.seek(pos)
    }
}
