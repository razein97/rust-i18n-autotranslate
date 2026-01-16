//api_version_v2

use std::{collections::HashMap, env};

use html_escape::decode_html_entities;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use ureq::http::StatusCode;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TranslatedResponse {
    pub translations: Vec<TranslationResponse>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TranslationResponse {
    pub detected_source_language: String,
    pub text: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TranslationRequestBody {
    pub text: Vec<String>,
    pub target_lang: String,
    pub source_lang: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_billed_characters: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split_sentences: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preserve_formatting: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glossary_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_instructions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_handling: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_handling_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline_detection: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_beta_languages: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_splitting_tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub splitting_tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_tags: Option<Vec<String>>,
}

///Translate using v2 api
///
pub fn translate_v2(
    source_data: &Vec<&str>,
    source_lang: &str,
    target_lang: &str,
) -> Result<Vec<String>, String> {
    let mut translated: Vec<String> = Vec::with_capacity(source_data.len());

    let (api_key, api_url) = get_key_url();

    let mut duplicates = 0;

    if let Some(key) = api_key
        && !key.is_empty()
    {
        let mut mem_cache: HashMap<&str, Vec<usize>> = HashMap::new();

        let chunks: Vec<&[&str]> = source_data.chunks(120).collect();

        for chunk in chunks {
            let mut qry_text: Vec<String> = Vec::new();

            for (idx, q) in chunk.iter().enumerate() {
                //if item in cache then record the position in the chunk array
                // send empty character for translation
                //You will be charged for only one character reducing usage
                if let Some(mem_val) = mem_cache.get_mut(q) {
                    mem_val.push(idx);
                    qry_text.push("".to_string());

                    duplicates += 1;
                } else {
                    mem_cache.insert(*q, vec![idx]);
                    qry_text.push(q.to_string());
                }
            }

            let json_body = TranslationRequestBody {
                text: qry_text,
                target_lang: target_lang.to_string(),
                source_lang: source_lang.to_string(),
                ..Default::default()
            };

            let response = ureq::post(&api_url)
                .config()
                .http_status_as_error(false)
                .build()
                .header("Authorization", &key)
                .content_type("application/json")
                .send_json(json_body);

            match response {
                Ok(mut translated_res) => {
                    match translated_res.status() {
                        StatusCode::OK => {
                            let data_res =
                                translated_res.body_mut().read_json::<TranslatedResponse>();
                            match data_res {
                                Ok(data) => {
                                    let g_translated_data = &data.translations;

                                    for (idx, translation_res) in
                                        data.translations.iter().enumerate()
                                    {
                                        let decoded_str =
                                            decode_html_entities(&translation_res.text);
                                        let decoded = decoded_str.trim();

                                        //replace the empty value with one in pos
                                        if decoded.is_empty() {
                                            for mem_val in mem_cache.values() {
                                                let pos = mem_val.iter().position(|x| x == &idx);

                                                if let Some(pos) = pos {
                                                    //We only want to use not 0 pos as it is the finder of the  translated value
                                                    if pos > 0 {
                                                        let init_pos = mem_val[0];
                                                        let translated_value =
                                                            g_translated_data.get(init_pos);
                                                        if let Some(translation) = translated_value
                                                        {
                                                            let init_pos_decoded =
                                                                decode_html_entities(
                                                                    &translation.text,
                                                                );
                                                            translated
                                                                .push(init_pos_decoded.to_string());
                                                            break;
                                                        } else {
                                                            translated.push(decoded.to_string());
                                                            break;
                                                        }
                                                    } else {
                                                        translated.push(decoded.to_string());
                                                        break;
                                                    }
                                                }
                                            }
                                        } else {
                                            translated.push(decoded.to_string());
                                        }
                                    }
                                }
                                Err(err) => {
                                    return Err(err.to_string());
                                }
                            }
                        }
                        _ => {
                            return Err(translated_res
                                .body_mut()
                                .read_to_string()
                                .unwrap_or_default());
                        }
                    }
                }
                Err(e) => {
                    return Err(e.to_string());
                }
            }

            mem_cache.clear();
        }

        debug!("Duplicates found: {duplicates}");

        Ok(translated)
    } else {
        warn!(
            "DeepL API key not found. Set it using DEEPL_FREE_API_KEY or DEEPL_PRO_API_KEY variable"
        );
        info!("Using deeplx local...");

        let mut mem_cache: HashMap<&str, String> = HashMap::new();

        for romanize in source_data.iter() {
            if let Some(mem_val) = mem_cache.get(romanize) {
                translated.push(mem_val.to_owned());
                duplicates += 1;
            } else {
                //if not in mem cache
                match deeplx_translate(source_lang, target_lang, *romanize) {
                    Ok(result) => {
                        translated.push(result.clone());
                        mem_cache.insert(*romanize, result);
                    }
                    Err(err) => return Err(err),
                }
            }
        }

        debug!("Duplicates: {duplicates}");
        Ok(translated)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct DeepLXTranslationResponse {
    pub detected_source_language: String,
    pub text: String,
}

// TODO: This is not tested... Built by referring to the api online.
fn deeplx_translate(source_lang: &str, target_lang: &str, q: &str) -> Result<String, String> {
    let json = TranslationRequestBody {
        text: [q.to_string()].to_vec(),
        target_lang: target_lang.to_string(),
        source_lang: source_lang.to_string(),
        ..Default::default()
    };

    let web_url = "http://127.0.0.1:1188/translate";
    let res = ureq::post(web_url).send_json(json);

    match res {
        Ok(mut response) => {
            if response.status() == StatusCode::OK {
                let json_res = response.body_mut().read_json::<DeepLXTranslationResponse>();

                match json_res {
                    Ok(translation) => {
                        let decoded = decode_html_entities(&translation.text);

                        Ok(decoded.to_string())
                    }
                    Err(e) => Err(e.to_string()),
                }
            } else {
                Err("Invalid request".to_string())
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

fn get_key_url() -> (Option<String>, String) {
    let free_api_key = env::var("DEEPL_FREE_API_KEY").ok();
    let pro_api_key = env::var("DEEPL_PRO_API_KEY").ok();

    let free_api_url = "https://api-free.deepl.com/v2/translate".to_string();
    let pro_api_url = "https://api.deepl.com/v2/translate".to_string();
    let web_url = "".to_string();

    match (free_api_key, pro_api_key) {
        (None, None) => (None, web_url),
        (None, Some(pro_key)) => {
            if pro_key.is_empty() {
                (None, web_url)
            } else {
                (Some(format!("DeepL-Auth-Key {pro_key}")), pro_api_url)
            }
        }
        (Some(free_key), None) => {
            if free_key.is_empty() {
                (None, web_url)
            } else {
                (Some(format!("DeepL-Auth-Key {free_key}")), free_api_url)
            }
        }
        (Some(free_key), Some(pro_key)) => match (!free_key.is_empty(), !pro_key.is_empty()) {
            (true, true) => (Some(format!("DeepL-Auth-Key {free_key}")), free_api_url),
            (true, false) => (Some(format!("DeepL-Auth-Key {free_key}")), free_api_url),
            (false, true) => (Some(format!("DeepL-Auth-Key {pro_key}")), pro_api_url),
            (false, false) => (None, web_url),
        },
    }
}

#[test]
fn test_translate_v2() {
    let source_values = vec!["hello", "mello", "cat", "god", "hello", "feline", "cat"];
    let translated_values: Vec<String> = vec![
        "Bonjour", "bonjour", "chat", "Dieu", "Bonjour", "félin", "chat",
    ]
    .iter()
    .map(|v| v.to_string())
    .collect();
    let translated = translate_v2(&source_values, "en", "fr");

    assert_eq!(translated, Ok(translated_values));
}
