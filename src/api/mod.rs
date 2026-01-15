use crate::TranslationProvider;

mod deepl_translate;
mod google_translate;
mod libre_translate;

///
/// Translates according to the provider selected
pub fn translate_data(
    provider: &TranslationProvider,
    source_data: &Vec<&str>,
    source_lang: &str,
    target_lang: &str,
) -> Result<Vec<String>, String> {
    match provider {
        TranslationProvider::GOOGLE => {
            google_translate::translate_v2(source_data, source_lang, target_lang)
        }
        TranslationProvider::DEEPL => {
            deepl_translate::translate_v2(source_data, source_lang, target_lang)
        }
        TranslationProvider::LIBRETRANSLATE => {
            libre_translate::translate_v1(source_data, source_lang, target_lang)
        }
    }
}
