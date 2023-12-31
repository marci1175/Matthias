use std::fs;
use chrono::Utc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icons/main.ico");
        res.compile().unwrap();
    }
    tonic_build::compile_protos("proto/messages.proto")?;
    
    //This will always make build_info.szechat_build update, regardless if it has been compiled (because of cargo test)
    let date = Utc::now().format("%Y.%m.%d. %H:%M");
    if let Err(err) = fs::write("build_info.szechat_build", date.to_string()){
        println!("{err}")
    };

    Ok(())
}
