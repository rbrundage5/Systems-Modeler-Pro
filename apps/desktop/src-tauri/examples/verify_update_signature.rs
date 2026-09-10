//! Release packaging check using the same verifier as tauri-plugin-updater.
use base64::Engine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("Usage: verify_update_signature INSTALLER SIGNATURE".into());
    }
    let decode = |text: &str| -> Result<String, Box<dyn std::error::Error>> {
        Ok(String::from_utf8(
            base64::engine::general_purpose::STANDARD.decode(text.trim())?,
        )?)
    };
    let key = decode(&std::env::var("SMP_UPDATER_PUBLIC_KEY")?)?;
    let signature = decode(&std::fs::read_to_string(&args[2])?)?;
    minisign_verify::PublicKey::decode(&key)?.verify(
        &std::fs::read(&args[1])?,
        &minisign_verify::Signature::decode(&signature)?,
        true,
    )?;
    println!("Update signature verified against the embedded public key.");
    Ok(())
}
