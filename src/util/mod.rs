use common_failures::prelude::*;
use rpassword;
use rprompt;
use std::{
    env, ffi, fs, io,
    io::{Read, Write},
    path::PathBuf,
    process,
};
use tempdir;
use Result;
use {id, proof, repo, util};

pub mod serde;

use app_dirs::{app_root, get_app_root, AppDataType, AppInfo};

const APP_INFO: AppInfo = AppInfo {
    name: "crev",
    author: "Dawid Ciężarkiewicz",
};

pub fn user_config_path() -> Result<PathBuf> {
    Ok(app_root(AppDataType::UserConfig, &APP_INFO)?.join("crev.yaml"))
}

pub fn read_passphrase() -> io::Result<String> {
    if let Ok(pass) = env::var("CREV_PASSPHRASE") {
        eprint!("Using passphrase set in CREV_PASSPHRASE\n");
        return Ok(pass);
    }
    eprint!("Enter passphrase to unlock: ");
    rpassword::read_password()
}

pub fn read_new_passphrase() -> io::Result<String> {
    if let Ok(pass) = env::var("CREV_PASSPHRASE") {
        eprint!("Using passphrase set in CREV_PASSPHRASE\n");
        return Ok(pass);
    }
    loop {
        eprint!("Enter new passphrase: ");
        let p1 = rpassword::read_password()?;
        eprint!("Enter new passphrase again: ");
        let p2 = rpassword::read_password()?;
        if p1 == p2 {
            return Ok(p1);
        }
        eprintln!("\nPassphrases don't match, try again.");
    }
}
fn get_editor_to_use() -> ffi::OsString {
    if let Some(v) = env::var_os("VISUAL") {
        return v;
    } else if let Some(v) = env::var_os("EDITOR") {
        return v;
    } else {
        return "vi".into();
    }
}

fn edit_text_iteractively(text: String) -> Result<String> {
    let editor = get_editor_to_use();
    let dir = tempdir::TempDir::new("crev")?;
    let file_path = dir.path().join("crev.review");
    let mut file = fs::File::create(&file_path)?;
    file.write_all(text.as_bytes())?;
    file.flush()?;
    drop(file);

    let status = process::Command::new(editor).arg(&file_path).status()?;

    if !status.success() {
        bail!("Editor returned {}", status);
    }

    let mut file = fs::File::open(&file_path)?;
    let mut res = String::new();
    file.read_to_string(&mut res)?;

    Ok(res)
}

fn yes_or_no_was_y() -> Result<bool> {
    loop {
        let reply = rprompt::prompt_reply_stderr("Try again (y/n)")?;

        match reply.as_str() {
            "y" | "Y" => return Ok(true),
            "n" | "N" => return Ok(false),
            _ => {}
        }
    }
}

pub fn edit_review_iteractively(review: proof::Review) -> Result<proof::Review> {
    let mut text = review.to_string();
    loop {
        text = edit_text_iteractively(text)?;
        match proof::Review::parse(&text) {
            Err(e) => {
                eprintln!("There was an error parsing review: {}", e);
                if !yes_or_no_was_y()? {
                    bail!("User canceled");
                }
            }
            Ok(review) => return Ok(review),
        }
    }
}
