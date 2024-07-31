---
description: This page is servers as an introduction to the Matthias scripting API's usage.
---

# 👶 Getting started

### Language

Matthias currently only supports LUA as a supported extension language. This means that you will be able to write "extensions" or scripts for Matthias in this language.

### Start developing&#x20;

Firstly, you will most likely need a code editor the most popular code editor right now is [Visual Studio Code](https://code.visualstudio.com/) (Which I will be referring to as vscode), which you can download right now for free. There are several extensions for vscode which you can download from vscode's own marketplace specified for LUA code, most of these will help you develop LUA code.

Second of all, you will most likely need a testing environment in which you will be able to test out your own code. You can find tutorials on the internet on how to set up one, and for obvious reasons I will not go into detail about that here. You could also use Matthias's built in code editor, however it is only there to make small changes in your extension.

Last but not least, the most interesting part running your actual creation in Matthias. After creating your masterpiece go to the `%appdata%\matthias\extensions` directory and drop your script file there (It is sometimes easier to develop code right there). After you have logged into Matthias go to settings, open the extensions tab. If you cant see your extension listed, press `Refresh` and run your extension.
