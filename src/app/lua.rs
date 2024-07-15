use std::{fs, path::PathBuf, sync::{Arc, Mutex}};

use egui::Rect;
use mlua::Lua;

pub fn execute_code(lua: &Lua, code: String) -> anyhow::Result<()> {
    //Execute code
    lua.load(code).exec()?;

    Ok(())
}

#[derive(Clone, Default, Debug, serde::Deserialize, serde::Serialize)]
pub struct ExtensionProperties {
    pub contents: String,

    pub name: String,

    pub is_running: bool,

    pub path_to_extension: PathBuf,

    pub text_edit_buffer: String,
}

impl ExtensionProperties {
    pub fn new(contents: String, path: PathBuf, name: String) -> Self {
        Self {
            text_edit_buffer: contents.clone(),
            contents,
            name,
            path_to_extension: path,
            ..Default::default()
        }
    }

    pub fn write_change_to_file(&mut self) -> anyhow::Result<()> {
        fs::write(
            self.path_to_extension.clone(),
            self.text_edit_buffer.clone(),
        )?;

        self.contents = self.text_edit_buffer.clone();

        Ok(())
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum LuaOutput {
    /// This enum type is used to report code panics (In the lua runtime)
    Error(String),

    /// Standard output from the lua runtime
    Standard(String),

    /// Displays useful information like a file got modifed (This message will only be added from the rust runtime, for example when saving a file)
    Info(String),
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Extension {
    /// This list shows all the Extensions read from the appdata folder, this list only gets refreshed if the user wants it
    pub extension_list: Vec<ExtensionProperties>,

    #[serde(skip)]
    /// This list contins all the output from the extensions, panics are logged and stdouts are also logged here as Standard()
    pub output: Arc<Mutex<Vec<LuaOutput>>>,

    pub output_rect: Rect,
}

/// These are the events which trigger a function call in the extensions
pub enum EventCall {
    /// Triggered when sending a message
    OnChatSend,
    
    /// Triggered when reciving a message
    OnChatRecive,
    
    /// Triggered when reciving a message from the server
    OnServerChatRecive,

    /// Triggered when sending a group voice call request
    OnCallSend,

    /// Triggered when reciving a group voice call,
    OnCallRecive,

    /// Triggered every draw of the ui
    OnDraw,

    /// Triggered when connecting to a server
    OnConnect,
}

impl Extension {
    pub fn event_call_extension(event: EventCall, lua: &Lua) {
        match event {
            EventCall::OnChatSend => {

            },
            EventCall::OnChatRecive => {

            },
            EventCall::OnServerChatRecive => {

            },
            EventCall::OnCallSend => {

            },
            EventCall::OnCallRecive => {

            },
            EventCall::OnDraw => {

            },
            EventCall::OnConnect => {

            },
        }
    }
}

impl Default for Extension {
    fn default() -> Self {
        Self {
            extension_list: Vec::new(),
            output: Arc::new(Mutex::new(Vec::new())),
            output_rect: Rect::NOTHING,
        }
    }
}
