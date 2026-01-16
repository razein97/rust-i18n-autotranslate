//!
//! _Config builder_
//!
//! Helps build the configuration for the translation api
//!

use normpath::PathExt;
use std::{
    io,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Errors for the Config Builder
#[derive(Error, Debug)]
pub enum DirectoryError {
    #[error("Locales Directory Path is Missing")]
    /// Input path may be malformed
    InvalidInput(#[from] io::Error),
}

/// Providers available for translation
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TranslationProvider {
    ///Google Cloud Translation
    #[default]
    GOOGLE,
    ///DeepL Cloud Translation
    DEEPL,
    ///LibreTranslate Translations
    LIBRETRANSLATE,
}

/// Providers available for translation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    ///Path pointing to where the locales are located
    pub locales_dir: PathBuf,
    ///Source language
    pub source_locale: String,
    ///Languages to translate
    pub target_locales: Vec<String>,
    ///Default: true
    pub use_cache: bool,
    ///Translation provider
    pub provider: TranslationProvider,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            locales_dir: "".into(),
            source_locale: "en".to_string(),
            target_locales: Default::default(),
            use_cache: true,
            provider: Default::default(),
        }
    }
}

impl Config {
    /// Return the defaults for the config
    pub fn new() -> Self {
        Self {
            locales_dir: "".into(),
            source_locale: "en".to_string(),
            target_locales: vec![],
            use_cache: true,
            provider: TranslationProvider::GOOGLE,
        }
    }

    /// Path to directory where the locales are located
    pub fn locales_directory<P: AsRef<Path>>(&mut self, p: P) -> &mut Self {
        let normalized = p
            .as_ref()
            .to_path_buf()
            .normalize()
            .expect("Locales directory path is malformed");
        self.locales_dir = normalized.as_path().to_path_buf();
        self
    }

    /// Language to translate from
    pub fn source_lang<S: Into<String>>(&mut self, lang: S) -> &mut Self {
        self.source_locale = lang.into();
        self
    }

    ///Language to translate to
    pub fn add_target_lang<S: Into<String>>(&mut self, lang: S) -> &mut Self {
        self.target_locales.push(lang.into());
        self
    }

    ///Languages to translate to -- add many
    pub fn add_target_langs<S: Into<String>>(&mut self, langs: Vec<S>) -> &mut Self {
        self.target_locales
            .extend(langs.into_iter().map(|s| s.into()));
        self
    }

    ///Use cache or not
    pub fn use_cache(&mut self, cache: bool) -> &mut Self {
        self.use_cache = cache;
        self
    }

    ///Provider to use
    pub fn translation_provider(&mut self, provider: TranslationProvider) -> &mut Self {
        self.provider = provider;
        self
    }

    /// Build the config
    pub fn build(&self) -> Self {
        Config {
            locales_dir: self.locales_dir.clone(),
            source_locale: self.source_locale.clone(),
            target_locales: self.target_locales.clone(),
            use_cache: self.use_cache,
            provider: self.provider.clone(),
        }
    }
}
