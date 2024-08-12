# Matthias
A decentralized chat application built completely in rust.
## Features
- Encrypted messages ensuring security
- Decentralized backend which doesnt rely on any central provider
- Customizable profiles
- Text, audio, image, file messages furthermore images are also displayed
- Voice call
- Exrtensive lua (using luajit) API with documentation at [Gitbook](https://matthias.gitbook.io/) with external libs available
- Custom connection urls (If the app is installed through the installer) this allows the user to connect to a server with just a link 
- Custom emojies
- Windows installer (Using a Visual Studio project)
- Intuitive user interface
- Experimental MD (Markdown) support

# Legacy
- The predecessor of Matthias was [ChatApp](https://github.com/marci1175/ChatApp)

# Children repositories (Crates/Repos created for the purpose of showcasing/improving Matthias)
- [Wincam](https://github.com/marci1175/wincam) (Used to capture images from the host's camera)
- [Protocol Showcase](https://github.com/marci1175/matthias-tokio-protocol/tree/master) (Used to showcase Matthias's TCP protocol)
- [mLua proc macro](https://github.com/marci1175/mlua_proc_macro) (Used in creating the lua API)

__All this__ with great preformance, due to the project being mulit-threaded, using async calls with egui and many more!
I have also tried to make my codebase futureproof, with implementing custom automatizations (Example: code generating for emojies) and custom proc macros.
## Community
Feel free to chat in the official [Matthias discord server](https://discord.gg/66KFkByMGa)!
## Preview
### Lua api
![lua api](https://github.com/marci1175/Matthias/blob/813d91dec618beca08e85f9c09e7acb1d977c03d/.github/assets/luaapi.png)
### Messages
![Messages](https://github.com/marci1175/Matthias/blob/813d91dec618beca08e85f9c09e7acb1d977c03d/.github/assets/messages.png)
### Register page
![Register page](https://github.com/marci1175/Matthias/blob/813d91dec618beca08e85f9c09e7acb1d977c03d/.github/assets/register.png)
