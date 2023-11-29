fn main() -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icons/main.ico");
        res.compile().unwrap();
    }
    tonic_build::compile_protos("proto/messages.proto")?;
    Ok(())
}
