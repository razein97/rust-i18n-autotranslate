const DEEPL_LANG_CODES: [&str; 121] = [
    "ACE", "AF", "AN", "AR", "AS", "AY", "AZ", "BA", "BE", "BG", "BHO", "BN", "BR", "BS", "CA",
    "CEB", "CKB", "CS", "CY", "DA", "DE", "EL", "EN", "EN-GB", "EN-US", "EO", "ES", "ES-419", "ET",
    "EU", "FA", "FI", "FR", "GA", "GL", "GN", "GOM", "GU", "HA", "HE", "HI", "HR", "HT", "HU",
    "HY", "ID", "IG", "IS", "IT", "JA", "JV", "KA", "KK", "KMR", "KO", "KY", "LA", "LB", "LMO",
    "LN", "LT", "LV", "MAI", "MG", "MI", "MK", "ML", "MN", "MR", "MS", "MT", "MY", "NB", "NE",
    "NL", "OC", "OM", "PA", "PAG", "PAM", "PL", "PRS", "PS", "PT", "PT-BR", "PT-PT", "QU", "RO",
    "RU", "SA", "SCN", "SK", "SL", "SQ", "SR", "ST", "SU", "SV", "SW", "TA", "TE", "TG", "TH",
    "TK", "TL", "TN", "TR", "TS", "TT", "UK", "UR", "UZ", "VI", "WO", "XH", "YI", "YUE", "ZH",
    "ZH-HANS", "ZH-HANT", "ZU",
];

/// All language codes supported by Google Cloud Translate NMT
const GOOGLE_TRANSLATE_LANG_CODES: [&str; 197] = [
    "ab", "ace", "ach", "af", "sq", "alz", "am", "ar", "hy", "as", "awa", "ay", "az", "ban", "bm",
    "ba", "eu", "btx", "bts", "bbc", "be", "bem", "bn", "bew", "bho", "bik", "bs", "br", "bg",
    "bua", "yue", "ca", "ceb", "ny", "zh-CN", "zh", "zh-TW", "cv", "co", "crh", "hr", "cs", "da",
    "din", "dv", "doi", "dov", "nl", "dz", "en", "eo", "et", "ee", "fj", "fil", "tl", "fi", "fr",
    "fr-FR", "fr-CA", "fy", "ff", "gaa", "gl", "lg", "ka", "de", "el", "gn", "gu", "ht", "cnh",
    "ha", "haw", "iw", "he", "hil", "hi", "hmn", "hu", "hrx", "is", "ig", "ilo", "id", "ga", "it",
    "ja", "jw", "jv", "kn", "pam", "kk", "km", "cgg", "rw", "ktu", "gom", "ko", "kri", "ku", "ckb",
    "ky", "lo", "ltg", "la", "lv", "lij", "li", "ln", "lt", "lmo", "luo", "lb", "mk", "mai", "mak",
    "mg", "ms", "ms-Arab", "ml", "mt", "mi", "mr", "chm", "mni-Mtei", "min", "lus", "mn", "my",
    "nr", "new", "ne", "nso", "no", "nus", "oc", "or", "om", "pag", "pap", "ps", "fa", "pl", "pt",
    "pt-PT", "pt-BR", "pa", "pa-Arab", "qu", "rom", "ro", "rn", "ru", "sm", "sg", "sa", "gd", "sr",
    "st", "crs", "shn", "sn", "scn", "szl", "sd", "si", "sk", "sl", "so", "su", "sw", "ss", "sv",
    "tg", "ta", "tt", "te", "tet", "th", "ti", "ts", "tn", "tr", "tk", "ak", "uk", "ur", "ug",
    "uz", "vi", "cy", "xh", "yi", "yo", "yua", "zu",
];

/// All language codes supported by Libretranslate
const LIBRE_TRANSLATE_LANG_CODES: [&str; 49] = [
    "en", "sq", "ar", "az", "eu", "bn", "bg", "ca", "zh-Hans", "zh-Hant", "cs", "da", "nl", "eo",
    "et", "fi", "fr", "gl", "de", "el", "he", "hi", "hu", "id", "ga", "it", "ja", "ko", "ky", "lv",
    "lt", "ms", "nb", "fa", "pl", "pt", "pt-BR", "ro", "ru", "sk", "sl", "es", "sv", "tl", "th",
    "tr", "uk", "ur", "vi",
];

use thiserror::Error;

use crate::config::TranslationProvider;

#[derive(Error, Debug)]
pub enum LanguageNormalizeError<T: Into<String>> {
    #[error("the language `{0}` is not supported")]
    Redaction(T),
}

pub fn normalize_lang(
    provider: &TranslationProvider,
    lang_code: &str,
) -> Result<String, LanguageNormalizeError<String>> {
    match provider {
        TranslationProvider::GOOGLE => normalize(&lang_code, &GOOGLE_TRANSLATE_LANG_CODES),
        TranslationProvider::DEEPL => {
            let lang_code_uppercase = lang_code.to_uppercase();
            normalize(&lang_code_uppercase, &DEEPL_LANG_CODES)
        }
        TranslationProvider::LIBRETRANSLATE => normalize(&lang_code, &LIBRE_TRANSLATE_LANG_CODES),
    }
}

fn normalize(locale: &str, codes: &[&str]) -> Result<String, LanguageNormalizeError<String>> {
    let contains = codes.contains(&locale);
    if contains {
        Ok(locale.to_string())
    } else {
        //split the incoming lang code get via split char
        //eg: zh-TW -> [zh, TW] -> search using 'zh'
        let split_source_lang: Vec<&str> = locale.split("-").collect();

        let first_source = split_source_lang.first();

        if let Some(first) = first_source {
            let find_code = codes.iter().position(|x| x.contains(first));

            if let Some(found_code) = find_code {
                let item = codes[found_code];
                Ok(item.to_string())
            } else {
                Err(LanguageNormalizeError::Redaction(locale.to_string()))
            }
        } else {
            Err(LanguageNormalizeError::Redaction(locale.to_string()))
        }
    }
}
