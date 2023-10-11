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

use data_encoding::HEXUPPER;
use ring::error::Unspecified;
use ring::rand::SecureRandom;
use ring::{digest, pbkdf2, rand};
use std::num::NonZeroU32;

pub fn main() -> Result<(), Unspecified> {
    let decryp_key = "Marcell";
    let cred_len: usize = decryp_key.len();
    let n_iter = NonZeroU32::new(100_000).unwrap();
    let rng = rand::SystemRandom::new();

    let mut salt = [0u8; cred_len];
    rng.fill(&mut salt)?;

    let password = "Guess Me If You Can!";
    let mut pbkdf2_hash = [0u8; cred_len];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA512,
        n_iter,
        &salt,
        password.as_bytes(),
        &mut pbkdf2_hash,
    );
    println!("Salt: {}", HEXUPPER.encode(&salt));
    println!("PBKDF2 hash: {}", HEXUPPER.encode(&pbkdf2_hash));

    let should_succeed = pbkdf2::verify(
        pbkdf2::PBKDF2_HMAC_SHA512,
        n_iter,
        &salt,
        password.as_bytes(),
        &pbkdf2_hash,
    );
    let wrong_password = "Definitely not the correct password";
    let should_fail = pbkdf2::verify(
        pbkdf2::PBKDF2_HMAC_SHA512,
        n_iter,
        &salt,
        wrong_password.as_bytes(),
        &pbkdf2_hash,
    );

    assert!(should_succeed.is_ok());
    assert!(!should_fail.is_ok());

    Ok(())
}