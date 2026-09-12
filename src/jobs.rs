//! Output preflight, durable task receipts, and bounded batch execution.
use crate::{
    api::{self, Api},
    config,
};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct RunOptions {
    pub dry_run: bool,
    pub force: bool,
    pub timeout: u64,
    pub interval: u64,
    pub config_dir: PathBuf,
}
#[derive(Clone)]
pub struct Job {
    pub payload: Value,
    pub paths: Vec<PathBuf>,
}
#[derive(Serialize, Deserialize)]
pub struct Receipt {
    pub task_id: String,
    pub model: String,
    pub outputs: Vec<PathBuf>,
}

pub fn paths(out: Option<&Path>, dir: Option<&Path>, count: u8, fmt: &str) -> Result<Vec<PathBuf>> {
    if let Some(dir) = dir {
        let dir = config::expand(dir)?;
        return Ok((1..=count)
            .map(|n| dir.join(format!("image-{n}.{fmt}")))
            .collect());
    }
    let mut base = config::expand(out.unwrap_or(Path::new("output/imagegen/image")))?;
    if base.extension().is_none() {
        base.set_extension(fmt);
    }
    let stem = base
        .file_stem()
        .context("Output needs a filename")?
        .to_string_lossy();
    let ext = base.extension().unwrap_or_default().to_string_lossy();
    Ok(if count == 1 {
        vec![base.clone()]
    } else {
        (1..=count)
            .map(|n| base.with_file_name(format!("{stem}-{n}.{ext}")))
            .collect()
    })
}
pub fn absolute(path: &Path) -> Result<PathBuf> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut clean = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::ParentDir => {
                clean.pop();
            }
            std::path::Component::CurDir => (),
            _ => clean.push(part.as_os_str()),
        }
    }
    Ok(clean)
}
pub fn preflight(paths: &[PathBuf], force: bool, writable: bool) -> Result<()> {
    let mut stems = std::collections::HashSet::new();
    for path in paths {
        let full = absolute(path)?;
        if !stems.insert(full.with_extension("").to_string_lossy().to_lowercase()) {
            bail!("Output paths collide after extension normalization");
        }
        if path.is_dir() {
            bail!("Output is a directory: {}", path.display());
        }
        if !force {
            for candidate in std::iter::once(path.clone())
                .chain(["png", "jpg", "jpeg", "webp", "gif"].map(|ext| path.with_extension(ext)))
            {
                if candidate.exists() {
                    bail!(
                        "Output exists: {}. Choose a new path or use --force.",
                        candidate.display()
                    );
                }
            }
        }
        if writable {
            let parent = full.parent().context("Output requires a parent")?;
            fs::create_dir_all(parent)?;
            tempfile::tempfile_in(parent).context("Output directory is not writable")?;
        }
    }
    Ok(())
}
pub fn receipt_path(dir: &Path, task: &str) -> PathBuf {
    dir.join("tasks")
        .join(format!("{:x}.json", Sha256::digest(task.as_bytes())))
}
pub fn read_receipt(dir: &Path, task: &str) -> Result<Option<Receipt>> {
    let path = receipt_path(dir, task);
    if !path.is_file() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(&config::read_text(&path)?)?))
}
pub fn save(api: &Api, data: &Value, paths: &[PathBuf], force: bool) -> Result<Vec<PathBuf>> {
    let urls = api::result_urls(data)?;
    let mut saved = Vec::new();
    for (url, path) in urls.iter().zip(paths) {
        let bytes = api.download(url)?;
        let fmt = api::image_format(&bytes)?;
        let mut target = absolute(path)?;
        if !(target
            .extension()
            .is_some_and(|e| e == fmt || (fmt == "jpeg" && e == "jpg")))
        {
            target.set_extension(fmt);
        }
        let parent = target.parent().context("Output needs a parent")?;
        fs::create_dir_all(parent)?;
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.write_all(&bytes)?;
        temp.as_file().sync_all()?;
        if force {
            temp.persist(&target).map_err(|e| e.error)?;
        } else {
            temp.persist_noclobber(&target).map_err(|e| e.error)?;
        }
        eprintln!("Saved: {}", target.display());
        saved.push(target);
    }
    if urls.len() != paths.len() {
        bail!(
            "Task returned {} images, expected {}; saved {}. Inspect task before requesting more.",
            urls.len(),
            paths.len(),
            saved.len()
        );
    }
    Ok(saved)
}
pub fn execute(api: Option<&Api>, job: &Job, options: &RunOptions) -> Result<Value> {
    if options.dry_run {
        return Ok(
            json!({"dry_run":true,"payload":job.payload,"outputs":job.paths,"note":"Local references upload only during a real run"}),
        );
    }
    let api = api.context("HTTP client required")?;
    let key = config::require_key(&options.config_dir)?;
    let mut payload = job.payload.clone();
    if let Some(mask) = payload["mask_url"].as_str() {
        api.validate_mask(
            mask,
            payload["image_urls"][0]
                .as_str()
                .context("Mask requires image")?,
        )?;
    }
    if let Some(refs) = payload["image_urls"].as_array() {
        let urls = refs
            .iter()
            .map(|v| api.upload(v.as_str().unwrap_or(""), &key))
            .collect::<Result<Vec<_>>>()?;
        payload["image_urls"] = json!(urls);
    }
    if let Some(mask) = payload["mask_url"].as_str() {
        payload["mask_url"] = api.upload(mask, &key)?.into();
    }
    let task = api.submit(&payload, &key)?;
    eprintln!(
        "Task: {task} | model: {}",
        payload["model"].as_str().unwrap_or("")
    );
    let receipt = Receipt {
        task_id: task.clone(),
        model: payload["model"].as_str().unwrap_or("").into(),
        outputs: job
            .paths
            .iter()
            .map(|p| absolute(p))
            .collect::<Result<_>>()?,
    };
    let path = receipt_path(&options.config_dir, &task);
    if let Err(err) = config::private_write(&path, &serde_json::to_vec_pretty(&receipt)?) {
        eprintln!("Receipt could not be saved ({err}); keep task ID {task}.");
    }
    let result = (|| -> Result<Value> {
        let data = api.poll(&task, &key, options.timeout, options.interval)?;
        let outputs = save(api, &data, &job.paths, options.force)?;
        Ok(
            json!({"task_id":task,"model":payload["model"],"outputs":outputs,"cost":data["cost"],"usage":data["usage"]}),
        )
    })();
    result.with_context(|| format!("Task {task} was submitted. Resume with imagegen fetch {task}; do not generate it again."))
}
