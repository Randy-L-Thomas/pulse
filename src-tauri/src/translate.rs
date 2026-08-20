use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Default, Serialize, Deserialize)]
struct Cache {
    entries: HashMap<String, String>,
}

fn cache_path() -> PathBuf {
    crate::config::user_config_dir().join("mt-cache.json")
}

fn load_cache() -> Cache {
    fs::read_to_string(cache_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_cache(cache: &Cache) {
    if let Ok(raw) = serde_json::to_string_pretty(cache) {
        let _ = fs::create_dir_all(crate::config::user_config_dir());
        let _ = fs::write(cache_path(), raw);
    }
}

fn key(from: &str, to: &str, text: &str) -> String {
    let mut h = Sha256::new();
    h.update(from.as_bytes());
    h.update(b"|");
    h.update(to.as_bytes());
    h.update(b"|");
    h.update(text.as_bytes());
    hex::encode(h.finalize())
}

#[derive(Serialize)]
pub struct TranslateOut {
    pub text: String,
    pub cached: bool,
    pub engine: String,
}

pub async fn translate(
    client: &reqwest::Client,
    ollama_url: &str,
    model: &str,
    from: &str,
    to: &str,
    source: &str,
    enrich: bool,
) -> Result<TranslateOut, String> {
    let src = source.trim();
    if src.is_empty() {
        return Ok(TranslateOut {
            text: String::new(),
            cached: true,
            engine: "empty".into(),
        });
    }
    let from = if from == "auto" {
        detect_lang(src)
    } else {
        from
    };
    let k = key(from, to, src);
    let mut cache = load_cache();
    if !enrich {
        if let Some(hit) = cache.entries.get(&k) {
            return Ok(TranslateOut {
                text: hit.clone(),
                cached: true,
                engine: "cache".into(),
            });
        }
    }
    let mut out = mt_ollama(client, ollama_url, model, from, to, src).await?;
    if enrich {
        if let Ok(polished) = enrich_ollama(client, ollama_url, model, from, to, src, &out).await {
            if !polished.trim().is_empty() {
                out = polished;
            }
        }
    }
    cache.entries.insert(k, out.clone());
    save_cache(&cache);
    Ok(TranslateOut {
        text: out,
        cached: false,
        engine: format!("{from}→{to}"),
    })
}

pub fn detect_lang(text: &str) -> &'static str {
    let t = text.to_lowercase();
    let marks = t.chars().filter(|c| "áéíóúñü¿¡".contains(*c)).count();
    let es = ["qué", "hola", "gracias", "por favor", "buenos", "usted", "está", "también", "mañana"];
    let en = ["the ", " and ", " you ", " that ", " with ", " this "];
    let es_hits = es.iter().filter(|w| t.contains(*w)).count() + marks;
    let en_hits = en.iter().filter(|w| t.contains(*w)).count();
    if es_hits > en_hits {
        "es"
    } else if en_hits > 0 {
        "en"
    } else {
        "es"
    }
}

fn lang_name(code: &str) -> &'static str {
    match code {
        "es" => "Spanish",
        "en" => "English",
        "fr" => "French",
        "de" => "German",
        "pt" => "Portuguese",
        "it" => "Italian",
        "ar" => "Arabic",
        "zh" => "Chinese",
        "ja" => "Japanese",
        _ => "the target language",
    }
}

async fn mt_ollama(
    client: &reqwest::Client,
    base: &str,
    model: &str,
    from: &str,
    to: &str,
    src: &str,
) -> Result<String, String> {
    let model = pick_model(client, base, model).await?;
    let prompt = format!(
        "Translate from {} to {}. Output only the translation, no quotes or notes.\n\n{}",
        lang_name(from),
        lang_name(to),
        src
    );
    generate(client, base, &model, &prompt, 0.0).await
}

async fn enrich_ollama(
    client: &reqwest::Client,
    base: &str,
    model: &str,
    from: &str,
    to: &str,
    src: &str,
    draft: &str,
) -> Result<String, String> {
    let model = pick_model(client, base, model).await?;
    let prompt = format!(
        "Improve this {} translation of a chat message so it sounds natural (slang/register). Output only the rewritten translation.\n\nSource ({}): {}\nDraft: {}",
        lang_name(to),
        lang_name(from),
        src,
        draft
    );
    generate(client, base, &model, &prompt, 0.3).await
}

async fn pick_model(client: &reqwest::Client, base: &str, preferred: &str) -> Result<String, String> {
    if !preferred.trim().is_empty() {
        return Ok(preferred.trim().to_string());
    }
    let names = crate::ollama::list_models(client, base).await?;
    let prefer = ["llama3.2:1b", "llama3.2:3b", "qwen2.5:1.5b", "qwen2.5:3b", "gemma2:2b"];
    for p in prefer {
        if names.iter().any(|n| n == p || n.starts_with(p)) {
            return Ok(p.to_string());
        }
    }
    names.into_iter().next().ok_or_else(|| "no Ollama models — pull one for local MT".into())
}

async fn generate(
    client: &reqwest::Client,
    base: &str,
    model: &str,
    prompt: &str,
    temperature: f32,
) -> Result<String, String> {
    let url = format!("{}/api/generate", base.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "options": { "temperature": temperature, "num_predict": 512 }
    });
    let res = client
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Ollama: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("Ollama HTTP {}", res.status()));
    }
    let v: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    Ok(v.get("response")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim()
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::detect_lang;

    #[test]
    fn detects_spanish_from_marks_and_words() {
        assert_eq!(detect_lang("¿Cómo estás? Gracias por el mensaje."), "es");
    }

    #[test]
    fn detects_english_from_common_words() {
        assert_eq!(detect_lang("The cat sat with you and that box."), "en");
    }
}
