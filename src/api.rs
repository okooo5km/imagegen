//! HTTP client: one paid submission, resumable polling, no secret-bearing logs.
use anyhow::{Context, Result, bail};
use reqwest::{
    Url,
    blocking::{Client, RequestBuilder, multipart},
    redirect::Policy,
};
use serde_json::{Value, json};
use std::{
    fs,
    io::Read,
    path::Path,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const BASE: &str = "https://api.apimart.ai";
pub const MAX_UPLOAD: u64 = 20 * 1024 * 1024;
const MAX_DOWNLOAD: u64 = 100 * 1024 * 1024;
#[derive(Clone)]
pub struct Api {
    base: String,
    auth: Client,
    download: Client,
}
impl Api {
    pub fn new() -> Result<Self> {
        Self::with_base(BASE)
    }
    pub fn with_base(base: &str) -> Result<Self> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let auth = Client::builder()
            .redirect(Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(60))
            .user_agent(concat!("imagegen/", env!("CARGO_PKG_VERSION")))
            .build()?;
        let download = Client::builder()
            .redirect(Policy::limited(5))
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(60))
            .build()?;
        Ok(Self {
            base: base.trim_end_matches('/').into(),
            auth,
            download,
        })
    }
    fn json(&self, request: RequestBuilder) -> Result<Value> {
        let response = request.send().map_err(|_| anyhow::anyhow!("Network request failed. If submitting, check APIMart history before retrying; no automatic resubmission."))?;
        if !response.status().is_success() {
            bail!(
                "HTTP {} from APIMart. Check credentials, balance, parameters or service status. No automatic resubmission.",
                response.status().as_u16()
            );
        }
        let mut raw = Vec::new();
        response
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut raw)
            .context("Cannot read API response")?;
        if raw.len() > 8 * 1024 * 1024 {
            bail!("API response exceeds 8 MiB");
        }
        let value: Value =
            serde_json::from_slice(&raw).context("Invalid JSON response from API")?;
        if !value.is_object()
            || value.get("success") == Some(&Value::Bool(false))
            || value.get("error").is_some_and(|v| !v.is_null())
            || value.get("code").is_some_and(|v| v != 0 && v != 200)
        {
            bail!("API reported an error or unexpected response; check APIMart history");
        }
        Ok(value)
    }
    /// Verify access with a read-only model catalog request; never submit an image.
    pub fn check_auth(&self, key: &str) -> Result<()> {
        let response = self.auth.get(format!("{}/v1/models", self.base))
            .bearer_auth(key).send().map_err(|error| {
                if error.is_timeout() {
                    anyhow::anyhow!("Authentication check timed out. Keep your key and retry later.")
                } else {
                    anyhow::anyhow!("Authentication check could not reach APIMart. Check your network; key validity is unknown.")
                }
            })?;
        match response.status().as_u16() {
            401 => bail!(
                "APIMart rejected the key (HTTP 401). Run imagegen auth set to replace it; check APIMART_API_KEY if set."
            ),
            403 => bail!(
                "APIMart denied model-list access (HTTP 403). Check key permissions or account restrictions; this does not prove the key is invalid."
            ),
            429 => {
                bail!("APIMart rate limited the check (HTTP 429). Keep your key and retry later.")
            }
            code if !(200..300).contains(&code) => bail!(
                "Authentication check returned HTTP {code}; key validity is unknown. Retry after checking service status."
            ),
            _ => (),
        }
        let mut raw = Vec::new();
        response
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut raw)
            .map_err(|_| {
                anyhow::anyhow!("Cannot read authentication response; key validity is unknown.")
            })?;
        if raw.len() > 8 * 1024 * 1024 {
            bail!("Authentication response exceeds 8 MiB");
        }
        let value: Value = serde_json::from_slice(&raw).map_err(|_| {
            anyhow::anyhow!("Unexpected authentication response; key validity is unknown.")
        })?;
        if !value["data"].is_array()
            || value.get("success") == Some(&Value::Bool(false))
            || value.get("error").is_some_and(|v| !v.is_null())
            || value.get("code").is_some_and(|v| v != 0 && v != 200)
        {
            bail!(
                "APIMart did not confirm model-list access; check account permissions. Key validity is unknown."
            );
        }
        Ok(())
    }
    pub fn upload(&self, reference: &str, key: &str) -> Result<String> {
        validate_reference(reference)?;
        if public_url(reference) {
            return Ok(reference.into());
        }
        let path = crate::config::expand(Path::new(reference))?;
        let bytes = fs::read(path)?;
        let format = image_format(&bytes)?;
        let part = multipart::Part::bytes(bytes)
            .file_name(format!("image.{format}"))
            .mime_str(&format!("image/{format}"))?;
        let result = self.json(
            self.auth
                .post(format!("{}/v1/uploads/images", self.base))
                .bearer_auth(key)
                .multipart(multipart::Form::new().part("file", part)),
        )?;
        let data = unwrap(&result)?;
        let url = data["url"]
            .as_str()
            .filter(|s| public_url(s))
            .context("Upload returned no public image URL")?;
        Ok(url.into())
    }
    pub fn submit(&self, payload: &Value, key: &str) -> Result<String> {
        let result = self.json(
            self.auth
                .post(format!("{}/v1/images/generations", self.base))
                .bearer_auth(key)
                .json(payload),
        )?;
        let data = unwrap(&result)?;
        let task = data["task_id"]
            .as_str()
            .or(data["id"].as_str())
            .filter(|s| !s.is_empty())
            .context("No task ID in submission response; check APIMart history before retrying")?;
        Ok(task.into())
    }
    pub fn poll(&self, task: &str, key: &str, timeout: u64, interval: u64) -> Result<Value> {
        let end = Instant::now() + Duration::from_secs(timeout);
        let mut url = Url::parse(&format!("{}/v1/tasks/", self.base))?;
        url.path_segments_mut()
            .map_err(|_| anyhow::anyhow!("Invalid API URL"))?
            .pop_if_empty()
            .push(task);
        while Instant::now() < end {
            let remaining = end.saturating_duration_since(Instant::now());
            let response = self.json(
                self.auth
                    .get(url.clone())
                    .bearer_auth(key)
                    .timeout(remaining.min(Duration::from_secs(60))),
            )?;
            let data = unwrap(&response)?;
            match data["status"].as_str().unwrap_or("") {
                "completed" | "succeeded" | "success" => return Ok(data.clone()),
                "failed" | "error" | "cancelled" | "canceled" => bail!(
                    "Task {task} failed; inspect APIMart task details. Do not blindly resubmit."
                ),
                _ => std::thread::sleep(
                    Duration::from_secs(interval)
                        .min(end.saturating_duration_since(Instant::now())),
                ),
            }
        }
        bail!("Task {task} is still pending. Resume: imagegen fetch {task}. Do not submit again.")
    }
    pub fn download(&self, url: &str) -> Result<Vec<u8>> {
        if !public_url(url) {
            bail!("Expected a public HTTP(S) URL");
        }
        let response = self.download.get(url).send().map_err(|_| {
            anyhow::anyhow!("Image download failed; use fetch to resume the same task")
        })?;
        if !response.status().is_success() {
            bail!(
                "Image download returned HTTP {}; use fetch to resume",
                response.status().as_u16()
            );
        }
        let mut bytes = Vec::new();
        response
            .take(MAX_DOWNLOAD + 1)
            .read_to_end(&mut bytes)
            .context("Cannot read image download")?;
        if bytes.len() as u64 > MAX_DOWNLOAD {
            bail!("Download exceeds 100 MiB");
        }
        image_format(&bytes)?;
        Ok(bytes)
    }
    pub fn validate_mask(&self, mask: &str, first: &str) -> Result<()> {
        let load = |v: &str| -> Result<Vec<u8>> {
            if public_url(v) {
                self.download(v)
            } else {
                Ok(fs::read(crate::config::expand(Path::new(v))?)?)
            }
        };
        validate_mask_bytes(&load(mask)?, &load(first)?)
    }
    pub fn catalog(&self, key: &str) -> Result<Value> {
        let result = self.json(
            self.auth
                .get(format!(
                    "{}/v1/models?expand=parameters&category=image",
                    self.base
                ))
                .bearer_auth(key),
        )?;
        let rows = result["data"]
            .as_array()
            .context("Model list returned no array")?;
        Ok(Value::Array(
            rows.iter()
                .filter(|r| r["category"] == "image" && r["id"].is_string())
                .map(|r| {
                    let mut row = r.clone();
                    row["runnable"] = crate::models::MODELS
                        .contains(&r["id"].as_str().unwrap_or(""))
                        .into();
                    row
                })
                .collect(),
        ))
    }
    pub fn pricing(&self, model: &str) -> Result<Value> {
        let mut url = Url::parse(&format!("{}/api/pricing/model", self.base))?;
        url.query_pairs_mut().append_pair("model", model);
        // This public endpoint must never receive Authorization.
        let response = self.json(self.auth.get(url.clone()))?;
        let data = unwrap(&response)?;
        let mut info = json!({"model":model,"source":url.as_str(),"retrieved_at_unix":SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),"scope":"public default group; account charges may differ"});
        if data["pricing"].is_object() {
            info["pricing"] = data["pricing"].clone();
            info["status"] = if data["pricing"]["effective_rates"].is_object() {
                "available"
            } else {
                "rates unavailable"
            }
            .into();
        } else {
            info["legacy_quote"] = json!({});
            for field in [
                "quota_type",
                "model_price",
                "resolution_prices",
                "size_quality_prices",
                "discount_rate",
                "discount_percent",
            ] {
                if let Some(v) = data.get(field) {
                    info["legacy_quote"][field] = v.clone();
                }
            }
            info["status"] = "legacy quote; payable rate unverified".into();
        }
        Ok(info)
    }
}
pub fn unwrap(value: &Value) -> Result<&Value> {
    let data = value.get("data").unwrap_or(value);
    let data = if data.is_array() {
        data.get(0).context("Empty API data")?
    } else {
        data
    };
    if !data.is_object() {
        bail!("Unexpected API data shape");
    }
    Ok(data)
}
pub fn public_url(s: &str) -> bool {
    Url::parse(s).is_ok_and(|u| {
        ["https", "http"].contains(&u.scheme())
            && u.host_str().is_some()
            && u.username().is_empty()
            && u.password().is_none()
    })
}
pub fn image_format(bytes: &[u8]) -> Result<&'static str> {
    match image::guess_format(bytes)? {
        image::ImageFormat::Png => Ok("png"),
        image::ImageFormat::Jpeg => Ok("jpeg"),
        image::ImageFormat::WebP => Ok("webp"),
        image::ImageFormat::Gif => Ok("gif"),
        _ => bail!("Only PNG, JPEG, WebP and GIF images are supported"),
    }
}
pub fn validate_reference(s: &str) -> Result<()> {
    if public_url(s) {
        return Ok(());
    }
    if s.starts_with("data:") || s.starts_with("http:") || s.starts_with("https:") {
        bail!("Use a public HTTP(S) URL or local file; Base64 is not supported");
    }
    let path = crate::config::expand(Path::new(s))?;
    let meta =
        fs::metadata(&path).with_context(|| format!("Image file not found: {}", path.display()))?;
    if !meta.is_file() || meta.len() > MAX_UPLOAD {
        bail!("Input must be a file at most 20 MiB: {}", path.display());
    }
    let mut head = Vec::new();
    fs::File::open(path)?.take(32).read_to_end(&mut head)?;
    image_format(&head)?;
    Ok(())
}
pub fn validate_local_mask(mask: &str, first: &str) -> Result<()> {
    if public_url(mask) || public_url(first) {
        return Ok(());
    }
    validate_mask_bytes(
        &fs::read(crate::config::expand(Path::new(mask))?)?,
        &fs::read(crate::config::expand(Path::new(first))?)?,
    )
}
fn validate_mask_bytes(mask: &[u8], first: &[u8]) -> Result<()> {
    let m = image::load_from_memory(mask)?;
    let r = image::load_from_memory(first)?;
    if image_format(mask)? != "png"
        || !m.color().has_alpha()
        || (m.width(), m.height()) != (r.width(), r.height())
    {
        bail!("Mask must be a PNG with alpha matching the first reference dimensions");
    }
    Ok(())
}
pub fn result_urls(data: &Value) -> Result<Vec<String>> {
    let images = data["result"]["images"]
        .as_array()
        .or(data["images"].as_array())
        .context("Completed task returned no images")?;
    let mut urls = Vec::new();
    for item in images {
        match &item["url"] {
            Value::String(s) if public_url(s) => urls.push(s.clone()),
            Value::Array(a) => urls.extend(
                a.iter()
                    .filter_map(|v| v.as_str().filter(|s| public_url(s)).map(str::to_owned)),
            ),
            _ => (),
        }
    }
    if urls.is_empty() {
        bail!("Completed task returned no valid image URLs");
    }
    Ok(urls)
}
