use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A network request issued by an extension that holds the `network` permission.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchRequest {
    pub url: String,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub body: Option<String>,
}

#[derive(Serialize)]
pub struct FetchResponse {
    pub status: u16,
    pub ok: bool,
    pub text: String,
    pub headers: HashMap<String, String>,
}

/// Perform an HTTP request on behalf of an extension. Routing through Rust (vs. a
/// direct browser `fetch`) is what makes the extension `network` capability work
/// at all in the packaged app, whose Content-Security-Policy forbids the WebView
/// from connecting to external origins.
#[tauri::command]
pub async fn ext_fetch(req: FetchRequest) -> Result<FetchResponse, String> {
    if !(req.url.starts_with("http://") || req.url.starts_with("https://")) {
        return Err("URL must start with http:// or https://".into());
    }
    let method = req.method.unwrap_or_else(|| "GET".into()).to_uppercase();
    let method = reqwest::Method::from_bytes(method.as_bytes()).map_err(|e| e.to_string())?;

    let client = reqwest::Client::new();
    let mut rb = client.request(method, req.url.as_str());
    if let Some(headers) = req.headers {
        for (k, v) in &headers {
            rb = rb.header(k.as_str(), v.as_str());
        }
    }
    if let Some(body) = req.body {
        rb = rb.body(body);
    }

    let resp = rb.send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let mut headers = HashMap::new();
    for (k, v) in resp.headers() {
        if let Ok(s) = v.to_str() {
            headers.insert(k.as_str().to_string(), s.to_string());
        }
    }
    let text = resp.text().await.map_err(|e| e.to_string())?;
    Ok(FetchResponse {
        status: status.as_u16(),
        ok: status.is_success(),
        text,
        headers,
    })
}
