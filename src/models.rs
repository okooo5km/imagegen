//! Explicit, verified APIMart parameter profiles.
use crate::config::{Config, DEFAULT_MODEL};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

pub const MODELS: &[&str] = &[
    "gpt-image-2.5-flare",
    "gpt-image-2.5-sunburst",
    "gpt-image-2-official",
    "gpt-image-2",
    "gemini-3.1-flash-image-preview",
];
pub const RATIOS: &[&str] = &[
    "auto", "1:1", "3:2", "2:3", "4:3", "3:4", "5:4", "4:5", "16:9", "9:16", "2:1", "1:2", "21:9",
    "9:21", "3:1", "1:3",
];
pub fn is_25(m: &str) -> bool {
    matches!(m, "gpt-image-2.5-flare" | "gpt-image-2.5-sunburst")
}
pub fn rich(m: &str) -> bool {
    is_25(m) || m == "gpt-image-2-official"
}
pub fn validate_model(m: &str) -> Result<()> {
    if !MODELS.contains(&m) {
        bail!(
            "No runnable profile for {m}. Use imagegen models; live discovery does not imply generation support."
        );
    }
    Ok(())
}
pub fn validate_ratio(s: &str) -> Result<()> {
    if !RATIOS.contains(&s) {
        bail!("Unknown aspect ratio: {s}");
    }
    Ok(())
}
pub fn validate_size(s: &str) -> Result<()> {
    if s == "auto" {
        return Ok(());
    }
    let pair = s
        .split_once('x')
        .and_then(|(a, b)| Some((a.parse::<u64>().ok()?, b.parse::<u64>().ok()?)));
    let Some((w, h)) = pair else {
        bail!("Size must be auto or exact WIDTHxHEIGHT; use aspect_ratio for ratios");
    };
    if w == 0
        || h == 0
        || w > 3840
        || h > 3840
        || w % 16 != 0
        || h % 16 != 0
        || w.max(h) > 3 * w.min(h)
        || !(655_360..=8_294_400).contains(&(w * h))
    {
        bail!(
            "Exact size requires multiples of 16, sides <=3840, aspect <=3:1, and 655360..8294400 pixels"
        );
    }
    Ok(())
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub prompt: Option<String>,
    pub model: Option<String>,
    pub aspect_ratio: Option<String>,
    pub size: Option<String>,
    pub resolution: Option<String>,
    pub quality: Option<String>,
    pub background: Option<String>,
    pub output_format: Option<String>,
    pub output_compression: Option<u8>,
    pub moderation: Option<String>,
    pub n: Option<u8>,
    #[serde(default)]
    pub image_urls: Vec<String>,
    pub mask_url: Option<String>,
    pub google_search: Option<bool>,
    pub google_image_search: Option<bool>,
    pub official_fallback: Option<bool>,
    pub out: Option<String>,
}

pub fn build(
    input: &Input,
    conf: &Config,
    env_model: Option<&str>,
    use_default_size: bool,
) -> Result<serde_json::Value> {
    let d = &conf.defaults;
    let model = input
        .model
        .as_deref()
        .filter(|m| *m != "auto")
        .or(env_model.filter(|s| !s.is_empty()))
        .or(d.model.as_deref())
        .unwrap_or(DEFAULT_MODEL);
    validate_model(model)?;
    let prompt = input.prompt.as_deref().unwrap_or("");
    if prompt.trim().is_empty() {
        bail!("A non-empty prompt is required");
    }
    let n = input.n.or(d.n).unwrap_or(1);
    let max_n = if model == "gpt-image-2" { 1 } else { 4 };
    if n < 1 || n > max_n {
        bail!("n must be 1..{max_n} for {model}");
    }
    if input.size.is_some() && input.aspect_ratio.is_some() {
        bail!("Choose size or aspect_ratio, not both");
    }
    if input.size.is_some() && input.resolution.is_some() {
        bail!("Exact size cannot be combined with resolution; use aspect_ratio + resolution");
    }
    let mut p = serde_json::json!({"model": model, "prompt": prompt, "n":n});
    let edit = !input.image_urls.is_empty();
    let inherit_geometry = !edit || use_default_size;
    let explicit_ratio = input.aspect_ratio.is_some() || input.resolution.is_some();
    let size = input
        .size
        .as_deref()
        .or(if inherit_geometry && !explicit_ratio {
            d.size.as_deref()
        } else {
            None
        });
    let ratio = input.aspect_ratio.as_deref().or(if inherit_geometry {
        d.aspect_ratio.as_deref()
    } else {
        None
    });
    if let Some(size) = size {
        validate_size(size)?;
        if !is_25(model) {
            bail!("Exact size/auto requires a GPT Image 2.5 profile");
        }
        p["size"] = size.into();
    } else {
        if let Some(ratio) = ratio.or(if edit { None } else { Some("1:1") }) {
            validate_ratio(ratio)?;
            if !is_25(model)
                && !["1:1", "16:9", "9:16", "4:3", "3:4", "3:2", "2:3", "21:9"].contains(&ratio)
            {
                bail!("Unsupported ratio for legacy model: {ratio}");
            }
            p["size"] = ratio.into();
        }
        let resolution = input
            .resolution
            .as_deref()
            .or(d.resolution.as_deref())
            .unwrap_or("1k");
        if !["1k", "2k", "4k"].contains(&resolution) {
            bail!("resolution must be 1k, 2k or 4k");
        }
        if !is_25(model)
            && resolution == "4k"
            && !["16:9", "9:16", "21:9"].contains(&p["size"].as_str().unwrap_or(""))
        {
            bail!("Legacy profile 4k requires 16:9, 9:16 or 21:9");
        }
        p["resolution"] = resolution.into();
    }
    for (name, v) in [
        ("quality", input.quality.as_ref().or(d.quality.as_ref())),
        (
            "background",
            input.background.as_ref().or(d.background.as_ref()),
        ),
        (
            "output_format",
            input.output_format.as_ref().or(d.output_format.as_ref()),
        ),
    ] {
        if let Some(value) = v {
            if !rich(model) {
                bail!(
                    "{model} does not support {name}; unset the saved default or choose another model"
                );
            }
            p[name] = value.as_str().into();
        }
    }
    if is_25(model) && p.get("quality").is_none() {
        p["quality"] = "medium".into();
    }
    if let Some(q) = p["quality"].as_str()
        && !["auto", "low", "medium", "high"].contains(&q)
        && !(is_25(model) && ["xhigh", "max"].contains(&q))
    {
        bail!("Invalid quality for {model}");
    }
    let fmt = p["output_format"].as_str().unwrap_or("png");
    if !["png", "jpeg", "webp"].contains(&fmt) {
        bail!("output_format must be png, jpeg or webp");
    }
    if let Some(bg) = p["background"].as_str() {
        if !["auto", "opaque", "transparent"].contains(&bg) {
            bail!("Invalid background");
        }
        if bg == "transparent" && fmt == "jpeg" {
            bail!("Transparency requires PNG or WebP");
        }
    }
    if let Some(compression) = input.output_compression {
        if !rich(model) || compression > 100 || !["jpeg", "webp"].contains(&fmt) {
            bail!("output_compression requires JPEG/WebP and a supported model, range 0..100");
        }
        p["output_compression"] = compression.into();
    }
    if let Some(value) = &input.moderation {
        if !rich(model) || !["auto", "low"].contains(&value.as_str()) {
            bail!("Unsupported moderation");
        }
        p["moderation"] = value.as_str().into();
    }
    for (flag, value) in [
        ("google_search", input.google_search),
        ("google_image_search", input.google_image_search),
        ("official_fallback", input.official_fallback),
    ] {
        if let Some(value) = value {
            let supported = if flag == "official_fallback" {
                !rich(model)
            } else {
                model == "gemini-3.1-flash-image-preview"
            };
            if !supported {
                bail!("Unsupported parameter {flag} for {model}");
            }
            p[flag] = value.into();
        }
    }
    if edit {
        let cap = if rich(model) { 16 } else { 8 };
        if input.image_urls.len() > cap {
            bail!("At most {cap} reference images supported");
        }
        for reference in &input.image_urls {
            crate::api::validate_reference(reference)?;
        }
        p["image_urls"] = serde_json::json!(input.image_urls);
    }
    if let Some(mask) = &input.mask_url {
        if model != "gpt-image-2-official" || !edit {
            bail!("Masks require gpt-image-2-official and a reference image");
        }
        crate::api::validate_reference(mask)?;
        crate::api::validate_local_mask(mask, &input.image_urls[0])?;
        p["mask_url"] = mask.as_str().into();
    }
    Ok(p)
}
