//api_version_v2

use std::env;

use log::{info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use ureq::http::StatusCode;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TranslatedResponse {
    pub data: Data,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    pub translations: Vec<Translation>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Translation {
    pub translated_text: String,
}

///Translate using v2 api
/// max string that can be taken by the q param is 128
///
pub fn translate_v2(
    source_data: &Vec<&str>,
    source_lang: &str,
    target_lang: &str,
) -> Result<Vec<String>, String> {
    let mut translated: Vec<String> = Vec::with_capacity(source_data.len());
    let api_url = "https://translation.googleapis.com/language/translate/v2";

    let api_key = env::var("GOOGLE_API_KEY").ok();

    if let Some(key) = api_key
        && !key.is_empty()
    {
        let chunks: Vec<&[&str]> = source_data.chunks(120).collect();

        for chunk in chunks {
            let qry_pairs: Vec<(&str, &str)> = chunk.iter().map(|q| ("q", *q)).collect();

            let response = ureq::get(api_url)
                .query("key", &key)
                .query("source", source_lang)
                .query("target", target_lang)
                .query_pairs(qry_pairs)
                .call();

            match response {
                Ok(mut translated_res) => {
                    if translated_res.status() == StatusCode::OK {
                        let data_res = translated_res.body_mut().read_json::<TranslatedResponse>();

                        match data_res {
                            Ok(data) => {
                                for translated_text in data.data.translations {
                                    translated.push(translated_text.translated_text);
                                }
                            }
                            Err(e) => return Err(e.to_string()),
                        }
                    } else {
                        return Err("Could not translate".to_string());
                    }
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(translated)
    } else {
        warn!("Google API key not found. Set it using GOOGLE_API_KEY variable");
        info!("Using google translate web...");

        let web_url = "https://translate.google.com/m";

        for romanize in source_data {
            let res = ureq::get(web_url)
                .query("sl", source_lang)
                .query("tl", target_lang)
                .query("q", romanize)
                .call();

            match res {
                Ok(mut response) => {
                    if response.status() == StatusCode::OK {
                        let t_text = get_translated_text(
                            &response.body_mut().read_to_string().unwrap_or_default(),
                        )?;

                        translated.push(t_text)
                    } else {
                        return Err("Invalid request".to_string());
                    }
                }
                Err(e) => return Err(e.to_string()),
            }
        }

        Ok(translated)
    }
}

fn get_translated_text(html: &str) -> Result<String, String> {
    // extracting translation text
    let pattern = Regex::new(r#"(?s)class="(?:t0|result-container)">(.*?)<"#).unwrap();
    if let Some(captures) = pattern.captures(html) {
        Ok(html_escape::decode_html_entities(&captures[1]).to_string())
    } else {
        Err("Invalid request".to_string())
    }
}
