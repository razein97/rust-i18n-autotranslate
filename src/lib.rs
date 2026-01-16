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
//! - Tracks the source language file and only translates when it has changed.
//! - Set `cache = true` to reuse already translated words.
//! - Normalizes languages to a supported language if supported.
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
//!  - DeepL (Cloud Translate - Fallback to deeplx)
//!  - DeepLX (Needs installation [Install DeepLX](<https://deeplx.owo.network/install/>))
//!  - LibreTranslate (Fallback - [Install Self Hosted](<https://docs.libretranslate.com/#self-hosted>)))
//!  - Yandex (Planned)
//!  - aws ML (Planned)
//!
//!
//! # Usage
//!
//! The crate uses env variables to set the api keys.
//!
//! Create a `.env` file in the root of your project and add the following key.
//!
//! The crate uses env variables to set the api key:
//!
//!- **GOOGLE_API_KEY = "xyz"** [How to generate google api key](<https://translatepress.com/docs/automatic-translation/generate-google-api-key/>)
//!- **DEEPL_FREE_API_KEY = "xyz"**
//!- **DEEPL_PRO_API_KEY = "xyz"**
//!- **LIBRE_TRANSLATE_API_KEY = "xyz"**
//!
//!
//! ## Language codes need to be in [ISO-639](<https://wikipedia.org/wiki/ISO_639>) format
//!
//! Call the translate function directly to translate your locales
//!
//! ```rust,no_run
//!use rust_i18n_autotranslate::{
//!    TranslationAPI,
//!    config::{Config, TranslationProvider},
//!};
//!
//!fn main() {
//!    env_logger::init();
//!
//!    let cfg = Config::new()
//!        .locales_directory("./locales")
//!        .source_lang("en")
//!        .add_target_lang("fr")
//!        .use_cache(true)
//!        .translation_provider(TranslationProvider::GOOGLE)
//!        .build();
//!
//!    TranslationAPI::translate(cfg).unwrap()
//!}
//! ```
//!
//!

use log::{error, info};
use rust_i18n_support::load_locales;

use std::collections::BTreeMap;

use crate::{
    api::translate_data,
    config::Config,
    i18n::autogen_cache::Autogen,
    utils::{match_sha256, verify_locales, write_locale_file},
};

mod api;
pub mod config;
mod i18n;
mod utils;

//TODO:: Setup errors correctly

/// The translation api
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TranslationAPI {}

impl TranslationAPI {
    /// Translate the source locale into multiple locales.
    ///
    /// Default output is json
    ///
    /// Choose a translation api.
    ///
    /// To use the paid api set any of the environment variable and select the appropriate provider
    ///
    /// _Environment Variables:_
    ///- GOOGLE_API_KEY="xxx"
    ///- DEEPL_FREE_API_KEY="xxx"
    ///- DEEPL_PRO_API_KEY="xxx"
    ///- LIBRE_TRANSLATE_API_KEY="xxx"
    ///`If both deepl api keys are set, priority is given to the free key`
    ///
    /// Cache: Use cache to save and reuse translations.
    ///

    /// Example:
    /// ```rust,no_run
    ///use rust_i18n_autotranslate::{
    ///    TranslationAPI,
    ///    config::{Config, TranslationProvider},
    ///};
    ///
    ///fn main() {
    ///    env_logger::init();
    ///
    ///    let cfg = Config::new()
    ///        .locales_directory("./locales")
    ///        .source_lang("en")
    ///        .add_target_lang("fr")
    ///        .use_cache(true)
    ///        .translation_provider(TranslationProvider::GOOGLE)
    ///        .build();
    ///
    ///    TranslationAPI::translate(cfg).unwrap()
    ///}
    /// ```
    /// ## Language codes need to be in [ISO-639](<https://wikipedia.org/wiki/ISO_639>) format
    pub fn translate(config: Config) -> Result<(), String> {
        //verify that the sha256 checksums are different then only proceed
        let locale_path = config.locales_dir.clone();

        let verify_locales = verify_locales(
            locale_path.as_path(),
            &config.source_locale,
            &config.target_locales,
        );

        let mut autogen = Autogen::load();

        if config.target_locales.is_empty() {
            info!("Already on latest");
            autogen.data.clear();
            let _ = autogen.update_cache();
            return Ok(());
        }

        let checksum_res = match_sha256(
            locale_path.as_path(),
            &config.source_locale,
            &autogen.checksum.unwrap_or_default(),
        );

        if checksum_res.is_some() || verify_locales.is_err() {
            //update the sha2
            autogen.checksum = checksum_res;

            //Preload google api key from env
            dotenvy::dotenv().ok();

            let mut locales_data =
                load_locales(config.locales_dir.to_str().unwrap_or_default(), |_| false);

            let source_locale_data = locales_data.get_mut(&config.source_locale);

            //use the source locale data
            if let Some(source_data) = source_locale_data {
                source_data.remove("_version");

                if config.use_cache {
                    //use autogen cache
                    for target_locale in config.target_locales {
                        let autogen_data = autogen
                            .data
                            .get(&target_locale)
                            .cloned()
                            .unwrap_or_default();

                        let mut to_translate_keys = Vec::with_capacity(source_data.len());
                        let mut to_translate_values = Vec::with_capacity(source_data.len());
                        let mut og_keys = Vec::with_capacity(source_data.len());

                        for (key, value) in source_data.iter() {
                            //TODO: Find a more performant solution to clones and duplications
                            //maintain a seperate copy iter later
                            og_keys.push(key.as_str());
                            //if it doesnt exist in the autogen cache then send for translate
                            if autogen_data.get(value).is_none() {
                                to_translate_keys.push(key.as_str());
                                to_translate_values.push(value.as_str());
                            }
                        }

                        let translated_values = translate_data(
                            &config.provider,
                            &to_translate_values,
                            &config.source_locale,
                            &target_locale,
                        )?;

                        //get the already present data
                        let mut autogen_locale = autogen
                            .data
                            .get(&target_locale)
                            .cloned()
                            .unwrap_or_default();

                        //combine the translated values
                        let mut translated_kv = BTreeMap::new();

                        if translated_values.len() == to_translate_keys.len() {
                            if translated_values.len() > 0 && to_translate_keys.len() > 0 {
                                //Updating the autogen values
                                for (index, value) in to_translate_values.iter().enumerate() {
                                    autogen_locale.insert(
                                        value.to_string(),
                                        translated_values[index].clone(),
                                    );
                                }
                                //update the autogen value
                                autogen
                                    .data
                                    .insert(target_locale.to_string(), autogen_locale.clone());

                                for (og_key, og_value) in source_data.iter() {
                                    //if contains then it was sent for translation else use cached value
                                    if let Some(pos) =
                                        to_translate_keys.iter().position(|x| x == &og_key)
                                    {
                                        //translated value
                                        // use the pos to get value from translated value
                                        let translated_value = translated_values.get(pos);
                                        if let Some(value) = translated_value {
                                            translated_kv
                                                .insert(og_key.to_string(), value.to_string());
                                        } else {
                                            translated_kv
                                                .insert(og_key.to_string(), og_value.to_string());
                                        }
                                    } else {
                                        //cached value
                                        let res = autogen_locale.get(og_value);
                                        if let Some(auto_data) = res {
                                            translated_kv
                                                .insert(og_key.to_string(), auto_data.to_string());
                                        } else {
                                            //default = not found = insert source value
                                            translated_kv
                                                .insert(og_key.to_string(), og_value.to_string());
                                        }
                                    }
                                }
                            } else {
                                //cached value
                                for (og_key, og_value) in source_data.iter() {
                                    let res = autogen_locale.get(og_value);
                                    if let Some(auto_data) = res {
                                        translated_kv
                                            .insert(og_key.to_string(), auto_data.to_string());
                                    } else {
                                        //default = not found = insert source value
                                        translated_kv
                                            .insert(og_key.to_string(), og_value.to_string());
                                    }
                                }
                            }

                            //write the locale file
                            let write_res = write_locale_file(
                                &locale_path,
                                &translated_kv,
                                &config.source_locale,
                                &target_locale,
                            );

                            if let Err(e) = write_res {
                                error!("{e}");
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

                    for target_locale in config.target_locales {
                        let translated = translate_data(
                            &config.provider,
                            &values,
                            &config.source_locale,
                            &target_locale,
                        )?;

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
                                &config.source_locale,
                                &target_locale,
                            );

                            if let Err(e) = write_res {
                                error!("{e}");
                            }
                        } else {
                            //some translations may have failed, so discard the whole translation
                            continue;
                        }
                    }
                }

                //update autogen
                let autogen_update_res = autogen.update_cache();
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
}
