use serde::Deserialize;

#[derive(Deserialize)]
struct Tags {
    models: Vec<Tag>,
}

#[derive(Deserialize)]
struct Tag {
    name: String,
}

pub async fn list_models(client: &reqwest::Client, base: &str) -> Result<Vec<String>, String> {
    let url = format!("{}/api/tags", base.trim_end_matches('/'));
    let res = client.get(url).send().await.map_err(|e| format!("Ollama: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("Ollama HTTP {}", res.status()));
    }
    let tags: Tags = res.json().await.map_err(|e| e.to_string())?;
    Ok(tags.models.into_iter().map(|m| m.name).collect())
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMsg {
    pub role: String,
    pub content: String,
}

pub async fn chat(
    client: &reqwest::Client,
    base: &str,
    model: &str,
    messages: Vec<ChatMsg>,
) -> Result<String, String> {
    if model.trim().is_empty() {
        return Err("pick a model".into());
    }
    let url = format!("{}/api/chat", base.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": false,
        "options": { "temperature": 0.7, "num_predict": 1024 }
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
    Ok(v.pointer("/message/content")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim()
        .to_string())
}
