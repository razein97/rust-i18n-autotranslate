#![deny(missing_docs)]
#![deny(missing_debug_implementations)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(test, deny(warnings))]

//! # rust-i18n-autotranslate
//!
//! The `rust-i18n-autotranslate` crate provides a simple function to autogenerate locales at runtime or buildtime
//!
//! This is meant to be a helper crate to [rust-i18n](<https://docs.rs/rust-i18n/latest/rust_i18n/>)
//!
//!## Features
//! - Does not run everytime. Tracks the source language file and only translates when it has changed.
//! - Set `cache = true` to reuse already translated words.
//!
//! The crate supports creating translations only for version_1 type locales
//! eg:
//!
//! ```text
//! ├── Cargo.lock
//! ├── Cargo.toml
//! ├── locales
//! │   ├── zh-CN.yml
//! │   ├── en.yml
//! └── src
//! │   └── main.rs
//! ```
//!
//!
//!
//! # Current support
//!  - Google Translate (Cloud Translate - Fallback to google translate web)
//!  - Deepl (Planned)
//!  - Yandex (Planned)
//!  - aws ML (Planned)
//!
//! # Usage
//! Add dependencies in your cargo.toml
//! ```rust
//! [dependencies]
//! rust-i18n-autotranslate = "0.1.0"
//! ```
//!
//!
//! The crate uses env variables to set the api keys.
//!
//! Create a `.env` file in the root of your project and add the following key.
//!
//! GOOGLE_API_KEY = "xyz"
//!
//! [How to generate google api key](<https://translatepress.com/docs/automatic-translation/generate-google-api-key/>)
//!
//!
//! Call the translate function directly to translate your locales
//!
//! ```rust
//! use rust_i18n_autotranslate::translate;
//!
//! let locale_dir = "./locales";
//! let source_language = "en";
//! let target_languages = ["fr", "ko"]
//! let use_cache = true;
//!
//! translate(locale_dir, source_language, target_languages.to_vec(), use_cache).unwrap();
//! ```
//!
//!

use log::{error, info};
use normpath::PathExt;
use rust_i18n_support::load_locales;

use std::{collections::BTreeMap, path::Path};

use crate::{
    api::google_translate,
    i18n::autogen_cache::{is_match_sha256, load_autogen, update_autogen_cache},
    utils::write_locale_file,
};

mod api;
mod i18n;
mod utils;

/// Translate the source locale into multiple locales
///
/// Default output is json
///
/// Cache: Use cache to save and reuse translations.
///
/// Example:
/// ```rust
/// use rust_i18n_autotranslate::translate;
///
/// let locale_dir = "./locales";
/// let source_language = "en";
/// let target_languages = ["fr", "ko"]
/// let use_cache = true;
///
/// translate(locale_dir, source_language, target_languages.to_vec(), use_cache).unwrap();
/// ```
pub fn translate(
    locale_directory: &str,
    source_locale: &str,
    target_locales: Vec<&str>,
    cache: bool,
) -> Result<(), String> {
    //verify that the sha256 checksums are different then only proceed
    let locale_path = Path::new(locale_directory)
        .normalize()
        .map_err(|e| e.to_string())?;

    let mut autogen = load_autogen();

    let sha256_res = is_match_sha256(
        locale_path.as_path(),
        source_locale,
        &autogen.sha256.unwrap_or_default(),
    );

    if let Some(sha2) = sha256_res {
        //update the sha2
        autogen.sha256 = Some(sha2);

        //Preload google api key from env
        dotenvy::dotenv().ok();

        let mut locales_data = load_locales(&locale_directory, |_| false);
        let source_locale_data = locales_data.get_mut(source_locale);

        //use the source locale data
        if let Some(source_data) = source_locale_data {
            source_data.remove("_version");

            if cache {
                //use autogen cache
                for target_locale in target_locales {
                    let autogen_data = autogen.data.get(target_locale).cloned().unwrap_or_default();

                    let mut keys = Vec::with_capacity(source_data.len());
                    let mut values = Vec::with_capacity(source_data.len());
                    let mut og_keys = Vec::with_capacity(source_data.len());

                    for (key, value) in source_data.iter() {
                        //TODO: Find a more performant solution to clones and duplications
                        //maintain a seperate copy iter later
                        og_keys.push(key.as_str());
                        //if it doesnt exist in the autogen cache then send for translate
                        if autogen_data.get(key).is_none() {
                            keys.push(key.as_str());
                            values.push(value.as_str());
                        }
                    }

                    let translated =
                        google_translate::translate_v2(&values, source_locale, target_locale)?;

                    //combine the translated
                    if translated.len() == keys.len() {
                        //Updating the autogen values
                        //get the already present data
                        let mut autogen_locale =
                            autogen.data.get(target_locale).cloned().unwrap_or_default();

                        for (index, value) in values.iter().enumerate() {
                            autogen_locale.insert(value.to_string(), translated[index].clone());
                        }
                        //update the autogen value
                        autogen
                            .data
                            .insert(target_locale.to_string(), autogen_locale);

                        //combine the translated values
                        let mut translated_kv = BTreeMap::new();
                        for n in 0..source_data.len() {
                            //if equal then it was sent for translation else use cached value
                            if let Some(og_key) = og_keys.get(n).cloned()
                                && keys[n] != og_key
                            {
                                //cached value
                                //add og key first
                                let res = autogen_data.get(og_key);
                                if let Some(auto_data) = res {
                                    translated_kv.insert(og_key.to_string(), auto_data.to_string());
                                }

                                translated_kv.insert(keys[n].to_string(), translated[n].clone());
                            } else {
                                translated_kv.insert(keys[n].to_string(), translated[n].clone());
                            }
                        }

                        //write the locale file
                        let write_res = write_locale_file(
                            &locale_path,
                            &translated_kv,
                            source_locale,
                            target_locale,
                        );

                        if let Err(e) = write_res {
                            eprintln!("{e}");
                        }
                    } else {
                        //some translations may have failed, so discard the whole translation
                        continue;
                    }
                }
            } else {
                //no use autogen
                let mut keys = Vec::with_capacity(source_data.len());
                let mut values = Vec::with_capacity(source_data.len());
                for (key, value) in source_data {
                    keys.push(key.as_str());
                    values.push(value.as_str());
                }

                for target_locale in target_locales {
                    let translated =
                        google_translate::translate_v2(&values, source_locale, target_locale)?;

                    //combine the translated
                    if translated.len() == keys.len() {
                        //combine the translated values
                        let mut translated_kv = BTreeMap::new();
                        for (index, key) in keys.iter().enumerate() {
                            translated_kv.insert(key.to_string(), translated[index].clone());
                        }

                        //write the locale file
                        let write_res = write_locale_file(
                            &locale_path,
                            &translated_kv,
                            source_locale,
                            target_locale,
                        );

                        if let Err(e) = write_res {
                            eprintln!("{e}");
                        }
                    } else {
                        //some translations may have failed, so discard the whole translation
                        continue;
                    }
                }
            }

            //update autogen
            let autogen_update_res = update_autogen_cache(&autogen);
            if let Err(err) = autogen_update_res {
                error!("{}", err);
            }

            Ok(())
        } else {
            Err("Could not find source locale data".to_string())
        }
    } else {
        info!("Already on latest");
        Ok(())
    }
}
