use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs::{self, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use normpath::BasePathBuf;
use serde_json::{Value, json};

use crate::i18n::autogen_cache::{load_autogen, update_autogen_cache};

pub fn write_locale_file(
    locale_dir: &BasePathBuf,
    data: &BTreeMap<String, String>,
    source_locale: &str,
    target_locale: &str,
) -> Result<(), String> {
    let locale_path = locale_dir.as_path();

    let item_path_res = get_source_file_path(locale_path, source_locale);

    if let Some(item_path) = item_path_res {
        let ext = item_path
            .extension()
            .unwrap_or(OsStr::new("json"))
            .to_str()
            .unwrap_or("json");

        let new_map = dot_to_json(data);
        let file_name = format!("{target_locale}.{ext}");
        let file_path = locale_path.join(file_name);

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(file_path)
            .unwrap();
        let mut writer = BufWriter::new(file);

        match ext {
            "yml" | "yaml" => serde_yaml::to_writer(writer, &new_map).map_err(|e| e.to_string())?,
            "toml" => writer
                .write_all(
                    toml::to_string_pretty(&new_map)
                        .map_err(|e| e.to_string())?
                        .as_bytes(),
                )
                .map_err(|e| e.to_string())?,

            _ => serde_json::to_writer_pretty(writer, &new_map).map_err(|e| e.to_string())?,
        }

        Ok(())
    } else {
        Err("Source file not found".to_string())
    }
}

fn dot_to_json(map: &BTreeMap<String, String>) -> Value {
    let mut root = json!({});

    for (key, value) in map {
        let parts: Vec<&str> = key.split('.').collect();
        let mut current = &mut root;

        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                current[part] = json!(value);
            } else {
                if current.get(part).is_none() {
                    current[part] = json!({});
                }
                current = &mut current[part];
            }
        }
    }

    root
}

pub fn get_source_file_path(locale_path: &Path, source_locale: &str) -> Option<PathBuf> {
    let directory = fs::read_dir(locale_path).ok()?;

    let mut path_buf = None;

    for item_res in directory {
        if let Ok(item) = item_res {
            let item_path = item.path();
            if item_path.is_file()
                && item_path
                    .file_stem()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or_default()
                    == source_locale
            {
                path_buf = Some(item_path);
                break;
            } else {
                path_buf = None;
            }
        } else {
            path_buf = None;
        }
    }

    path_buf
}

///check things list to re-translate data if
/// - whether all the target languages specified exist
/// eg: if all the languages specified already exist in the locale dir then no retranslate
/// - whether there is addition or subtraction of languages
/// eg: if new language is added then re-translate
///
///
/// If verification fails retranslate
pub fn verify_locales(
    locale_path: &Path,
    source_locale: &str,
    target_locales: &Vec<&str>,
) -> Result<(), &'static str> {
    let source_locale_path_res = get_source_file_path(locale_path, source_locale);

    if let Some(source_locale_path) = source_locale_path_res {
        let ext = source_locale_path
            .extension()
            .unwrap_or(OsStr::new("json"))
            .to_str()
            .unwrap_or("json");

        let target_locales_with_ext: Vec<String> = target_locales
            .iter()
            .map(|t| format!("{t}.{ext}"))
            .collect();

        let read_dir = fs::read_dir(locale_path).map_err(|_| "Read Dir Error")?;

        let mut file_names_dir = Vec::new();

        let source_filename = source_locale_path
            .file_name()
            .unwrap_or_default()
            .display()
            .to_string();

        for entry in read_dir {
            if let Ok(dir) = entry {
                //Check if the files in directory are in target locales
                //if not in target locales delete them
                let dir_file_name = dir.file_name().display().to_string();
                if !target_locales_with_ext.contains(&dir_file_name)
                    && dir_file_name != source_filename
                {
                    let _ = fs::remove_file(dir.path());

                    //remove locale data from autogen also
                    let mut autogen = load_autogen();
                    let file_stem = dir
                        .path()
                        .file_stem()
                        .unwrap_or_default()
                        .display()
                        .to_string();
                    autogen.data.remove(&file_stem);
                    let _ = update_autogen_cache(&autogen);
                } else {
                    file_names_dir.push(dir_file_name);
                }
            }
        }

        let mut should_retranslate = false;

        //Now check if new locales are specified that do not exist in the directory
        for target_locale_ext in target_locales_with_ext {
            if !file_names_dir.contains(&target_locale_ext) {
                should_retranslate = true;
            }
        }

        if should_retranslate {
            Err("Files mismatch..retranslate")
        } else {
            Ok(())
        }
    } else {
        return Err("No source file path");
    }
}

#[test]
fn test_locale_file() {
    use normpath::PathExt;
    let mut data = BTreeMap::new();
    data.insert("hello.me".to_string(), "Bonjour Me".to_string());
    data.insert("hello.world".to_string(), "Monde".to_string());

    let locales = Path::new("./locales");
    fs::create_dir_all(&locales).unwrap();
    fs::File::create(locales.join("en.json")).unwrap();
    let locale_dir = &locales.normalize().unwrap();

    assert_eq!(write_locale_file(&locale_dir, &data, "en", "fr"), Ok(()));

    fs::remove_dir_all(&locales).unwrap();
}
