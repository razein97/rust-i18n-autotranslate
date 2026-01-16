# rust-i18n-autotranslate [![Crates.io][crates-badge]][crates-url]

[crates-badge]: https://img.shields.io/crates/v/rust-i18n-autotranslate
[crates-url]: https://crates.io/crates/rust-i18n-autotranslate

Auto translate locales build time and runtime.

## Features

- Tracks the source language file and only translates when it has changed.
- Set `cache = true` to reuse already translated words.
- Normalizes languages to a supported language if supported.

## Current support

- Google Translate (Cloud Translate - Fallback to google translate web)
- DeepL (Cloud Translate - Fallback to deeplx)
- DeepLX (Needs installation [Install DeepLX](https://deeplx.owo.network/install/))
- LibreTranslate (Fallback - [Install Self Hosted](https://docs.libretranslate.com/#self-hosted)))
- Yandex (Planned)
- aws ML (Planned)

## Install

use cargo:

```sh
cargo add rust-i18n-autotranslate
```

Add dependencies in your cargo.toml

```rust
[dependencies]
rust-i18n-autotranslate = "0.1"
```

The crate uses env variables to set the api key:

- **GOOGLE_API_KEY = "xyz"**
- **DEEPL_FREE_API_KEY = "xyz"**
- **DEEPL_PRO_API_KEY = "xyz"**
- **LIBRE_TRANSLATE_API_KEY="xyz"**

[How to generate google api key](https://translatepress.com/docs/automatic-translation/generate-google-api-key/)

Call the translate function directly to translate your locales

```rust
use rust_i18n_autotranslate::{
    TranslationAPI,
    config::{Config, TranslationProvider},
};

fn main() {
    env_logger::init();

    let cfg = Config::new()
        .locales_directory("./locales")
        .source_lang("en")
        .add_target_lang("fr")
        .use_cache(true)
        .translation_provider(TranslationProvider::GOOGLE)
        .build();

    TranslationAPI::translate(cfg).unwrap()
}
```
