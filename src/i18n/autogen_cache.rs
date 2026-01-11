use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter},
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::utils::get_source_file_path;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Autogen {
    pub sha256: Option<String>,
    pub data: HashMap<String, HashMap<String, String>>,
}

pub fn load_autogen() -> Autogen {
    //Using just create was replacing the file always, hence the additional check
    let auto_translate_file = File::create_new("./.autotranslate_gen.json");

    match auto_translate_file {
        Ok(_) => Autogen::default(),
        Err(_) => {
            let existing_file = File::open("./.autotranslate_gen.json");
            if let Ok(file) = existing_file {
                let reader = BufReader::new(file);
                let parsed = serde_json::from_reader::<_, Autogen>(reader);
                if let Ok(autogen) = parsed {
                    autogen
                } else {
                    Autogen::default()
                }
            } else {
                Autogen::default()
            }
        }
    }
}

pub fn update_autogen_cache(autogen: &Autogen) -> Result<(), String> {
    let auto_translate_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open("./.autotranslate_gen.json")
        .map_err(|e| e.to_string())?;
    let writer = BufWriter::new(auto_translate_file);

    serde_json::to_writer(writer, &autogen).map_err(|e| e.to_string())
}

/// If it does not match then return the new sha256
pub fn is_match_sha256(locale_path: &Path, source_lang: &str, autogen_sha: &str) -> Option<String> {
    let res = get_source_file_path(locale_path, source_lang);
    if let Some(item_path) = res {
        let sha256_res = sha256::try_digest(item_path);
        if let Ok(sha) = sha256_res {
            if autogen_sha != sha { Some(sha) } else { None }
        } else {
            None
        }
    } else {
        None
    }
}
