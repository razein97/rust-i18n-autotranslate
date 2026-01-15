use rust_i18n_autotranslate::translate;

fn main() {
    env_logger::init();
    let res = translate(
        "./locales",
        "en",
        ["fr"].to_vec(),
        true,
        rust_i18n_autotranslate::TranslationProvider::LIBRETRANSLATE,
    );
    println!("{res:?}")
}
