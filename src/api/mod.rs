use crate::{config::TranslationProvider, utils::languages::normalize_lang};

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
    let normalized_source_lang =
        normalize_lang(provider, source_lang).map_err(|e| e.to_string())?;

    let normalized_target_lang =
        normalize_lang(provider, target_lang).map_err(|e| e.to_string())?;

    match provider {
        TranslationProvider::GOOGLE => google_translate::translate_v2(
            source_data,
            &normalized_source_lang,
            &normalized_target_lang,
        ),
        TranslationProvider::DEEPL => deepl_translate::translate_v2(
            source_data,
            &normalized_source_lang,
            &normalized_target_lang,
        ),
        TranslationProvider::LIBRETRANSLATE => libre_translate::translate_v1(
            source_data,
            &normalized_source_lang,
            &normalized_target_lang,
        ),
    }
}
