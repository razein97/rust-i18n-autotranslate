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
