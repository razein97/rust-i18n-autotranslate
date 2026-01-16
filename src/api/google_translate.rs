//api_version_v2

use std::{collections::HashMap, env};

use html_escape::decode_html_entities;
use log::{debug, info, warn};
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
    let mut duplicates = 0;

    if let Some(key) = api_key
        && !key.is_empty()
    {
        let mut mem_cache: HashMap<&str, Vec<usize>> = HashMap::new();

        let chunks: Vec<&[&str]> = source_data.chunks(120).collect();

        for chunk in chunks {
            let mut qry_pairs: Vec<(&str, &str)> = Vec::new();

            for (idx, q) in chunk.iter().enumerate() {
                //if item in cache then record the position in the chunk array
                // send empty character for translation
                //You will be charged for only one character reducing usage
                if let Some(mem_val) = mem_cache.get_mut(q) {
                    mem_val.push(idx);
                    qry_pairs.push(("q", ""));

                    duplicates += 1;
                } else {
                    mem_cache.insert(*q, vec![idx]);
                    qry_pairs.push(("q", *q));
                }
            }

            let response = ureq::get(api_url)
                .config()
                .http_status_as_error(false)
                .build()
                .query("key", &key)
                .query("source", source_lang)
                .query("target", target_lang)
                .query_pairs(qry_pairs)
                .call();

            match response {
                Ok(mut translated_res) => {
                    match translated_res.status() {
                        StatusCode::OK => {
                            let data_res =
                                translated_res.body_mut().read_json::<TranslatedResponse>();

                            match data_res {
                                Ok(data) => {
                                    let g_translated_data = &data.data.translations;

                                    for (idx, translated_text) in
                                        data.data.translations.iter().enumerate()
                                    {
                                        let decoded_str =
                                            decode_html_entities(&translated_text.translated_text);

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
                                                                    &translation.translated_text,
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
                                Err(e) => return Err(e.to_string()),
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
                Err(e) => return Err(e.to_string()),
            }

            mem_cache.clear();
        }

        debug!("Duplicates found: {duplicates}");

        Ok(translated)
    } else {
        warn!("Google API key not found. Set it using GOOGLE_API_KEY variable");
        info!("Using google translate web...");

        let mut mem_cache: HashMap<&str, String> = HashMap::new();

        for romanize in source_data.iter() {
            if let Some(mem_val) = mem_cache.get(romanize) {
                translated.push(mem_val.to_owned());
                duplicates += 1;
            } else {
                //if not in mem cache
                match google_web_translate(source_lang, target_lang, *romanize) {
                    Ok(result) => {
                        translated.push(result.clone());
                        mem_cache.insert(*romanize, result);
                    }
                    Err(err) => return Err(err.to_string()),
                }
            }
        }

        debug!("Duplicates: {duplicates}");
        Ok(translated)
    }
}

fn google_web_translate(
    source_lang: &str,
    target_lang: &str,
    q: &str,
) -> Result<String, &'static str> {
    let web_url = "https://translate.google.com/m";
    let res = ureq::get(web_url)
        .query("sl", source_lang)
        .query("tl", target_lang)
        .query("q", q)
        .call();

    match res {
        Ok(mut response) => {
            if response.status() == StatusCode::OK {
                let t_text =
                    get_translated_text(&response.body_mut().read_to_string().unwrap_or_default())?;

                let decoded = decode_html_entities(&t_text);

                Ok(decoded.to_string())
            } else {
                Err("Invalid request")
            }
        }
        Err(_) => Err("Could not query google translate api"),
    }
}

fn get_translated_text(html: &str) -> Result<String, &'static str> {
    // extracting translation text
    let pattern = Regex::new(r#"(?s)class="(?:t0|result-container)">(.*?)<"#).unwrap();
    if let Some(captures) = pattern.captures(html) {
        Ok(html_escape::decode_html_entities(&captures[1]).to_string())
    } else {
        Err("Invalid request")
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
