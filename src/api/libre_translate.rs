//api_version_v1

//According to docs
//You can make up to 80 API calls per minute. These are bursts of up to 80 / minute.
//If you are translating non-stop, the actual limit is closer to 20 / minute (1200 / hour). Each call has a 2,000 character limit.

use std::{
    collections::HashMap,
    env,
    sync::Mutex,
    time::{Duration, Instant},
};

use html_escape::decode_html_entities;
use log::debug;
use serde::{Deserialize, Serialize};
use ureq::http::StatusCode;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TranslationResponse {
    pub translated_text: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranslationRequestBody {
    pub q: Vec<String>,
    pub source: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

pub struct TranslationLimiter {
    max_burst: u32,
    tokens_per_sec: f64,
    tokens: f64,
    last_update: Instant,
}

impl TranslationLimiter {
    pub fn new() -> Self {
        Self {
            max_burst: 80,
            // 20 per minute = 1 permit every 3 seconds (0.333... per second)
            tokens_per_sec: 20.0 / 60.0,
            tokens: 80.0, // Start full for the burst
            last_update: Instant::now(),
        }
    }
}

pub struct SyncRateLimiter(Mutex<TranslationLimiter>);

impl SyncRateLimiter {
    pub fn new() -> Self {
        Self(Mutex::new(TranslationLimiter::new()))
    }

    pub fn translate(
        &self,
        api_url: &str,
        json_body: TranslationRequestBody,
    ) -> Result<ureq::http::Response<ureq::Body>, ureq::Error> {
        let mut guard = self.0.lock().unwrap();

        loop {
            let now = Instant::now();
            let elapsed = now.duration_since(guard.last_update).as_secs_f64();
            guard.tokens =
                (guard.tokens + elapsed * guard.tokens_per_sec).min(guard.max_burst as f64);
            guard.last_update = now;

            if guard.tokens >= 1.0 {
                guard.tokens -= 1.0;
                break;
            }

            let wait_time = Duration::from_secs_f64((1.0 - guard.tokens) / guard.tokens_per_sec);

            drop(guard);
            std::thread::sleep(wait_time);

            // Re-acquire lock and try again
            guard = self.0.lock().unwrap();
        }

        ureq::post(api_url).send_json(json_body)
    }
}

///Translate using v1 api
///
pub fn translate_v1(
    source_data: &Vec<&str>,
    source_lang: &str,
    target_lang: &str,
) -> Result<Vec<String>, String> {
    let limiter = SyncRateLimiter::new();

    let mut translated: Vec<String> = Vec::with_capacity(source_data.len());

    let (api_key, api_url) = if let Some(key) = env::var("LIBRE_TRANSLATE_API_KEY").ok() {
        if key.is_empty() {
            (None, "http://127.0.0.1:5001/translate")
        } else {
            (Some(key), "https://libretranslate.com/translate")
        }
    } else {
        (None, "http://127.0.0.1:5001/translate")
    };

    let mut duplicates = 0;

    // if let Some(key) = api_key
    //     && !key.is_empty()
    // {
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
            q: qry_text,
            target: target_lang.to_string(),
            source: source_lang.to_string(),
            api_key: api_key.clone(),
        };

        let response = limiter.translate(api_url, json_body);
        match response {
            Ok(mut translated_res) => {
                if translated_res.status() == StatusCode::OK {
                    let data_res = translated_res.body_mut().read_json::<TranslationResponse>();

                    match data_res {
                        Ok(data) => {
                            let g_translated_data = &data.translated_text;

                            for (idx, text) in data.translated_text.iter().enumerate() {
                                let decoded_str = decode_html_entities(&text);
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
                                                if let Some(translation) = translated_value {
                                                    let init_pos_decoded =
                                                        decode_html_entities(&translation);
                                                    translated.push(init_pos_decoded.to_string());
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
                        Err(err) => return Err(err.to_string()),
                    }
                } else {
                    return Err("Could not translate".to_string());
                }
            }
            Err(e) => return Err(e.to_string()),
        }

        mem_cache.clear();
    }

    debug!("Duplicates found: {duplicates}");

    Ok(translated)
}

#[test]
fn test_translate_v1() {
    let source_values = vec!["hello", "mello", "cat", "god", "hello", "feline", "cat"];
    let translated_values: Vec<String> = vec![
        "bonjour", "mello", "chat", "dieu", "bonjour", "féline", "chat",
    ]
    .iter()
    .map(|v| v.to_string())
    .collect();
    let translated = translate_v1(&source_values, "en", "fr");

    assert_eq!(translated, Ok(translated_values));
}
