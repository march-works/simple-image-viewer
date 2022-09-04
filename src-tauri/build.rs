fn main() -> Result<(), Box<dyn std::error::Error>> {
    tauri_build::build();
    tonic_build::compile_protos("proto/add_tab.proto")?;
    tonic_build::compile_protos("proto/new_window.proto")?;
    Ok(())
}
