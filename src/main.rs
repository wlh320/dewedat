use colour::red;
use glob::glob;
use std::error::Error;
use std::path::{Path, PathBuf, StripPrefixError};
use tokio;
use tokio::fs;

fn find_xor_key(b1: u8, b2: u8) -> Result<(u8, &'static str), String> {
    match b1 ^ b2 {
        b if b == 0xff ^ 0xd8 => Ok((b1 ^ 0xff, "jpg")),
        b if b == 0x89 ^ 0x50 => Ok((b1 ^ 0x89, "png")),
        b if b == 0x47 ^ 0x49 => Ok((b1 ^ 0x47, "gif")),
        b if b == 0x49 ^ 0x49 => Ok((b1 ^ 0x49, "tiff")),
        b if b == 0x4d ^ 0x4d => Ok((b1 ^ 0x4d, "tiff")),
        b if b == 0x42 ^ 0x4d => Ok((b1 ^ 0x42, "bmp")),
        _ => Err(String::from("Not Implementation")),
    }
}
fn decode(cipher: &[u8], key: u8) -> Vec<u8> {
    cipher.iter().map(|b| b ^ key).collect()
}

fn replace_prefix(p: &Path, from: &Path, to: &Path) -> Result<PathBuf, StripPrefixError> {
    p.strip_prefix(from).map(|p| to.join(p))
}
async fn dewedat(file: &Path, source_dir: &Path, target_dir: &Path) -> Result<(), Box<dyn Error>> {
    // load file
    let cipher = fs::read(&file).await?;
    // decode xor in another thread
    let (send, recv) = tokio::sync::oneshot::channel();
    let (key, ext) = find_xor_key(cipher[0], cipher[1])?;
    rayon::spawn(move || {
        let plain = decode(&cipher, key);
        let _ = send.send(plain);
    });
    let plain = recv.await.unwrap();
    // save file
    let target_file = replace_prefix(&file, &source_dir, &target_dir)?;
    println!("{:?}", target_file);
    if let Some(prefix) = target_file.parent() {
        fs::create_dir_all(prefix).await?;
    }
    let target = target_file.with_extension(ext);
    fs::write(target, plain).await?;
    Ok(())
}
async fn dewedat_dir(source: &str, target: &str) -> Result<(), Box<dyn Error>> {
    if !PathBuf::from(source).is_dir() {
        Err("Invalid Source Directory")?
    }
    if !PathBuf::from(target).is_dir() {
        if let Err(e) = fs::create_dir_all(target).await {
            eprintln!("Error when creating target directory:{}", e);
            Err("Invalid Target Directory")?
        }
    }
    let tasks = glob(&format!("{}/**/*.dat", source))?.map(|entry| async {
        let entry = entry.unwrap();
        match dewedat(&entry, &PathBuf::from(source), &PathBuf::from(target)).await {
            Ok(()) => {
                // green!("Ok ");
                // println!("{}", entry.to_string_lossy());
                Ok(())
            }
            Err(e) => {
                red!("Fail ({}) ", e);
                println!("{}", entry.to_string_lossy());
                Err(())
            }
        }
    });
    let results = futures::future::join_all(tasks).await;
    let succes = results.iter().filter(|r| r.is_ok()).count();
    let total = results.len();
    println!("Finished ({succes}/{total}).");
    Ok(())
}

fn usage() {
    println!("dewedat: decode wechat dat images");
    println!("Usage:");
    println!("./dewedat source_dir target_dir");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 3 {
        usage();
    } else {
        dewedat_dir(&args[1], &args[2]).await?;
    }
    Ok(())
}
