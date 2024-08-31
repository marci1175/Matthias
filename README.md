# Matthias

A (soon to be) multiplatform self-hosted chat application built completely in Rust.

## Features

| Features                                                                                                                            | Desktop | Mobile |
| ----------------------------------------------------------------------------------------------------------------------------------- | ------- | ------ |
| Encrypted messages ensuring security                                                                                                | ✅      | ✅     |
| Backend which doesn't rely on any central provider                                                                                  | ✅      | ✅     |
| Customizable profiles                                                                                                               | ✅      | ✅     |
| Text, audio, image, file messages, and images                                                                                       | ✅      | ✅     |
| Custom emojis                                                                                                                       | ✅      | ✅     |
| Intuitive user interface                                                                                                            | ✅      | ✅     |
| Experimental MD (Markdown) support                                                                                                  | ✅      | ✅     |
| Voice calls                                                                                                                         | ✅      | ❌     |
| Extensive lua (using luajit) API with documentation at [Gitbook](https://matthias.gitbook.io/) with external libs available         | ✅      | ❌     |
| Custom connection urls (If the app is installed through the installer) This allows the user to connect to a server with just a link | ✅      | ❌     |

**Disclaimer: The application has never been security audited, and has known flaws.**

### Additional Features (For desktop only):

- Windows installer (Using a Visual Studio project)

# Children repositories (Crates/Repos created for the purpose of showcasing/improving Matthias)

- [Wincam](https://github.com/marci1175/wincam) (Used to capture images from the host's camera)
- [Protocol Showcase](https://github.com/marci1175/matthias-tokio-protocol/tree/master) (Used to showcase Matthias's TCP protocol)
- [mLua proc macro](https://github.com/marci1175/mlua_proc_macro) (Used in creating the lua API)

---

**All this** with great performance, due to the project being multi-threaded, using async calls with egui, and many more!
I have also tried to make my codebase futureproof by implementing custom automatizations (Example: code generating for emojis) and custom proc macros.

---

## How to compile:

- First, you must have the Rust compiler installed on your computer with all of its dependencies.
- The next step is to download the source code of this project. (whether that be git cloning or downloading it from github's website)
- Navigate to the source folder and run `cargo r --release` (Or without --release for debugging)
- Please note that some features may not be available when running the application after compilation (For links to work you must "install" the application through the installer provided)

### How to create an installer (Note: You must have the Visual Studio installed for this):

- Navigate to `desktop/Installer` in the project folder, and open up the Matthias.sln file.
- Click on build on the top menu bar and click Build Solution (Or use the `ctrl+shift+b` key combination)
- After building go to `desktop/Installer/MatthiasSetup/Release/` and you will find two files:
  - One containing the dependencies (Smaller file size)
  - One containing the application itself (Bigger file size)

## Community

Feel free to chat in the official [Matthias discord server](https://discord.gg/66KFkByMGa)!

## Preview

### Lua API

![lua api](https://github.com/marci1175/Matthias/blob/813d91dec618beca08e85f9c09e7acb1d977c03d/.github/assets/luaapi.png)

### Messages

![Messages](https://github.com/marci1175/Matthias/blob/813d91dec618beca08e85f9c09e7acb1d977c03d/.github/assets/messages.png)

### Register page

![Register page](https://github.com/marci1175/Matthias/blob/813d91dec618beca08e85f9c09e7acb1d977c03d/.github/assets/register.png)

**When wanting to install both, start by opening up the smaller file (Dependency installer), it will automatically open up the application installer.**

**Github actions**:
- Typos: [![typos](https://github.com/marci1175/Matthias/actions/workflows/typos.yml/badge.svg)](https://github.com/marci1175/Matthias/actions/workflows/typos.yml)

# Legacy

- The predecessor of Matthias was [ChatApp](https://github.com/marci1175/ChatApp)
