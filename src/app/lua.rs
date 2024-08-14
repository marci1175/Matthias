use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use egui::{Rect, Vec2};
use mlua::{Lua, Value::Nil};
use strum::Display;

/// This function executes code provided, and if its a function adds it to the global register
pub fn execute_code(lua: &Lua, code: String) -> anyhow::Result<()>
{
    //Execute code, load into the global register if fn
    lua.load(code).exec()?;

    Ok(())
}

/// This function loads code into the global scope, but doesnt execute it
pub fn load_code(lua: &Lua, code: String) -> anyhow::Result<()>
{
    lua.load(code).exec()?;

    Ok(())
}

/// This function calls the function from the global scope
/// Functions must be loaded (via ```load_code(. . .)```), in order to be able to call them
/// This function calls the function specified in the fn_name arg, an optional arg can be provided to the called function
pub fn call_function(lua: &Lua, arg: Option<String>, fn_name: String) -> anyhow::Result<()>
{
    let function = lua.globals().get::<_, mlua::Function<'_>>(fn_name)?;

    //Match args
    match arg {
        Some(arg) => {
            //Call the function with an arg
            function.call::<_, ()>(arg)?;
        },
        None => {
            //Call function with no args
            function.call::<_, ()>(())?;
        },
    }

    Ok(())
}

/// This struct holds all the information of an extension
#[derive(Clone, Default, Debug, serde::Deserialize, serde::Serialize)]
pub struct ExtensionProperties
{
    /// The contents of said extension (This is plain text as its a .lua script)
    pub contents: String,

    /// The name of the extension
    pub name: String,

    /// If the extension is running
    pub is_running: bool,

    /// The path to this extension
    pub path_to_extension: PathBuf,

    /// The buffer of the texteditor to this extension
    pub text_edit_buffer: String,
}

impl ExtensionProperties
{
    /// Create a new instance of an extension
    pub fn new(contents: String, path: PathBuf, name: String) -> Self
    {
        Self {
            text_edit_buffer: contents.clone(),
            contents,
            name,
            path_to_extension: path,
            ..Default::default()
        }
    }

    /// Write changes to the file
    /// This writes the ```self.text_edit_buffer``` to the file itself
    pub fn write_change_to_file(&mut self) -> anyhow::Result<()>
    {
        fs::write(
            self.path_to_extension.clone(),
            self.text_edit_buffer.clone(),
        )?;

        self.contents = self.text_edit_buffer.clone();

        Ok(())
    }
}

/// This enum contains all the types of lua outputs
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum LuaOutput
{
    /// This enum type is used to report code panics (In the lua runtime)
    Error(String),

    /// Standard output from the lua runtime
    Standard(String),

    /// Displays useful information like a file got modifed (This message will only be added from the rust runtime, for example when saving a file)
    Info(String),
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Extension
{
    /// This list shows all the Extensions read from the appdata folder, this list only gets refreshed if the user wants it
    pub extension_list: Vec<ExtensionProperties>,

    #[serde(skip)]
    /// This list contins all the output from the extensions, panics are logged and stdouts are also logged here as Standard()
    pub output: Arc<Mutex<Vec<LuaOutput>>>,

    pub output_rect: Rect,

    pub extension_table_size: Vec2,
}

#[derive(Display)]
/// These are the events which trigger a function call in the extensions.
/// Please refer to the [Documentation](https://matthias.gitbook.io/matthias)
pub enum EventCall
{
    /// Triggered when sending a message
    /// If this Event is invoked the function will recive what the user has sent, this is optional to "recive"
    /// ``` lua
    /// function OnChatSend(message)
    ///     --Do anything with the message
    /// end
    /// function OnChatSend()
    ///     --The function will still be called
    /// end
    /// ```
    OnChatSend,

    /// Triggered when reciving a message
    OnChatRecive,

    /// Triggered when reciving a message from the server
    /// This is unused and will be reused for something else in the future
    #[allow(dead_code)]
    OnServerChatReceive,

    /// Triggered when sending a group voice call request
    OnCallSend,

    /// Triggered when reciving a group voice call,
    OnCallReceive,

    /// Triggered every draw of the ui
    OnDraw,

    /// Triggered when connecting to a server
    OnConnect,

    /// Triggered when disconnecting from a server
    OnDisconnect,
}

impl Extension
{
    pub fn event_call_extensions(&mut self, event: EventCall, lua: &Lua, arg: Option<String>)
    {
        for ext in self.extension_list.iter_mut() {
            //If the extension should be running we skip that entry
            if !ext.is_running {
                continue;
            }

            match Self::load_and_call_function(lua, ext, &arg, event.to_string()) {
                Ok(_) => (),
                Err(err) => {
                    //If the lua returned this error it means the callback couldnt be called
                    //This should be ignored, thats why we return
                    if err.to_string() == "error converting Lua nil to function" {
                        let _ = execute_code(lua, ext.contents.clone());
                        return;
                    }

                    Self::add_msg_to_log(self.output.clone(), ext, err.to_string());
                },
            };
        }
    }

    /// This function loads and calls the function
    /// This function also sets the loaded function to a ```Nil``` to reset it and avoid the function being called from a different script
    fn load_and_call_function(
        lua: &Lua,
        ext: &mut ExtensionProperties,
        arg: &Option<String>,
        fn_name: String,
    ) -> Result<(), anyhow::Error>
    {
        load_code(lua, ext.contents.clone())?;
        call_function(lua, arg.clone(), fn_name.clone())?;

        //Set the function called here to a Nil, to avoid cross script exploiting
        lua.globals().set(fn_name, Nil)?;

        Ok(())
    }

    pub fn add_msg_to_log(
        output: Arc<Mutex<Vec<LuaOutput>>>,
        extension: &mut ExtensionProperties,
        log_inner: String,
    )
    {
        match output.lock() {
            Ok(mut output) => {
                output.push(crate::app::lua::LuaOutput::Error(log_inner.to_string()));
                //Stop the execution of this script
                extension.is_running = false;
                output.push(crate::app::lua::LuaOutput::Info(format!(
                    r#"Extension "{}" was forcibly stopped due to a runtime error."#,
                    extension.name
                )));
            },
            Err(err) => {
                tracing::error!("{}", err);
            },
        }
    }
}

impl Default for Extension
{
    fn default() -> Self
    {
        Self {
            extension_list: Vec::new(),
            output: Arc::new(Mutex::new(Vec::new())),
            output_rect: Rect::NOTHING,
            extension_table_size: Vec2::default(),
        }
    }
}
