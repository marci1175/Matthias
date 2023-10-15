use aes::cipher::{generic_array::GenericArray, BlockDecrypt, BlockEncrypt, KeyInit};
use aes::Aes256;
use base64::engine::general_purpose;
use base64::Engine;
use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, Read, Write};
use std::str::from_utf8;
use windows_sys::w;
use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONWARNING};

pub fn encrypt(passw: String) -> String {
    let key = GenericArray::from([0u8; 32]);

    let mut block = GenericArray::from([42u8; 16]);

    let cipher = Aes256::new(&key);

    let block_copy = block.clone();

    cipher.encrypt_block(&mut block);
    println!("{:?}", block);
    cipher.decrypt_block(&mut block);
    println!("{:?}", block);

    passw.into()
}

pub fn login(username: String, passw: String) -> (bool, Option<File>) {
    match env::var("USERNAME") {
        Ok(win_usr) => {
            match File::open(format!(
                "C:\\Users\\{}\\AppData\\Roaming\\szeChat\\{}.szch",
                win_usr, username
            )) {
                Ok(ok) => {
                    match ok.try_clone() {
                        Ok(ok_clone) => {
                            let mut reader: io::BufReader<File> = io::BufReader::new(ok_clone);
                            let mut buffer = String::new();

                            // Read the contents of the file into a buffer
                            match reader.read_to_string(&mut buffer) {
                                Ok(_) => {}
                                Err(err) => {
                                    println!("{}", err);
                                }
                            };
                            let decoded =
                                general_purpose::STANDARD.decode(buffer).expect("Brah wat");
                            let decoded_clone = decoded.clone();
                            let decoded: Vec<&str> = match from_utf8(&decoded_clone) {
                                Ok(ok) => ok.lines().collect(),
                                Err(_) => todo!( /* apad */ ),
                            };
                            return (decoded[0] == username && decoded[1] == passw, Some(ok));
                        }
                        Err(_) => {
                            panic!("Failed to clone file reference.");
                        }
                    };
                }
                Err(_) => {
                    return (false, None);
                }
            };
        }
        Err(_) => {
            return (false, None);
        }
    }
}
pub fn register(username: String, passw: String) -> bool {
    match env::var("APPDATA") {
        Ok(app_data) => {
            let _create_dir = fs::create_dir(format!("{}\\szeChat", app_data));

            if std::fs::metadata(format!("{}\\szeChat\\{}.szch", app_data, username)).is_ok() {
                println!("File already exists");
                std::thread::spawn(|| unsafe {
                    MessageBoxW(0, w!("User already exists"), w!("Error"), MB_ICONWARNING);
                });
                return false;
            }

            match File::create(format!("{}\\szeChat\\{}.szch", app_data, username)) {
                Ok(mut file) => {
                    let b64 = general_purpose::STANDARD.encode(format!("{}\n{}", username, passw));

                    println!("{}", &b64);

                    match file.write_all(b64.as_bytes()) {
                        Ok(_) => {
                            println!("File wrote sexsexfully");
                            return true;
                        }
                        Err(_) => {
                            std::thread::spawn(|| unsafe {
                                MessageBoxW(
                                    0,
                                    w!("Failed to create folder"),
                                    w!("Error"),
                                    MB_ICONWARNING,
                                );
                            });
                            return false;
                        }
                    };
                }
                Err(e) => {
                    panic!("{e}")
                }
            };
        }
        Err(_) => {
            println!("Unable to retrieve the username.");
            return false;
        }
    }
}
