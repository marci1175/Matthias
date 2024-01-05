use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key,
};
use anyhow::{ensure, Context, Result};
use base64::engine::general_purpose;
use base64::Engine;
use rfd::FileDialog;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::string::FromUtf8Error;

use argon2::{self, Config, Variant, Version};

use super::backend::{ServerAudioReply, ServerFileReply, ServerImageReply};

pub fn encrypt_aes256(string_to_be_encrypted: String) -> aes_gcm::aead::Result<String> {
    let key: &[u8] = &[42; 32];

    let key = Key::<Aes256Gcm>::from_slice(key);

    let cipher = Aes256Gcm::new(key);
    let nonce = GenericArray::from([69u8; 12]); // funny encryption key hehehe

    let ciphertext = cipher.encrypt(&nonce, string_to_be_encrypted.as_bytes().as_ref())?;
    let ciphertext = hex::encode(ciphertext);

    Ok(ciphertext)
}

#[inline]
pub fn decrypt_aes256(string_to_be_decrypted: String) -> Result<String, FromUtf8Error> {
    let ciphertext = hex::decode(string_to_be_decrypted).unwrap();
    let key: &[u8] = &[42; 32];

    let key = Key::<Aes256Gcm>::from_slice(key);

    let cipher = Aes256Gcm::new(key);
    let nonce = GenericArray::from([69u8; 12]); // funny encryption key hehehe

    let plaintext = cipher.decrypt(&nonce, ciphertext.as_ref()).unwrap();
    String::from_utf8(plaintext)
}

#[inline]
pub fn pass_encrypt(string_to_be_encrypted: String) -> String {
    let password = string_to_be_encrypted.as_bytes();
    let salt = b"c1eaa94ec38ab7aa16e9c41d029256d3e423f01defb0a2760b27117ad513ccd2";
    let config = Config {
        variant: Variant::Argon2i,
        version: Version::Version13,
        mem_cost: 65536,
        time_cost: 12,
        lanes: 5,
        secret: &[],
        ad: &[],
        hash_length: 64,
    };

    argon2::hash_encoded(password, salt, &config).unwrap()
}

#[inline]
pub fn pass_hash_match(to_be_verified: String, file_ln: String) -> bool {
    argon2::verify_encoded(&file_ln, to_be_verified.as_bytes()).unwrap()
}

pub fn login(username: String, passw: String) -> Result<PathBuf> {
    let app_data = env::var("APPDATA")?;

    let path = PathBuf::from(format!("{app_data}\\Matthias\\{username}.szch"));

    let file_contents = fs::read_to_string(&path)?;

    let mut file_lines = file_contents.lines();

    let usr_check = username
        == file_lines
            .next()
            .context("Corrupted Matthias file at username")?;

    let pwd_check = pass_hash_match(
        passw,
        file_lines
            .next()
            .context("Corrupted Matthias file at password")?
            .into(),
    );

    ensure!(usr_check && pwd_check, "Invalid Password");
    Ok(path)
}

pub fn register(username: String, passw: String) -> Result<()> {
    if username.contains(" ") || username.contains("@") || username.contains(" "){
        return Err(anyhow::Error::msg("Cant use special characters in name"));
    }

    let app_data = env::var("APPDATA")?;

    //always atleast try to make the folder
    let _ = fs::create_dir_all(format!("{app_data}\\Matthias"));

    let user_path = PathBuf::from(format!("{app_data}\\Matthias\\{username}.szch"));

    //user check
    if std::fs::metadata(&user_path).is_ok() {
        return Err(anyhow::Error::msg("User already exists"));
    }

    let hex_encrypted = username + "\n" + &pass_encrypt(passw);

    let mut file = fs::File::create(&user_path)?;

    file.write_all(hex_encrypted.as_bytes())?;

    file.flush()?;

    Ok(())
}

pub fn append_to_file(path: PathBuf, write: String) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(false)
        .append(true)
        .open(path)?;

    match encrypt_aes256(write) {
        Ok(write) => {
            file.write_all((format!("\n{write}")).as_bytes())?;

            file.flush()?;

            Ok(())
        }
        Err(_) => Err(anyhow::Error::msg(
            "Error occured when trying to encrypt with sha256",
        )),
    }
}

pub fn decrypt_lines_from_vec(mut file_ln: Vec<String>) -> Result<Vec<String>> {
    //remove pass and user
    file_ln.remove(0);
    file_ln.remove(0);

    let mut output_vec: Vec<String> = Vec::new();

    if !file_ln.is_empty() {
        for item in file_ln {
            //skip invalid lines
            if !item.trim().is_empty() {
                let decrypted_line = decrypt_aes256(item.to_string())?;

                output_vec.push(decrypted_line)
            }
        }
    }

    Ok(output_vec)
}

pub fn read_from_file(path: PathBuf) -> Result<Vec<String>> {
    let file: Vec<String> = fs::read_to_string(path)?
        .lines()
        .map(|f| f.to_string())
        .collect();

    Ok(file)
}

pub fn delete_line_from_file(line_number: usize, path: PathBuf) -> Result<()> {
    //copy everything from orignal convert to vec representing lines delete line rewrite file
    let mut file = std::fs::OpenOptions::new().read(true).open(path.clone())?;

    let mut buf: String = Default::default();
    file.read_to_string(&mut buf)?;

    let final_vec: Vec<&str> = buf.lines().collect();

    let mut final_vec: Vec<String> = final_vec.iter().map(|s| s.to_string()).collect();
    final_vec.remove(line_number);

    file.flush()?;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)?;

    let buf = final_vec.join("\n");

    file.write_all(buf.as_bytes())?;

    file.flush()?;

    Ok(())
}

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

#[inline]
pub fn write_image(file_response: &ServerImageReply, ip: String) -> Result<()> {
    //secondly create the folder labeled with the specified server ip

    let path = format!(
        "{}\\Matthias\\Client\\{}\\Images\\{}",
        env!("APPDATA"),
        general_purpose::URL_SAFE_NO_PAD.encode(ip),
        file_response.index
    );

    fs::write(path, &file_response.bytes)?;

    Ok(())
}

#[inline]
pub fn write_audio(file_response: ServerAudioReply, ip: String) -> Result<()> {
    //secondly create the folder labeled with the specified server ip
    let path = format!(
        "{}\\Matthias\\Client\\{}\\Audios\\{}",
        env!("APPDATA"),
        general_purpose::URL_SAFE_NO_PAD.encode(ip),
        file_response.index
    );

    fs::write(path, file_response.bytes)?;

    Ok(())
}
