use base64::engine::general_purpose;
use base64::Engine;
use windows_sys::w;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, MB_ICONWARNING,
};
use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, Read, Write};
use std::str::from_utf8;
use aes::cipher::{
    BlockCipher, BlockEncrypt, BlockDecrypt, KeyInit,
    generic_array::GenericArray,
};
use aes::Aes256;
pub fn encrypt(passw: String) -> String {

    let key = GenericArray::from([0u8; 32]);

    let mut block = GenericArray::from([42u8; 16]);

    let cipher = Aes256::new(&key);

    let block_copy = block.clone();

    cipher.encrypt_block(&mut block);
    println!("{:?}", block);
    cipher.decrypt_block(&mut block);
    println!("{:?}", block);

    let cipher = Aes256::new(&key);
    "fasz".into()
}

pub fn login(username: String, passw: String) -> bool {
    match env::var("USERNAME") {
        Ok(win_usr) => {
            match File::open(format!(
                "C:\\Users\\{}\\AppData\\Roaming\\szeChat\\{}.szch",
                win_usr, username
            )) {
                Ok(ok) => {
                    let mut reader: io::BufReader<File> = io::BufReader::new(ok);
                    let mut buffer = String::new();

                    // Read the contents of the file into a buffer
                    match reader.read_to_string(&mut buffer) {
                        Ok(_) => {}
                        Err(err) => {
                            println!("{}", err);
                        }
                    };
                    let decoded = general_purpose::STANDARD.decode(buffer).expect("Nigga");
                    let decoded: Vec<&str> = match from_utf8(&decoded) {
                        Ok(ok) => ok.lines().collect(),
                        Err(_) => todo!( /* apad */ ),
                    };

                    return decoded[0] == username && decoded[1] == passw;
                }
                Err(_) => {
                    return false;
                }
            };
        }
        Err(_) => {
            return false;
        }
    }
}
pub fn register(username: String, passw: String) -> bool {
    match env::var("USERNAME") {
        Ok(win_usr) => {
            let _create_dir = fs::create_dir(format!(
                "C:\\Users\\{}\\AppData\\Roaming\\szeChat",
                username
            ));

            if std::fs::metadata(format!(
                "C:\\Users\\{}\\AppData\\Roaming\\szeChat\\{}.szch",
                win_usr, username
            )).is_ok() {
                    println!("File already exists");
                    std::thread::spawn( || unsafe {
                        MessageBoxW(0, w!("User already exists"), w!("Error"), MB_ICONWARNING);
                    });
                    return false;
            }

            let mut file = File::create(format!(
                "C:\\Users\\{}\\AppData\\Roaming\\szeChat\\{}.szch",
                win_usr, username
            ))
            .unwrap();

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
        Err(_) => {
            println!("Unable to retrieve the username.");
            return false;
        }
    }
}

