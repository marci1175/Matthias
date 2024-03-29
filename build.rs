use chrono::{Duration, Utc};
use std::fs;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icons/main.ico");
        res.compile().unwrap();
    }

    tonic_build::compile_protos("proto/messages.proto")?;

    //This will always make build_info.matthias_build update, regardless if it has been compiled (because of cargo test)
    let date = Utc::now()
        .checked_add_signed(Duration::hours(1))
        .unwrap_or_default()
        .format("%Y.%m.%d. %H:%M");
    if let Err(err) = fs::write("build_info.Matthias_build", date.to_string()) {
        println!("{err}")
    };

    Ok(())
}
