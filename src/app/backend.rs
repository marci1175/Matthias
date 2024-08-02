use super::client::{connect_to_server, ServerReply};
use super::lua::{Extension, LuaOutput};
use super::read_extensions_dir;
use super::server::SharedFields;
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key,
};
use anyhow::{bail, ensure, Error, Result};
use argon2::Config;
use base64::engine::general_purpose;
use base64::Engine;
use chrono::{DateTime, NaiveDate, Utc};
use dashmap::DashMap;
use egui::load::BytesPoll;
use egui::load::LoadError;
use egui::{
    vec2, Align2, Color32, FontId, Image, Pos2, Rect, Response, RichText, Stroke, Ui, Vec2,
};
use egui_notify::{Toast, Toasts};
use image::DynamicImage;
use mlua::Lua;
use mlua_proc_macro::ToTable;
use rand::rngs::ThreadRng;
use regex::Regex;
use rfd::FileDialog;
use rodio::{OutputStream, OutputStreamHandle, Sink};
use std::collections::HashMap;
use std::env;
use std::fmt::{Debug, Display};
use std::fs;
use std::io;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use strum::{EnumDiscriminants, EnumMessage};
use strum_macros::EnumString;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(serde::Deserialize, serde::Serialize, ToTable, Clone)]
#[serde(default)]
pub struct Application {
    /// This is field is used to display notifications
    #[serde(skip)]
    pub toasts: Arc<Mutex<Toasts>>,

    #[serde(skip)]
    pub lua: Arc<Lua>,

    /*
        Font
    */
    ///fontbook
    pub filter: String,

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
    ///Checks whether server is already started TODO: FIX DUMB STUFF LIKE THIS, INSTEAD USE AN OPTION
    #[serde(skip)]
    pub server_has_started: bool,

    #[table(save)]
    #[serde(skip)]
    /// This is used to store the connected client profile
    /// The field get modified by the server_main function (When a server is started this is passed in and is later modified by the server)
    /// This might seem very similar to ```self.client_ui.incoming_msg.connected_clients_profile```, but that field is only modifed when connecting to a server, so when we start a server but dont connect to it, we wont have the fields
    /// This field gets directly modified by the server thread
    pub server_connected_clients_profile: Arc<DashMap<String, ClientProfile>>,

    ///Public ip address, checked by pinging external website
    #[serde(skip)]
    pub public_ip: String,

    ///server settings
    pub server_req_password: bool,

    ///Server shutdown handler channel
    #[serde(skip)]
    pub server_shutdown_token: CancellationToken,

    ///Voip thread shutdown token
    #[serde(skip)]
    pub voip_shutdown_token: CancellationToken,

    ///What is the server's password set to
    pub server_password: String,

    ///Which port is the server open on
    pub open_on_port: String,

    ///child windows
    #[serde(skip)]
    pub settings_window: bool,

    ///thread communication for audio recording
    #[serde(skip)]
    pub atx: Option<mpsc::Sender<bool>>,

    ///thread communication for audio ! SAVING !
    #[serde(skip)]
    pub audio_save_rx: Arc<mpsc::Receiver<(Option<Arc<Sink>>, PlaybackCursor, usize, PathBuf)>>,
    #[serde(skip)]
    pub audio_save_tx: Arc<mpsc::Sender<(Option<Arc<Sink>>, PlaybackCursor, usize, PathBuf)>>,

    ///Channels for sending recorded, and formatted Wav audio bytes
    #[serde(skip)]
    pub audio_bytes_tx: Arc<mpsc::Sender<Vec<u8>>>,
    #[serde(skip)]
    pub audio_bytes_rx: Arc<mpsc::Receiver<Vec<u8>>>,

    /// This is used as an interrupt to the voice recording function
    #[serde(skip)]
    pub record_audio_interrupter: mpsc::Sender<()>,

    /*
        Register
    */
    #[serde(skip)]
    pub register: Register,

    /*
        Main
    */
    pub main: Main,

    /*
        Client main
    */
    //We can skip this entry of the table since, we implement Totable separetly
    #[table(skip)]
    pub client_ui: Client,

    #[table(save)]
    #[serde(skip)]
    pub client_connection: ClientConnection,

    ///data sync
    #[serde(skip)]
    pub drx: Arc<mpsc::Receiver<String>>,
    #[serde(skip)]
    pub dtx: Arc<mpsc::Sender<String>>,

    /// Server connection
    /// This channel hosts a Client connection and the sync message sent by the server in a String format
    #[serde(skip)]
    pub connection_reciver: Arc<mpsc::Receiver<Option<(ClientConnection, String)>>>,
    #[serde(skip)]
    pub connection_sender: mpsc::Sender<Option<(ClientConnection, String)>>,

    /// Voip (UdpSocket) maker
    /// When a successful ```Voip``` instance is created it is sent over from the async thread
    #[serde(skip)]
    pub voip_connection_reciver: Arc<mpsc::Receiver<Voip>>,
    #[serde(skip)]
    pub voip_connection_sender: mpsc::Sender<Voip>,

    ///Server - client syncing thread
    #[serde(skip)]
    pub server_sender_thread: Option<()>,

    /// Voip audio sender thread
    #[serde(skip)]
    pub voip_thread: Option<()>,

    #[serde(skip)]
    /// This is what the main thread uses to recive messages from the sync thread
    pub server_output_reciver: Arc<Receiver<Option<String>>>,

    #[serde(skip)]
    /// This is what the sync thread uses to send messages to the main thread
    pub server_output_sender: Sender<Option<String>>,

    #[serde(skip)]
    /// This is what the main thread uses to send the shutdown message to the sync thread
    pub autosync_shutdown_token: CancellationToken,

    #[serde(skip)]
    pub audio_file: Arc<Mutex<PathBuf>>,

    #[serde(skip)]
    pub opened_user_information: UserInformation,
}

impl Default for Application {
    fn default() -> Self {
        let (dtx, drx) = mpsc::channel::<String>();
        let (audio_save_tx, audio_save_rx) =
            mpsc::channel::<(Option<Arc<Sink>>, PlaybackCursor, usize, PathBuf)>();

        let (audio_bytes_tx, audio_bytes_rx) = mpsc::channel::<Vec<u8>>();

        let (connection_sender, connection_reciver) =
            mpsc::channel::<Option<(ClientConnection, String)>>();

        let (server_output_sender, server_output_reciver) = mpsc::channel::<Option<String>>();

        let (voip_connection_sender, voip_connection_reciver) = mpsc::channel::<Voip>();

        Self {
            record_audio_interrupter: mpsc::channel::<()>().0,
            toasts: Arc::new(Mutex::new(Toasts::new())),

            voip_shutdown_token: CancellationToken::new(),
            voip_thread: None,

            //Make it so we can import any kind of library
            lua: unsafe { Arc::new(Lua::unsafe_new()) },

            register: Register::default(),

            audio_file: Arc::new(Mutex::new(PathBuf::from(format!(
                "{}\\Matthias\\Client\\voice_recording.wav",
                env!("APPDATA")
            )))),

            //fontbook
            filter: Default::default(),

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

            //child windows
            settings_window: false,

            //This default value will get overwritten when crating the new server, so we can pass in the token to the thread
            //Also, the shutdown reciver is unnecessary in this context because we never use it, I too lazy to delete a few lines instead of writing this whole paragraph >:D
            server_shutdown_token: CancellationToken::new(),

            //thread communication for audio recording
            atx: None,

            //thread communication for audio saving
            audio_save_rx: Arc::new(audio_save_rx),
            audio_save_tx: Arc::new(audio_save_tx),

            //Channels for sending recorded, and formatted Wav audio bytes
            audio_bytes_rx: Arc::new(audio_bytes_rx),
            audio_bytes_tx: Arc::new(audio_bytes_tx),

            //main
            main: Main::default(),

            //client main
            client_ui: Client::default(),

            client_connection: ClientConnection::default(),

            //font
            font_size: 20.,

            //emoji button

            //Server connection
            connection_sender,
            connection_reciver: Arc::new(connection_reciver),

            //data sync
            drx: Arc::new(drx),
            dtx: Arc::new(dtx),
            server_sender_thread: None,

            server_output_reciver: Arc::new(server_output_reciver),
            server_output_sender,

            voip_connection_reciver: Arc::new(voip_connection_reciver),
            voip_connection_sender,

            autosync_shutdown_token: CancellationToken::new(),
            server_connected_clients_profile: Arc::new(DashMap::new()),
            opened_user_information: UserInformation::default(),
        }
    }
}

impl Application {
    /// Set global lua table so that the luas can use it
    /// This function is called when creating a new ```Application``` instance
    pub fn set_global_lua_table(&self) {
        self.client_ui.clone().set_lua_table_function(&self.lua);
        self.clone().set_lua_table_function(&self.lua);
        self.client_connection
            .clone()
            .set_lua_table_function(&self.lua);
        self.opened_user_information
            .clone()
            .set_lua_table_function(&self.lua);
    }

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            let mut data: Application =
                eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();

            //Read extension dir every startup
            match read_extensions_dir() {
                Ok(extension_list) => {
                    data.client_ui.extension.extension_list = extension_list;
                }
                //If there was an error, print it out and create the extensions folder as this is the most likely thing to error
                Err(err) => {
                    dbg!(err);
                    let _ = fs::create_dir(format!("{}\\matthias\\extensions", env!("APPDATA")));
                }
            }

            let output_list = data.client_ui.extension.output.clone();

            return set_lua_functions(data, output_list, cc);
        }

        Default::default()
    }
}

fn set_lua_functions(
    data: Application,
    output_list: Arc<Mutex<Vec<LuaOutput>>>,
    cc: &eframe::CreationContext,
) -> Application {
    let print = data
        .lua
        .create_function(move |_, msg: String| {
            match output_list.lock() {
                Ok(mut list) => list.push(LuaOutput::Standard(msg)),
                Err(err) => {
                    dbg!(err);
                }
            }

            Ok(())
        })
        .unwrap();

    let ctx_clone_rect = cc.egui_ctx.clone();
    let ctx_clone_circle = cc.egui_ctx.clone();
    let ctx_clone_line = cc.egui_ctx.clone();
    let ctx_clone_text = cc.egui_ctx.clone();
    let ctx_clone_image = cc.egui_ctx.clone();
    let ctx_clone_image_buffer_clean = cc.egui_ctx.clone();

    let toasts = data.toasts.clone();
    let toasts_clone1 = data.toasts.clone();
    let toasts_clone2 = data.toasts.clone();

    let draw_line = data
        .lua
        .create_function(move |_, args: ([f32; 2], [f32; 2], [u8; 4])| {
            let color = args.2;

            //Create area
            egui::Area::new("draw_line".into()).show(&ctx_clone_line, |ui| {
                //Draw line based on args
                ui.painter().line_segment(
                    [
                        Pos2::new(args.0[0], args.0[1]),
                        Pos2::new(args.1[0], args.1[1]),
                    ],
                    Stroke::new(
                        5.,
                        Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3]),
                    ),
                );
            });

            Ok(())
        })
        .unwrap();

    let draw_rect =
        data.lua
            .create_function(move |_, args: ([f32; 2], [f32; 2], bool, [u8; 4])| {
                let (start_pos, end_pos, is_filled, color) = args;

                //Create area
                egui::Area::new("draw_rect_filled".into()).show(&ctx_clone_rect, |ui| {
                    match is_filled {
                        true => {
                            ui.painter().rect_filled(
                                Rect::from_points(&[start_pos.into(), end_pos.into()]),
                                0.,
                                Color32::from_rgba_premultiplied(
                                    color[0], color[1], color[2], color[3],
                                ),
                            );
                        }
                        false => {
                            ui.painter().rect_stroke(
                                Rect::from_points(&[start_pos.into(), end_pos.into()]),
                                0.,
                                Stroke::new(
                                    5.,
                                    Color32::from_rgba_premultiplied(
                                        color[0], color[1], color[2], color[3],
                                    ),
                                ),
                            );
                        }
                    }
                });

                Ok(())
            })
            .unwrap();

    let draw_circle = data
        .lua
        .create_function(move |_, args: ([f32; 2], f32, bool, [u8; 4])| {
            let (position, radius, is_filled, color) = args;

            //Create area
            egui::Area::new("draw_circle".into()).show(&ctx_clone_circle, |ui| {
                let painter = ui.painter();

                //Is the circle filled
                match is_filled {
                    true => {
                        painter.circle_filled(
                            position.into(),
                            radius,
                            Color32::from_rgba_premultiplied(
                                color[0], color[1], color[2], color[3],
                            ),
                        );
                    }
                    false => {
                        painter.circle_stroke(
                            position.into(),
                            radius,
                            Stroke::new(
                                5.,
                                Color32::from_rgba_premultiplied(
                                    color[0], color[1], color[2], color[3],
                                ),
                            ),
                        );
                    }
                }
            });

            Ok(())
        })
        .unwrap();

    let draw_text = data
        .lua
        .create_function(move |_, args: ([f32; 2], f32, String, [u8; 4])| {
            let (pos, size, text, color) = args;

            egui::Area::new("draw_text".into()).show(&ctx_clone_text, |ui| {
                ui.painter().text(
                    pos.into(),
                    Align2::LEFT_TOP,
                    text,
                    FontId::new(size, egui::FontFamily::Monospace),
                    Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3]),
                )
            });

            Ok(())
        })
        .unwrap();

    let draw_image = data
        .lua
        .create_function(move |_, args: ([f32; 2], [f32; 2], String)| {
            let (pos, size, path) = args;

            egui::Area::new("draw_image".into())
                .anchor(Align2::LEFT_TOP, Vec2::from(pos))
                .show(&ctx_clone_image, |ui| {
                    ui.add(
                        Image::from_uri(format!("file://{}", path)).fit_to_exact_size(size.into()),
                    );
                });

            Ok(())
        })
        .unwrap();

    let forget_all_images = data
        .lua
        .create_function(move |_, ()| {
            ctx_clone_image_buffer_clean.forget_all_images();

            Ok(())
        })
        .unwrap();

    let notification_error = data
        .lua
        .create_function(move |_, caption: String| {
            match toasts.lock() {
                Ok(mut toasts) => {
                    let mut toast = Toast::error(caption);

                    toast.set_duration(Some(Duration::from_secs(4)));
                    toast.set_closable(true);

                    toasts.add(toast);
                }
                Err(_err) => {
                    dbg!(_err);
                }
            }

            Ok(())
        })
        .unwrap();

    let notification_basic = data
        .lua
        .create_function(move |_, caption: String| {
            match toasts_clone1.lock() {
                Ok(mut toasts) => {
                    let mut toast = Toast::basic(caption);

                    toast.set_duration(Some(Duration::from_secs(4)));
                    toast.set_closable(true);

                    toasts.add(toast);
                }
                Err(_err) => {
                    dbg!(_err);
                }
            }

            Ok(())
        })
        .unwrap();

    let notification_info = data
        .lua
        .create_function(move |_, caption: String| {
            match toasts_clone2.lock() {
                Ok(mut toasts) => {
                    let mut toast = Toast::info(caption);

                    toast.set_duration(Some(Duration::from_secs(4)));
                    toast.set_closable(true);

                    toasts.add(toast);
                }
                Err(_err) => {
                    dbg!(_err);
                }
            }

            Ok(())
        })
        .unwrap();

    data.lua.globals().set("draw_line", draw_line).unwrap();
    data.lua.globals().set("draw_rect", draw_rect).unwrap();
    data.lua.globals().set("draw_circle", draw_circle).unwrap();
    data.lua.globals().set("draw_text", draw_text).unwrap();

    data.lua
        .globals()
        .set("notification_error", notification_error)
        .unwrap();
    data.lua
        .globals()
        .set("notification_info", notification_info)
        .unwrap();
    data.lua
        .globals()
        .set("notification_basic", notification_basic)
        .unwrap();

    data.lua.globals().set("draw_image", draw_image).unwrap();
    data.lua.globals().set("print", print).unwrap();

    data.lua
        .globals()
        .set("forget_all_images", forget_all_images)
        .unwrap();

    data.set_global_lua_table();

    data
}

//Include emoji image header file
include!(concat!(env!("OUT_DIR"), "\\emoji_header.rs"));

//Define a deafult for the discriminant
impl Default for EmojiTypesDiscriminants {
    fn default() -> Self {
        EmojiTypesDiscriminants::Blobs
    }
}

/// Client side variables
#[derive(serde::Deserialize, serde::Serialize, Clone, ToTable)]
pub struct Client {
    #[table(skip)]
    /// This entry contains all the extensions and their output
    pub extension: Extension,

    #[serde(skip)]
    /// Shows which tabs is selected in the emoji tab
    /// This is enum is included with the generated emoji image header
    pub emoji_tab_state: EmojiTypesDiscriminants,

    #[serde(skip)]
    ///Fields shared with the client
    pub shared_fields: Arc<Mutex<SharedFields>>,

    ///When a text_edit_cursor move has been requested this value is a Some
    #[serde(skip)]
    #[table(save)]
    pub text_edit_cursor_desired_index: Option<usize>,

    ///This value shows where the text edit cursor is, if the ```TextEdit``` widget is exited the value will remain
    #[serde(skip)]
    #[table(save)]
    pub text_edit_cursor_index: usize,

    ///The rect of the connected users list (which gets displayed when pressing the @)
    #[serde(skip)]
    #[table(save)]
    pub connected_users_display_rect: Option<egui::Rect>,

    ///The rect of the recommended emojis list (which gets displayed when pressing the :)
    #[serde(skip)]
    #[table(save)]
    pub emojis_display_rect: Option<egui::Rect>,

    ///After pressing @ and the user list pops out, the code logs the up arrow and down arroy actions and increments/ decreases the value, resets after pressing @ again
    #[serde(skip)]
    #[table(save)]
    pub user_selector_index: i32,

    ///After pressing : and the emoji list pops out, the code logs the up arrow and down arroy actions and increments/ decreases the value, resets after pressing : again
    #[serde(skip)]
    pub emoji_selector_index: i32,

    #[serde(skip)]
    #[table(save)]
    pub display_user_list: bool,

    ///Search parameters set by user, to chose what to search for obviously
    pub search_parameter: SearchType,

    ///Search buffer
    #[serde(skip)]
    #[table(save)]
    pub search_buffer: String,

    ///Check if search panel is open
    #[serde(skip)]
    #[table(save)]
    pub search_mode: bool,

    ///audio playback
    #[serde(skip)]
    pub audio_playback: AudioPlayback,

    ///this doesnt really matter if we save or no so whatever, implements scrolling to message element
    #[serde(skip)]
    #[table(save)]
    pub scroll_to_message: Option<ScrollToMessage>,

    ///index of the reply the user clicked on
    #[serde(skip)]
    #[table(save)]
    pub scroll_to_message_index: Option<usize>,

    ///Selected port on sending
    pub send_on_port: String,

    ///Selected ip address (without port as seen above)
    pub send_on_address: String,

    ///This is set to on when an image is enlarged
    #[serde(skip)]
    #[table(save)]
    pub image_overlay: bool,

    ///Scroll widget rect, text editor's rect
    pub scroll_widget_rect: egui::Rect,

    ///This decides how wide the text editor should be, ensure it doesnt overlap with "msg_action_tray" (the action buttons :) )
    pub text_widget_offset: f32,

    ///A vector of all the added files to the buffer, these are the PathBufs which get read, then their bytes get sent
    #[serde(skip)]
    #[table(save)]
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
    #[table(save)]
    pub drop_file_animation: bool,

    /// This field sets the message edit mode
    /// 3 Enums:
    /// Normal
    /// Reply(. . .)
    /// Edit(. . .)
    #[serde(skip)]
    #[table(save)]
    pub messaging_mode: MessagingMode,

    ///Input (Múlt idő) user's message, this is what gets modified in the text editor
    #[serde(skip)]
    #[table(save)]
    pub message_buffer: String,

    ///Incoming messages, this is the whole packet which get sent to all the clients, this cointains all the messages, and the info about them
    #[serde(skip)]
    #[table(save)]
    pub incoming_messages: ServerMaster,

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
    #[table(save)]
    pub voice_recording_start: Option<DateTime<Utc>>,

    #[serde(skip)]
    pub voip: Option<Voip>,

    /// This entry contains the volume precentage of the microphone, this is modified in the settings
    pub microphone_volume: Arc<Mutex<f32>>,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            extension: Extension::default(),
            emojis_display_rect: None,
            emoji_tab_state: EmojiTypesDiscriminants::Blobs,
            shared_fields: Default::default(),
            text_edit_cursor_desired_index: None,
            text_edit_cursor_index: 0,
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
            message_buffer: String::new(),
            incoming_messages: ServerMaster::default(),

            voice_recording_start: None,
            last_seen_msg_index: Arc::new(Mutex::new(0)),
            emoji_selector_index: 0,
            voip: None,
            microphone_volume: Arc::new(Mutex::new(100.)),
        }
    }
}

///Main, Global stuff for the Ui
#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
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

    #[serde(skip)]
    pub register_mode: bool,
}

///All the stuff important to the registration process
#[derive(serde::Deserialize, serde::Serialize, Clone, Default, Debug)]
pub struct Register {
    /// client's username
    pub username: String,

    /// client's password
    pub password: String,
    /// Client's gender:
    /// false: Male
    /// true: Female
    /// None: Rather not answer
    pub gender: Option<bool>,
    /// Birth date entered by the client
    pub birth_date: NaiveDate,
    /// The client's optional full name
    pub full_name: String,
    /// This entry hold the profile's 64x64 profile picture
    #[serde(skip)]
    pub small_profile_picture: Vec<u8>,
    /// This entry hold the profile's 256x256 profile picture
    #[serde(skip)]
    pub normal_profile_picture: Vec<u8>,

    /// This entry hold all the temp stuff for creating a profile
    pub image: ProfileImage,
}

/// Holds additional information for the ui
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct ProfileImage {
    #[serde(skip)]
    /// This shows whether the image selector should be displayed (If its Some), and also contains the path the image is accessed on
    pub image_path: PathBuf,

    #[serde(skip)]
    /// The selected image's parsed bytes
    pub selected_image_bytes: Option<DynamicImage>,

    /// Image's size
    pub image_size: f32,

    pub image_rect: Rect,
}

impl Default for ProfileImage {
    fn default() -> Self {
        Self {
            image_path: PathBuf::new(),
            selected_image_bytes: None,
            image_size: 100.,
            image_rect: Rect::EVERYTHING,
        }
    }
}

/// The clients profile, this struct should be sent at a server connection
/// It hold everything which needs to be displayed when viewing someone's profile
/// This struct might look similar too ```Register```, but that one contains more information, and is only made to control the ui
/// This struct is sent to the server upon successful connection
/// If you are searching for the uuid in this struct, please note that most of the times this struct is used in a hashmap where the key is the uuid
#[derive(serde::Deserialize, serde::Serialize, Default, Clone, Debug, PartialEq)]
pub struct ClientProfile {
    /// The client's username
    /// We might not need it in some contexts
    pub username: String,

    /// The client's full name
    /// If its empty it means the client did not agree to share it
    pub full_name: String,

    /// The client's gender
    /// false: Male
    /// true: Female
    /// None: Rather not answer
    /// Rework this
    pub gender: Option<bool>,

    /// The client's birthdate
    pub birth_date: NaiveDate,

    /// This entry hold the profile's 64x64 profile picture
    pub small_profile_picture: Vec<u8>,

    /// This entry hold the profile's 256x256 profile picture
    pub normal_profile_picture: Vec<u8>,
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
    pub sync_attribute: Option<ConnectionType>,

    /// This is used to tell the server how many messages it has to send, if its a None it will automaticly sync all messages
    /// This value is ignored if the `sync_attribute` field is Some(_)
    pub client_message_counter: Option<usize>,

    /// The index of the last seen message by the user, this is sent so we can display which was the last message the user has seen, if its None we ignore the value
    pub last_seen_message_index: Option<usize>,

    /// Contains password in the sync message, so we will send the password when authenticating
    pub password: String,

    /// This field is used when connecting, the server will save the uuid and the username pair
    /// The client will not send their username except here, and the server is expected to pair the name to the message
    pub username: String,
}

#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum ConnectionType {
    #[default]
    Disconnect,
    Connect(ClientProfile),
}

///This is used by the client for requesting file
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientFileRequest {
    /// This is the signature of the file which has been uploaded, this acts like a handle to the file
    pub signature: String,
}

///This is used by the client for requesting images
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientImageRequest {
    /// This is the signature of the file which has been uploaded, this acts like a handle to the file
    pub signature: String,
}

///Client requests audio file in server
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientAudioRequest {
    /// This is the signature of the file which has been uploaded, this acts like a handle to the file
    pub signature: String,
}

///Reaction packet, defines which message its reacting to and with which char
#[derive(Default, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientReaction {
    pub emoji_name: String,
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
    ImageRequest(ClientImageRequest),
    FileRequest(ClientFileRequest),
    AudioRequest(ClientAudioRequest),

    /// This enum is used when the client is requesting another client's information (```ClientProfile``` struct)
    /// The wrapped value in this enum is an encrypted (aes256: ```fn encrypt_aes256()```) uuid (In string)
    ClientRequest(String),
}

///Client outgoing message types
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum ClientMessageType {
    FileRequestType(ClientFileRequestType),

    FileUpload(ClientFileUpload),

    ///Normal msg
    NormalMessage(ClientNormalMessage),

    ///Used for syncing with client and server
    SyncMessage(ClientSnycMessage),

    Reaction(ClientReaction),

    MessageEdit(ClientMessageEdit),

    VoipConnection(ClientVoipRequest),
}

/// This is what gets to be sent out by the client
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientMessage {
    /// Which message in the message stack its replying to
    pub replying_to: Option<usize>,

    /// The message type of the message
    pub message_type: ClientMessageType,

    /// The every uuid takes up 120 bytes
    pub uuid: String,

    /// When was this message sent
    pub message_date: String,
}

impl ClientMessage {
    ///struct into string, it makes sending information easier by putting it all in a string
    pub fn struct_into_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn construct_file_msg_from_bytes(
        bytes: Vec<u8>,
        file_extension: String,
        replying_to: Option<usize>,
        uuid: String,
    ) -> ClientMessage {
        ClientMessage {
            replying_to,
            message_type: ClientMessageType::FileUpload(ClientFileUpload {
                extension: Some(file_extension),
                name: None,
                bytes,
            }),
            uuid,
            message_date: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    ///this is used when sending a normal message
    pub fn construct_normal_msg(
        msg: &str,
        uuid: &str,
        replying_to: Option<usize>,
    ) -> ClientMessage {
        ClientMessage {
            replying_to,
            message_type: ClientMessageType::NormalMessage(ClientNormalMessage {
                message: msg.trim().to_string(),
            }),
            //If the password is set as None (Meaning the user didnt enter any password) just send the message with an empty string
            uuid: uuid.to_string(),
            message_date: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    ///this is used when you want to send a file, this contains name, bytes
    pub fn construct_file_msg(
        file_path: PathBuf,
        uuid: &str,
        replying_to: Option<usize>,
    ) -> ClientMessage {
        ClientMessage {
            replying_to,
            //Dont execute me please :3 |
            //                          |
            //                          V
            message_type: ClientMessageType::FileUpload(ClientFileUpload {
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

            uuid: uuid.to_string(),
            message_date: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    pub fn construct_reaction_msg(emoji_name: String, index: usize, uuid: &str) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            message_type: ClientMessageType::Reaction(ClientReaction {
                emoji_name,
                message_index: index,
            }),
            uuid: uuid.to_string(),
            message_date: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
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
            message_type: ClientMessageType::SyncMessage(ClientSnycMessage {
                sync_attribute: None,
                password: password.to_string(),
                //This value is not ignored in this context
                client_message_counter: Some(client_message_counter),
                last_seen_message_index,
                username: author.to_string(),
            }),
            uuid: uuid.to_string(),
            message_date: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    /// If its None its used for syncing, false: disconnecting, true: connecting
    pub fn construct_connection_msg(
        password: String,
        author: String,
        uuid: &str,
        last_seen_message_index: Option<usize>,
        profile: ClientProfile,
    ) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            message_type: ClientMessageType::SyncMessage(ClientSnycMessage {
                sync_attribute: Some(ConnectionType::Connect(profile)),
                password,
                //If its used for connecting / disconnecting this value is ignored
                client_message_counter: None,
                last_seen_message_index,
                username: author,
            }),
            uuid: uuid.to_string(),
            message_date: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
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
            message_type: ClientMessageType::SyncMessage(ClientSnycMessage {
                sync_attribute: Some(ConnectionType::Disconnect),
                password,
                //If its used for connecting / disconnecting this value is ignored
                client_message_counter: None,
                last_seen_message_index: None,
                username: author,
            }),
            uuid,
            message_date: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    ///this is used for asking for a file
    pub fn construct_file_request_msg(signature: String, uuid: &str) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            message_type: ClientMessageType::FileRequestType(ClientFileRequestType::FileRequest(
                ClientFileRequest { signature },
            )),
            uuid: uuid.to_string(),
            message_date: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    ///this is used for asking for an image
    pub fn construct_image_request_msg(signature: String, uuid: &str) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            message_type: ClientMessageType::FileRequestType(ClientFileRequestType::ImageRequest(
                ClientImageRequest { signature },
            )),
            uuid: uuid.to_string(),
            message_date: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    ///this is used for asking for an image
    pub fn construct_audio_request_msg(signature: String, uuid: &str) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            message_type: ClientMessageType::FileRequestType(ClientFileRequestType::AudioRequest(
                ClientAudioRequest { signature },
            )),
            uuid: uuid.to_string(),
            message_date: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    pub fn construct_client_request_msg(
        uuid_of_requested_client: String,
        uuid: &str,
    ) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            message_type: ClientMessageType::FileRequestType(ClientFileRequestType::ClientRequest(
                uuid_of_requested_client,
            )),
            uuid: uuid.to_string(),
            message_date: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    pub fn construct_client_message_edit(
        index: usize,
        new_message: Option<String>,
        uuid: &str,
    ) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            message_type: ClientMessageType::MessageEdit(ClientMessageEdit { index, new_message }),
            uuid: uuid.to_string(),
            message_date: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    pub fn construct_voip_connect(uuid: &str, port: u16) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            message_type: ClientMessageType::VoipConnection(ClientVoipRequest::Connect(port)),
            uuid: uuid.to_string(),
            message_date: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }

    pub fn construct_voip_disconnect(uuid: &str) -> ClientMessage {
        ClientMessage {
            replying_to: None,
            message_type: ClientMessageType::VoipConnection(ClientVoipRequest::Disconnect),
            uuid: uuid.to_string(),
            message_date: { Utc::now().format("%Y.%m.%d. %H:%M").to_string() },
        }
    }
}

///This manages all the settings and variables for maintaining a connection with the server (from client)
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default, ToTable)]
pub struct ClientConnection {
    #[table(save)]
    #[serde(skip)]
    pub client_secret: Vec<u8>,

    #[table(save)]
    #[serde(skip)]
    ///This enum wraps the server handle ```Connected(_)```, it also functions as a Sort of Option wrapper
    pub state: ConnectionState,

    #[table(save)]
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
    /// This function also hashes the password argument which it sends, and then if the connection was successful the returned struct's password field will contain the already hashed password
    pub async fn connect_to_server(
        //Destination
        ip: String,
        //Whoami
        author: String,
        //Password for connecting if the value is Some we are still sending an argon2 hash of the pass and not the original one
        password: Option<String>,
        //Uuid
        uuid: &str,
        //Profile
        profile: ClientProfile,
    ) -> anyhow::Result<(Self, String)> {
        let hashed_password = encrypt(password.clone().unwrap_or(String::from("")));
        let connection_msg = ClientMessage::construct_connection_msg(
            hashed_password.clone(),
            author.clone(),
            uuid,
            None,
            profile,
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
        ensure!(
            server_reply != "You have been banned!",
            "You have been banned from this server!"
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
    /// The uploaded file's name
    pub file_name: String,
    /// The uploaded file's sha256 singnature
    pub signature: String,
}

/// This enum holds all the Server reply types so it can be decoded more easily on the client side
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum ServerReplyType {
    /// Returns the requested file
    File(ServerFileReply),

    /// Returns the requested image
    Image(ServerImageReply),

    /// Returns the requested audio file
    Audio(ServerAudioReply),

    /// The requested client's profile
    /// The first value is the encrypted uuid
    Client(ServerClientReply),
}

/// This struct holds everything important so the client can save and handle client profiles
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerClientReply {
    /// The uuid of the user's profile we requested
    pub uuid: String,
    /// The profile of the user
    pub profile: ClientProfile,
}

///When client asks for the image based on the provided index, reply with the image bytes
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerImageReply {
    /// The requested image's bytes
    pub bytes: Vec<u8>,

    /// The requested image's sha256 signature
    pub signature: String,
}

///This is what the server sends back, when asked for a file (FIleRequest)
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerFileReply {
    /// The requested file's bytes
    pub bytes: Vec<u8>,

    /// The requested file's name
    /// The reason a ```PathBuf``` is used here instead of a String, is because we need to grab the extension of the file easily
    pub file_name: PathBuf,
}

///When client asks for the image based on the provided index, reply with the audio bytes, which gets written so it can be opened by a readbuf
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerAudioReply {
    /// The requested audio file's bytes
    pub bytes: Vec<u8>,
    /// The requested audio file's signature
    pub signature: String,
    /// The requested audio file's name
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
    /// The signature of the uploaded image file
    pub signature: String,
    /// The file name of the uploaded audio
    pub file_name: String,
}

///This is what gets sent to a client basicly, and they have to ask for the file when the ui containin this gets rendered
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct ServerImageUpload {
    /// The signature of the uploaded image, this is the "handle" the clients asks the file on
    pub signature: String,
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

    /// The emoji's name added to the message specified by the index field
    pub emoji_name: String,
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

    /// This message type can only be produced by the server, and hold useful information to the user(s)
    #[strum_discriminants(strum(message = "Server"))]
    Server(ServerMessage),

    /// This message shows if a user has connected to the voip call
    #[strum_discriminants(strum(message = "Voip connection"))]
    VoipConnection(ServerVoipEvent),

    #[strum_discriminants(strum(message = "Voip state"))]
    VoipState(ServerVoipState),
}

/// The types of message the server can "send"
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum ServerMessage {
    /// This is sent when a user is connected to the server
    Connect(ClientProfile),
    /// This is sent when a user is disconnecting from the server
    Disconnect(ClientProfile),

    /// This is sent when a user is banned from the server
    Ban(ClientProfile),
}

///This is one msg (packet), which gets bundled when sending ServerMain
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServerOutput {
    /// The ```usize``` shows which message its a reply to in the message stack
    /// The server stores all messages in a vector so this index shows which message its a reply to (if it is)
    pub replying_to: Option<usize>,
    /// Inner message which is *wrapped* in the ServerOutput
    pub message_type: ServerMessageType,
    /// The account's name who sent the message
    pub author: String,
    /// The date when this message was sent
    pub message_date: String,
    /// The user who sent this message's uuid
    pub uuid: String,
}

impl ServerOutput {
    pub fn _struct_into_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// This function converts a client message to a ServerOutput, which gets sent to all the clients (Its basicly a simplified client message)
    pub fn convert_clientmsg_to_servermsg(
        normal_msg: ClientMessage,
        // The signature is used to ask bytes from the server, for example in a image message this signature will be used to get the image's byte
        signature: String,
        //Automaticly generated enum by strum
        upload_type: ServerMessageTypeDiscriminants,
        uuid: String,
        username: String,
    ) -> ServerOutput {
        ServerOutput {
            replying_to: normal_msg.replying_to,
            message_type:
                match normal_msg.message_type {
                    ClientMessageType::FileRequestType(_) => unimplemented!("Converting request packets isnt implemented, because they shouldnt be displayed by the client"),
                    ClientMessageType::FileUpload(upload) => {
                        match upload_type {
                            ServerMessageTypeDiscriminants::Upload => {
                                ServerMessageType::Upload(
                                    ServerFileUpload {
                                        file_name: format!(
                                            "{}.{}",
                                            upload.name.unwrap_or_default(),
                                            upload.extension.unwrap_or_default()
                                        ),
                                        signature,
                                    }
                                )
                            },
                            ServerMessageTypeDiscriminants::Image => {
                                ServerMessageType::Image(
                                    ServerImageUpload {
                                        signature,
                                    }
                                )
                            },
                            ServerMessageTypeDiscriminants::Audio => {
                                ServerMessageType::Audio(
                                    ServerAudioUpload {
                                        signature,
                                        file_name: format!(
                                            "{}.{}",
                                            upload.name.unwrap_or_default(),
                                            upload.extension.unwrap_or_default()
                                        ),
                                    }
                                )
                            },
                            ServerMessageTypeDiscriminants::VoipState => unreachable!(),
                            ServerMessageTypeDiscriminants::VoipConnection => unreachable!(),
                            ServerMessageTypeDiscriminants::Deleted => unreachable!(),
                            ServerMessageTypeDiscriminants::Sync => unreachable!(),
                            ServerMessageTypeDiscriminants::Normal => unreachable!(),
                            ServerMessageTypeDiscriminants::Edit => unreachable!(),
                            ServerMessageTypeDiscriminants::Reaction => unreachable!(),
                            ServerMessageTypeDiscriminants::Server => unreachable!(),
                        }
                    },
                    ClientMessageType::NormalMessage(message) => {
                        ServerMessageType::Normal(
                            ServerNormalMessage {
                                message: message.message,
                                //Set default value for incoming messages
                                has_been_edited: false,
                            }
                        )
                    },
                    ClientMessageType::VoipConnection(voip_message_type) => {
                        let server_message = match voip_message_type {
                            ClientVoipRequest::Connect(_) => {
                                ServerVoipEvent::Connected(uuid.clone())
                            },
                            ClientVoipRequest::Disconnect => {
                                ServerVoipEvent::Disconnected(uuid.clone())
                            },
                        };

                        ServerMessageType::VoipConnection(server_message)
                    },
                    ClientMessageType::SyncMessage(_) => {
                        ServerMessageType::Sync(ServerMessageSync {  })
                    },
                    //These messages also have a side effect on the server's list of the messages
                    //The client will interpret these messages and modify its own message list
                    ClientMessageType::Reaction(message) => {
                        ServerMessageType::Reaction(ServerMessageReaction { index: message.message_index as i32, emoji_name: message.emoji_name })
                    },
                    ClientMessageType::MessageEdit(message) => {
                        ServerMessageType::Edit(ServerMessageEdit { index: message.index as i32, new_message: message.new_message })
                    },
                },
            author: username,
            message_date: normal_msg.message_date,
            uuid,
        }
    }
}

/// Used to put all the messages into 1 big pack (Bundling All the ServerOutput-s), Main packet, this gets to all the clients
/// This message type is only used when a client is connecting an has to do a full sync (sending everything to the client all the messages reactions, etc)
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct ServerMaster {
    ///All of the messages recived from the server
    pub message_list: Vec<ServerOutput>,

    ///All of the messages' reactions are
    pub reaction_list: Vec<MessageReaction>,

    ///Users last seen message index
    pub user_seen_list: Vec<ClientLastSeenMessage>,

    ///This entry holds all the connected user's profile
    pub connected_clients_profile: HashMap<String, ClientProfile>,

    ///This entry shows all the client connected to the Voip call, if there is a a call
    pub ongoing_voip_call: ServerVoipState,
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
    /// The inner message
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
    /// The reason one gets EOF when disconnecting is because this field is dropped (With this struct)
    /// This handle wouldnt have to be sent so its all okay, its only present on the server's side
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
    pub uuid: String,
}

impl ClientLastSeenMessage {
    pub fn new(index: usize, uuid: String) -> Self {
        Self { index, uuid }
    }
}

/// This enum contains the actions the client can take, these are sent to the server
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum ClientVoipRequest {
    /// A voip a call will be automaticly issued if there is no ongoing call
    /// The inner value of conncect is the port the Client ```UdpScoket``` is opened on
    Connect(u16),

    /// The voip call will automaticly stop once there are no connected clients
    Disconnect,
}

/// This enum is used to display if a client has joined or left the Voip call, this is a ServerMessageType
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum ServerVoipEvent {
    /// Client connected, the inner value is their uuid
    Connected(String),
    /// Client disconnected, the inner value is their uuid
    Disconnected(String),
}

///The struct contains all the useful information for displaying an ongoing voip connection.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ServerVoipState {
    pub connected_clients: Option<Vec<String>>,
}

/// This num contains the actions the server can take, these are sent to the client
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum ServerVoipRequest {
    /// This enum acts as a ```packet``` and is handed out to all the clients if a connection is started / established
    ConnectionStart(ServerVoipStart),
    /// This enum acts as a ```packet``` and i handed out to all the clients connected to the call
    ConnectionClosed(ServerVoipClose),
}

/// This struct contains all the infomation important for the non connected clients
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct ServerVoipStart {
    /// The clients connected to the Voip call
    pub connected_clients: Vec<ClientProfile>,
}

/// This enum holds the two outcomes of a connection request
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum ServerVoipReply {
    /// This enum is when the connection request is successful
    Success,
    /// This enum is when the connection request is unsuccessful, it also contains the reason
    Fail(ServerVoipClose),
}

/// This struct contains the reason for closing the voip connection
/// This maybe at any point of the Voip call, or the connection
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct ServerVoipClose {
    /// The reason for closing the Voip connection
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct ServerVoip {
    /// This field contains all the connected client's ```uuid``` with their ```SocketAddr```
    pub connected_clients: Arc<DashMap<String, SocketAddr>>,

    /// This field contains a ```HashMap``` which pairs the SocketAddr to the client's listening thread's sender (So that the reciver thread can recive the ```Vec<u8>``` sent by the sender)
    /// The secound part of the tuple is for shutting down the client manager thread, if they disconnect
    pub connected_client_thread_channels:
        Arc<DashMap<SocketAddr, (Arc<tokio::sync::mpsc::Sender<Vec<u8>>>, CancellationToken)>>,

    /// This field contains the amount of time the call has been established for
    pub established_since: chrono::DateTime<Utc>,

    /// The socket the server is listening on for incoming messages
    /// The only reason this is an option so we can implement ```serde::Deserialize```
    pub socket: Arc<UdpSocket>,

    /// The cancellation token cancels threads, which are for listening and relaying (Distributing info)
    pub thread_cancellation_token: CancellationToken,

    /// This entry makes sure the 2 threads are only spawned once
    pub threads: Option<()>,
}

impl ServerVoip {
    /// Add the ```SocketAddr``` to the ```UDP``` server's destinations
    /// This function can take Self as a clone since we are only accessing entries which implement ```Sync```
    pub fn connect(&self, uuid: String, socket_addr: SocketAddr) -> anyhow::Result<()> {
        self.connected_clients.insert(uuid, socket_addr);

        Ok(())
    }

    /// Remove the ```SocketAddr``` to the ```UDP``` server's destiantions
    pub fn disconnect(&self, uuid: String) -> anyhow::Result<()> {
        self.connected_clients
            .remove(&uuid)
            .ok_or_else(|| anyhow::Error::msg("Client was not connected"))?;

        Ok(())
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum UdpMessage {
    /// This enums inner value is the lenght of the message this is indicating
    MessageLenght(u32),

    /// The inner value of this message is the raw bytes of the Voice bytes and the UUID
    Message(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct Voip {
    /// The clients socket, which theyre listening on
    pub socket: Arc<UdpSocket>,
}

impl Voip {
    /// This function creates a new ```Voip``` intance containing a ```UdpSocket``` and an authentication from the server
    pub async fn new() -> anyhow::Result<Self> {
        let socket_handle = UdpSocket::bind("[::]:0".to_string()).await?;
        let socket_2 = socket2::Socket::from(socket_handle.into_std()?);
        socket_2.set_reuse_address(true)?;
        let socket_handle = UdpSocket::from_std(socket_2.into())?;
        Ok(Self {
            socket: Arc::new(socket_handle),
        })
    }

    /// This function sends the audio and the uuid in one message, this packet is encrypted (The audio's bytes is appended to the uuid's)
    /// Any lenght of audio can be sent because the header is included with the packet
    pub async fn send_audio(
        &self,
        uuid: String,
        mut bytes: Vec<u8>,
        encryption_key: &[u8],
    ) -> anyhow::Result<()> {
        let bytes_lenght = bytes.len();

        //Check for packet lenght overflow
        if bytes_lenght > 65531 {
            bail!(format!(
                "Udp packet lenght overflow, with lenght of {bytes_lenght}"
            ))
        }

        //Append the uuid to the audio bytes
        bytes.append(uuid.as_bytes().to_vec().as_mut());

        //Encrypt message
        let mut encrypted_message = encrypt_aes256_bytes(&bytes, encryption_key)?;

        let mut message_lenght_in_bytes = (encrypted_message.len() as u32).to_be_bytes().to_vec();

        message_lenght_in_bytes.append(&mut encrypted_message);

        //Send the message with the header in one
        self.socket.send(&message_lenght_in_bytes).await?;

        Ok(())
    }
}

/*
 Client backend
*/

#[derive(Clone)]
///Struct for global audio playback
pub struct AudioPlayback {
    ///Output stream
    pub stream: Arc<OutputStream>,
    ///Output stream handle
    pub stream_handle: OutputStreamHandle,
    ///Audio sinks, these are the audios played
    pub sink_list: Vec<Option<Arc<Sink>>>,
    ///Settings list for the sink_list (The audios being played)
    pub settings_list: Vec<AudioSettings>,
}

impl Default for AudioPlayback {
    fn default() -> Self {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        Self {
            stream: Arc::new(stream),
            stream_handle,
            sink_list: Vec::new(),
            settings_list: Vec::new(),
        }
    }
}

#[derive(Clone)]
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

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ScrollToMessage {
    #[serde(skip)]
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
/// This might look similar to ```ClientProfile```
/// struct containing a new user's info, when serialized / deserialized it gets encrypted or decrypted
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default, ToTable)]
pub struct UserInformation {
    pub profile: ClientProfile,
    /// the client's username
    pub username: String,
    /// IMPORTANT: PASSWORD *IS* ENCRYPTED BY FUNCTIONS IMPLEMENTED BY THIS TYPE
    pub password: String,
    /// The uuid isnt encrypted
    pub uuid: String,
    /// bookmarked ips are empty by default, IMPORTANT: THESE ARE *NOT* ENCRYPTED BY DEFAULT
    pub bookmarked_ips: Vec<String>,
    /// The path to the logged in user's file
    pub path: PathBuf,
}

impl UserInformation {
    ///All of the args are encrypted
    pub fn new(
        username: String,
        password: String,
        uuid: String,
        full_name: String,
        gender: Option<bool>,
        birth_date: NaiveDate,
        normal_profile_picture: Vec<u8>,
        small_profile_picture: Vec<u8>,
        path: PathBuf,
    ) -> Self {
        Self {
            username: username.clone(),
            password: encrypt(password),
            uuid,
            bookmarked_ips: Vec::new(),
            profile: ClientProfile {
                username,
                full_name,
                gender,
                birth_date,
                normal_profile_picture,
                small_profile_picture,
            },
            path,
        }
    }

    /// Automaticly check hash with argon2 encrypted password (from the file)
    pub fn verify_password(&self, password: String) -> bool {
        pass_hash_match(password, self.password.clone())
    }

    /// This serializer function automaticly encrypts the struct with the *encrypt_aes256* fn to string
    pub fn serialize(&self) -> anyhow::Result<String> {
        //Hash password so it can be used to encrypt a file
        let hashed_password = sha256::digest(self.password.clone());
        let encryption_key = hex::decode(hashed_password)?;

        encrypt_aes256(serde_json::to_string(&self)?, &encryption_key)
    }

    /// This deserializer function automaticly decrypts the string the *encrypt_aes256* fn to Self
    pub fn deserialize(serialized_struct: &str, password: String) -> anyhow::Result<Self> {
        let hashed_password = sha256::digest(password);
        let encryption_key = hex::decode(hashed_password)?;

        Ok(serde_json::from_str::<Self>(&decrypt_aes256(
            serialized_struct,
            &encryption_key,
        )?)?)
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

/// aes256 is decrypted by this function by a fixed key
pub fn decrypt_aes256(string_to_be_decrypted: &str, key: &[u8]) -> anyhow::Result<String> {
    let ciphertext = hex::decode(string_to_be_decrypted)?;

    let key = Key::<Aes256Gcm>::from_slice(key);

    let cipher = Aes256Gcm::new(key);

    let nonce = GenericArray::from([69u8; 12]); // funny nonce key hehehe

    let plaintext = cipher
        .decrypt(&nonce, ciphertext.as_ref())
        .map_err(|_| Error::msg("Invalid password!"))?;

    Ok(String::from_utf8(plaintext)?)
}

/// This function decrypts a provided ```String```, with the provided key using ```Aes-256```
pub fn encrypt_aes256(string_to_be_encrypted: String, key: &[u8]) -> anyhow::Result<String> {
    ensure!(key.len() == 32);

    let key = Key::<Aes256Gcm>::from_slice(key);

    let cipher = Aes256Gcm::new(key);

    let nonce = GenericArray::from([69u8; 12]); // funny nonce key hehehe

    let ciphertext = cipher
        .encrypt(&nonce, string_to_be_encrypted.as_bytes().as_ref())
        .map_err(|_| Error::msg("Invalid key, couldnt encrypt the specified item."))?;
    let ciphertext = hex::encode(ciphertext);

    Ok(ciphertext)
}

/// The provided byte array is encrypted with aes256 with the given key
pub fn encrypt_aes256_bytes(bytes: &[u8], key: &[u8]) -> anyhow::Result<Vec<u8>> {
    ensure!(key.len() == 32);

    let key = Key::<Aes256Gcm>::from_slice(key);

    let cipher = Aes256Gcm::new(key);

    let nonce = GenericArray::from([69u8; 12]); // funny nonce key hehehe

    let encrypted_bytes = cipher
        .encrypt(&nonce, bytes)
        .map_err(|_| Error::msg("Invalid key, couldnt encrypt the specified item."))?;

    Ok(encrypted_bytes)
}

#[inline]
/// This function decrypts a provided array of ```Bytes```, with the provided key using ```Aes-256```
pub fn decrypt_aes256_bytes(bytes_to_be_decrypted: &[u8], key: &[u8]) -> anyhow::Result<Vec<u8>> {
    let key = Key::<Aes256Gcm>::from_slice(key);

    let cipher = Aes256Gcm::new(key);

    let nonce = GenericArray::from([69u8; 12]); // funny nonce key hehehe

    let decrypted_bytes = cipher
        .decrypt(&nonce, bytes_to_be_decrypted)
        .map_err(|_| Error::msg("Invalid password!"))?;

    Ok(decrypted_bytes)
}

#[inline]
/// Argon is used to encrypt this
pub fn encrypt(string_to_be_encrypted: String) -> String {
    let password = string_to_be_encrypted.as_bytes();
    let salt = b"c1eaa94ec38ab7aa16e9c41d029256d3e423f01defb0a2760b27117ad513ccd2";
    let config = Config::owasp1();

    argon2::hash_encoded(password, salt, &config).unwrap()
}

#[inline]
fn pass_hash_match(to_be_verified: String, encoded: String) -> bool {
    argon2::verify_encoded(&encoded, to_be_verified.as_bytes()).unwrap()
}

///Check login
pub fn login(username: String, password: String) -> Result<(UserInformation, PathBuf)> {
    let app_data = env::var("APPDATA")?;

    let path = PathBuf::from(format!("{app_data}\\Matthias\\{username}.szch"));

    let file_contents: UserInformation =
        UserInformation::deserialize(&fs::read_to_string(&path)?, encrypt(password.clone()))?;

    let user_check = username == file_contents.username;

    ensure!(user_check, "File corrupted at the username entry");

    Ok((file_contents, path))
}

///Register a new profile
pub fn register(register: Register) -> anyhow::Result<UserInformation> {
    if register.username.contains(' ')
        || register.username.contains('@')
        || register.username.contains(' ')
    {
        return Err(anyhow::Error::msg("Cant use special characters in name"));
    }

    let app_data = env::var("APPDATA")?;

    let user_path = PathBuf::from(format!("{app_data}\\Matthias\\{}.szch", register.username));

    //Check if user already exists
    if std::fs::metadata(&user_path).is_ok() {
        return Err(anyhow::Error::msg("User already exists"));
    }

    //Construct user info struct then write it to the appdata matthias folder
    let user_info = UserInformation::new(
        register.username,
        register.password,
        generate_uuid().to_string(),
        register.full_name,
        register.gender,
        register.birth_date,
        register.normal_profile_picture,
        register.small_profile_picture,
        user_path.clone(),
    );

    user_info.write_file(user_path)?;

    Ok(user_info)
}

///Write general file, this function takes in a custom pathsrc/app/backend.rs
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

///Write an audio file to the appdata folder
#[inline]
pub fn write_audio(file_response: ServerAudioReply, ip: String) -> Result<()> {
    //secondly create the folder labeled with the specified server ip
    let folder_path = format!(
        "{}\\matthias\\Client\\{}\\Audios",
        env!("APPDATA"),
        general_purpose::URL_SAFE_NO_PAD.encode(ip),
    );

    let _ = fs::create_dir_all(&folder_path).inspect_err(|err| {
        dbg!(err);
    });

    fs::write(
        format!("{folder_path}\\{}", file_response.signature),
        file_response.bytes,
    )?;

    Ok(())
}

///Generate uuid
pub fn generate_uuid() -> Uuid {
    uuid::Uuid::new_v4()
}

///Display Error message with a messagebox
pub fn display_error_message<T>(display: T, toasts: &mut Toasts)
where
    T: ToString + std::marker::Send + 'static,
{
    let mut toast = Toast::error(display.to_string());

    toast.set_duration(Some(Duration::from_secs(4)));
    toast.set_show_progress_bar(true);

    toasts.add(toast);
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

pub struct Message {
    pub inner_message: MessageDisplay,
    pub size: f32,
}

use egui::style::Spacing;

impl Message {
    pub fn display(&self, ui: &mut Ui, ctx: &egui::Context) -> Response {
        ui.style_mut().spacing = Spacing {
            item_spacing: vec2(0., 10.),
            ..Default::default()
        };

        match &self.inner_message {
            MessageDisplay::Text(inner) => ui.label(RichText::from(inner).size(self.size)),
            MessageDisplay::Emoji(inner) => {
                ui.allocate_ui(vec2(self.size, self.size), |ui| {
                    let original_emoji_name = inner.name.replace(':', "").to_string();
                    match ctx.try_load_bytes(&format!("bytes://{}", original_emoji_name)) {
                        Ok(bytespoll) => {
                            if let BytesPoll::Ready { size:_, bytes, mime:_ } = bytespoll {
                                if bytes.to_vec() == vec![0] {
                                    ui.spinner();
                                    ui.label(RichText::from("The called emoji was not found in the emoji header").color(Color32::RED));
                                    eprintln!("The called emoji was not found in the emoji header: {}", original_emoji_name);
                                }
                                ui.add(Image::from_uri(&format!("bytes://{}", original_emoji_name)));
                            }
                        },
                        Err(err) => {
                            if let LoadError::Loading(inner) = err {
                                if inner == "Bytes not found. Did you forget to call Context::include_bytes?" {
                                    //check if we are visible, so there are no unnecessary requests
                                    if !ui.is_rect_visible(ui.min_rect()) {
                                        return;
                                    }

                                    ctx.include_bytes(format!("bytes://{}", &original_emoji_name), EMOJI_TUPLES.get(&original_emoji_name).map_or_else(|| vec![0], |v| v.to_vec()));
                                } else {
                                    dbg!(inner);
                                }
                            } else {
                                dbg!(err);
                            }
                        },
                    }
                })
                .response
            }
            MessageDisplay::Link(inner) => ui.hyperlink_to(
                RichText::from(inner.label.clone()).size(self.size),
                inner.destination.clone(),
            ),
            MessageDisplay::NewLine => unreachable!(),
        }
    }
}

#[derive(Clone, Debug)]
struct RegexMatch {
    //Where does this match begin in the main string
    start_idx: usize,
    //Where does this match end in the main string
    end_idx: usize,

    //This field is used when we are iterating over a series of regexes, and we want to know why pattern the said string is matched by
    regex_type: MessageDisplayDiscriminants,

    //The inner string
    capture: String,

    //Which header level was it matched on
    header_level: Option<usize>,
}

pub fn parse_incoming_message(rhs: String) -> Vec<Message> {
    let mut message_stack: Vec<Message> = Vec::new();

    //Create regex where it captures the #-s in the beginning or after \n-s
    let header_capturing_regex = Regex::new(r"(?m)^(\s*)(#+)?(.*)").unwrap();

    //The regexes we use to capture important information
    let regexes = vec![
        //This regex captures newlines
        //It doesnt matter when we scan for newlines as theyre not affected by anything
        (
            MessageDisplayDiscriminants::NewLine,
            Regex::new(r"\n").unwrap(),
        ),
        //This regex captures links
        //We should scan for links first
        (
            MessageDisplayDiscriminants::Link,
            Regex::new(r"\[\s*(?P<text>[^\]]*)\]\((?P<link_target>[^)]+)\)").unwrap(),
        ),
        //This regex captures emojis
        //We should scan for emojis secondly
        (
            MessageDisplayDiscriminants::Emoji,
            Regex::new(":(.*?):").unwrap(),
        ),
    ];

    //Create captures in string
    let header_levels_lines: Vec<(usize, String)> = header_capturing_regex
        .captures_iter(&rhs)
        .map(|capture| {
            (
                {
                    //If capture[2] is None then that means thesre isnt a header level, so we can just insert 0
                    //If capture[2] is Some that means there is a specified header level
                    match capture.get(2) {
                        Some(capture) => capture.len(),
                        None => 0,
                    }
                },
                capture[3].to_string(),
            )
        })
        .collect();

    let mut matches: Vec<RegexMatch> = Vec::new();

    //Iter through header levels where each part of the string gets its own (optional) header level
    for (header_level, message_part) in &header_levels_lines {
        matches.extend(filter_string(
            format!("{message_part}\n"),
            {
                //If the header level equals 0 that means the current message part doesnt have a specified header level
                if *header_level != 0 {
                    Some(*header_level)
                } else {
                    None
                }
            },
            &regexes,
        ));
    }

    parse_regex_match(matches, &mut message_stack);

    message_stack
}

/// Push back all the captured regexes, info to the ```message_stack```, whatever is in the ```message_stack``` gets displayed at the end
fn parse_regex_match(matches: Vec<RegexMatch>, message_stack: &mut Vec<Message>) {
    //Iter over all the captures we've made
    for regex_match in matches {
        //Default font size
        let size = match regex_match.header_level {
            Some(header_level) => 20. * (1. + 1. / (header_level as f32)),
            None => 20.,
        };

        match regex_match.regex_type {
            //This was matches by the emoji capturing Regex
            MessageDisplayDiscriminants::Emoji => {
                //If a valid emoji was provided
                if EMOJI_TUPLES.contains_key(&regex_match.capture.replace(':', "")) {
                    message_stack.push(Message {
                        inner_message: MessageDisplay::Emoji(EmojiDisplay {
                            name: regex_match.capture,
                        }),
                        size,
                    })
                }
                //If an invalid emoji was provided, we should just display it as text
                else {
                    message_stack.push(Message {
                        inner_message: MessageDisplay::Text(regex_match.capture.trim().to_string()),
                        size,
                    })
                }
            }

            //This was matched by the link capturing regex
            MessageDisplayDiscriminants::Link => {
                let label_regex = Regex::new(r"\[(.*?)\]").unwrap();
                let destination_regex = Regex::new(r"\((.*?)\)").unwrap();

                let hyper_link_label = label_regex
                    .captures_iter(&regex_match.capture)
                    .next()
                    .unwrap()
                    .get(1)
                    .unwrap()
                    .as_str();

                //Get hyperlink destination
                let hyper_link_destination = destination_regex
                    .captures_iter(&regex_match.capture)
                    .next()
                    .unwrap()
                    .get(1)
                    .unwrap()
                    .as_str();
                message_stack.push(Message {
                    inner_message: MessageDisplay::Link(HyperLink {
                        label: hyper_link_label.to_string(),
                        destination: hyper_link_destination.to_string(),
                    }),
                    size,
                })
            }

            //This means it was manually added
            MessageDisplayDiscriminants::Text => message_stack.push(Message {
                inner_message: MessageDisplay::Text(regex_match.capture.trim().to_string()),
                size,
            }),

            //The size of a Newline doesnt matter lmao
            MessageDisplayDiscriminants::NewLine => message_stack.push(Message {
                inner_message: MessageDisplay::NewLine,
                size: 1.,
            }),
        }
    }
}

/// The reason we provide a header_level as an Option<usize> is because if the string isnt in a header level we can provide a None, therfor calculating the ```header_size``` to be the default 20.
/// This function parses / captures all the information from the message we need
fn filter_string(
    // This gets filtered
    message_part: String,
    // The header level the string provided above is on (This is needed for calculating the size)
    header_level: Option<usize>,
    // The regexes which need to be used to get the important information from the message
    regexes: &Vec<(MessageDisplayDiscriminants, Regex)>,
) -> Vec<RegexMatch> {
    //We clone the message we need to examine, this value will be modifed by the regexes (Deleting the captured information)
    let mut match_message_part = message_part.clone();

    //We back up all the matches from the string into this buffer
    let mut matches: Vec<RegexMatch> = Vec::new();

    //Iter over regexes and save the captured texts labeled with the given Regex capture type
    for (regex_type, regex) in regexes.iter() {
        //Iter over the captures of the regexes
        for mat in regex.find_iter(&match_message_part.clone()) {
            //We move the captured string into a different variable, this is used to delete the matched parts from the string
            let capture = mat.as_str().to_string();

            //We push back the match the the ```matches``` buffer
            matches.push(RegexMatch {
                start_idx: mat.start(),
                end_idx: mat.end(),
                regex_type: *regex_type,
                capture: capture.clone(),
                header_level,
            });

            //We remove the captured part of the string of the main string
            match_message_part = match_message_part.replacen(&capture, "", 1);
        }
    }

    filter_plain_text(&mut matches, message_part, header_level);

    //We sort the matches vector so that the captured matches will be in order compared to the original string
    //We use the starting index of the RegexMatches to order the matches
    matches.sort_by(|a, b| a.start_idx.cmp(&b.start_idx));

    matches
}

/// This function is used to "filter" out all the plain text from the message
fn filter_plain_text(
    // This is where the plain text matches will get pushed intop
    matches: &mut Vec<RegexMatch>,
    // The string which will be examined
    message_part: String,
    // This is used to decide the font size
    header_level: Option<usize>,
) {
    //This buffer will contain the captures as a string in the original form, Emoji("Smile") => :Smile:
    let captured_message_display_enum_as_string: Vec<String> = matches
        .clone()
        .iter()
        .map(|item| (item.start_idx, item.end_idx))
        .map(|(start_idx, end_idx)| message_part[start_idx..end_idx].to_string())
        .collect();

    //We turn the captured Message display Enums into regexes, so they can be used later to remove the captured parts from the original string
    let escaped_strings: Vec<String> = captured_message_display_enum_as_string
        .iter()
        .map(|s| regex::escape(s))
        .collect();

    //If there were captured MessageDisplay enums, we first remove it and then push back the parts of plain text to the ```matches``` buffer
    if !escaped_strings.is_empty() {
        // Join the escaped strings into a single regex pattern separated by '|', to be used in regex later
        let pattern = escaped_strings.join("|");

        // Compile the regex
        let re = Regex::new(&pattern).unwrap();

        //Split the string based on the regex pattern, constructed above
        let split_strings: Vec<&str> = re.split(&message_part).collect();

        //Iter over the split strings
        for split_string in split_strings {
            //Fetch the starting index in the original string for ordering the matches later
            let start_idx = message_part.find(split_string).unwrap();

            //Push back the plaintext match to the ```matches``` buffer
            matches.push(RegexMatch {
                start_idx,
                end_idx: start_idx + split_string.len(),
                regex_type: MessageDisplayDiscriminants::Text,
                capture: split_string.to_string(),
                header_level,
            });
        }
    }
    //If there were no MessageDisplay enums aka there werent any emojis links etc, we can just push back the whole string as a whole text message
    else {
        matches.push(RegexMatch {
            start_idx: 0,
            end_idx: message_part.len(),
            regex_type: MessageDisplayDiscriminants::Text,
            capture: message_part.to_string(),
            header_level,
        });
    }
}

/// The discriminants of this enum are used to diffrenciate the types of regex captures
#[derive(EnumDiscriminants, PartialEq)]
/// These are the types of messages which can be displayed within a normal message
pub enum MessageDisplay {
    /// This is used to display normal plain text
    Text(String),
    /// This is used to display emojies, and holds the important info for displaying an emoji
    Emoji(EmojiDisplay),
    /// This is used to display hyperlinks, and holds the important info for displaying a hyperlink
    Link(HyperLink),

    /// This signals that the following ```MessageDisplay``` enums should be in another line
    NewLine,
}

/// This struct serves as the information struct for the MessageDisplay's Emoji enum
#[derive(PartialEq)]
pub struct EmojiDisplay {
    /// The name of the emoji wanting to be displayed, we make sure to load in all the emojies into the egui buffer when theyre displayed
    pub name: String,
}

/// This struct serves as the information struct for the MessageDisplay's Link enum
#[derive(PartialEq)]
pub struct HyperLink {
    /// This is the part of the hyperlink which gets to be displayed
    pub label: String,
    /// This is the part of the hyperlink which it redirects to
    pub destination: String,
}

///This struct contains all the reactions of one message
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct MessageReaction {
    /// The list of reactions added to a message
    pub message_reactions: Vec<Reaction>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Reaction {
    /// The reaction's corresponding emoji name
    pub emoji_name: String,
    /// The coutner of how many times this emoji has been sent
    pub times: i64,
}
