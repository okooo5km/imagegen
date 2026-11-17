//! imagegen CLI. Author: okooo5km(十里).
use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use imagegen::{
    api::Api,
    config::{self, Config},
    jobs::{self, Job, RunOptions},
    models::{self, Input},
};
use serde_json::{Value, json};
use std::{
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Parser)]
#[command(version,about="Generate/edit images with APIMart; persistent defaults, model prices and task recovery",long_about=None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Generate(Generate),
    Edit(Generate),
    #[command(alias = "generate-batch")]
    Batch(Batch),
    Fetch(Fetch),
    Models {
        #[arg(long)]
        live: bool,
        #[arg(long)]
        prices: bool,
        #[arg(long)]
        json: bool,
    },
    Prices {
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    /// Show remaining and used balance for the API key (or whole account).
    Balance {
        /// Whole account instead of the current key.
        #[arg(long)]
        account: bool,
        #[arg(long)]
        json: bool,
    },
    /// Aggregated spend and tokens over a time range (default: last 24h).
    Usage(UsageArgs),
    /// Per-request logs, or one record by request/task ID.
    Logs(LogsArgs),
    #[command(name = "check-key")]
    CheckKey,
    Doctor,
    RemoveChromaKey(Chroma),
}
#[derive(Subcommand)]
enum ConfigCommand {
    Show,
    Path,
    Set { key: String, value: String },
    Unset { key: String },
    Reset,
}
#[derive(Subcommand)]
enum AuthCommand {
    Set,
    Status,
    /// Verify model-list access online without generating an image.
    Check,
}
#[derive(Args, Clone)]
struct Output {
    #[arg(long, conflicts_with = "out_dir")]
    out: Option<PathBuf>,
    #[arg(long)]
    out_dir: Option<PathBuf>,
    #[arg(long)]
    force: bool,
    #[arg(long,value_parser=clap::value_parser!(u64).range(1..=86400))]
    timeout: Option<u64>,
    #[arg(long,value_parser=clap::value_parser!(u64).range(1..=60))]
    poll_interval: Option<u64>,
}
#[derive(Args)]
struct Generate {
    #[arg(
        long,
        required_unless_present = "prompt_file",
        conflicts_with = "prompt_file"
    )]
    prompt: Option<String>,
    #[arg(long)]
    prompt_file: Option<PathBuf>,
    #[arg(long)]
    model: Option<String>,
    #[arg(long, alias = "ratio", conflicts_with = "size")]
    aspect_ratio: Option<String>,
    #[arg(
        long,
        conflicts_with = "resolution",
        help = "Exact WIDTHxHEIGHT or auto; use --aspect-ratio for ratios"
    )]
    size: Option<String>,
    #[arg(long)]
    resolution: Option<String>,
    #[arg(long)]
    quality: Option<String>,
    #[arg(long)]
    background: Option<String>,
    #[arg(long)]
    output_format: Option<String>,
    #[arg(long)]
    output_compression: Option<u8>,
    #[arg(long)]
    moderation: Option<String>,
    #[arg(long)]
    n: Option<u8>,
    #[arg(long = "image")]
    images: Vec<String>,
    #[arg(long)]
    mask: Option<String>,
    #[arg(long,num_args=0..=1,default_missing_value="true")]
    google_search: Option<bool>,
    #[arg(long,num_args=0..=1,default_missing_value="true")]
    google_image_search: Option<bool>,
    #[arg(long,num_args=0..=1,default_missing_value="true")]
    official_fallback: Option<bool>,
    #[arg(long)]
    dry_run: bool,
    #[arg(
        long,
        help = "Apply saved size/aspect ratio to edits; otherwise preserve reference proportions"
    )]
    use_default_size: bool,
    #[command(flatten)]
    output: Output,
}
#[derive(Args)]
struct UsageArgs {
    /// Range start: unix seconds or RFC3339 (max range 31 days).
    #[arg(long, alias = "since")]
    start: Option<String>,
    /// Range end: unix seconds or RFC3339.
    #[arg(long)]
    end: Option<String>,
    /// Comma-separated model IDs (at most 50).
    #[arg(long)]
    model: Option<String>,
    #[arg(long, value_parser = ["none", "model", "date", "model,date"])]
    group_by: Option<String>,
    /// IANA time zone for date grouping (server default Asia/Shanghai).
    #[arg(long)]
    tz: Option<String>,
    /// Whole account instead of the current key.
    #[arg(long)]
    account: bool,
    #[arg(long)]
    json: bool,
}
#[derive(Args)]
struct LogsArgs {
    /// Request ID or task ID; returns one record.
    #[arg(conflicts_with_all = ["start", "end", "kind", "model", "limit", "cursor", "token_id"])]
    id: Option<String>,
    /// Range start in unix seconds (default: 24 hours ago).
    #[arg(long, alias = "since")]
    start: Option<u64>,
    /// Range end in unix seconds.
    #[arg(long)]
    end: Option<u64>,
    #[arg(long = "type", value_parser = ["all", "consume", "error"])]
    kind: Option<String>,
    #[arg(long)]
    model: Option<String>,
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=1000))]
    limit: Option<u32>,
    /// Pagination cursor from a previous next_cursor (requires --start).
    #[arg(long, requires = "start")]
    cursor: Option<String>,
    /// Filter by API token (key) ID. Logs cover the whole account, not just this key.
    #[arg(long)]
    token_id: Option<String>,
    #[arg(long)]
    json: bool,
}
#[derive(Args)]
struct Fetch {
    task_id: String,
    #[arg(long,value_parser=clap::value_parser!(u8).range(1..=4))]
    n: Option<u8>,
    #[command(flatten)]
    output: Output,
}
#[derive(Args)]
struct Batch {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    out_dir: PathBuf,
    #[arg(long)]
    model: Option<String>,
    #[arg(long)]
    concurrency: Option<usize>,
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    force: bool,
    #[arg(long,value_parser=clap::value_parser!(u64).range(1..=86400))]
    timeout: Option<u64>,
    #[arg(long,value_parser=clap::value_parser!(u64).range(1..=60))]
    poll_interval: Option<u64>,
}
#[derive(Args)]
struct Chroma {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    out: PathBuf,
    #[arg(long, default_value = "00ff00")]
    key: String,
    #[arg(long, default_value_t = 30.0)]
    tolerance: f32,
    #[arg(long, default_value_t = 60.0)]
    softness: f32,
    #[arg(long)]
    despill: bool,
    #[arg(long)]
    force: bool,
}
fn emit(v: &Value) -> Result<()> {
    let mut out = io::stdout().lock();
    serde_json::to_writer_pretty(&mut out, v)?;
    writeln!(out)?;
    Ok(())
}
fn options(out: &Output, dry_run: bool, dir: &Path, conf: &Config) -> RunOptions {
    RunOptions {
        dry_run,
        force: out.force,
        timeout: out.timeout.or(conf.defaults.timeout).unwrap_or(600),
        interval: out
            .poll_interval
            .or(conf.defaults.poll_interval)
            .unwrap_or(4),
        config_dir: dir.into(),
    }
}
fn model_env() -> Option<String> {
    std::env::var("APIMART_IMAGE_MODEL")
        .ok()
        .filter(|s| !s.is_empty())
}
fn api_for(dir: &Path) -> Result<Api> {
    Api::with_base(&config::resolve_api_base(dir)?)
}
fn from_args(args: &Generate) -> Result<Input> {
    Ok(Input {
        prompt: match &args.prompt_file {
            Some(p) => Some(config::read_text(&config::expand(p)?)?),
            None => args.prompt.clone(),
        },
        model: args.model.clone(),
        aspect_ratio: args.aspect_ratio.clone(),
        size: args.size.clone(),
        resolution: args.resolution.clone(),
        quality: args.quality.clone(),
        background: args.background.clone(),
        output_format: args.output_format.clone(),
        output_compression: args.output_compression,
        moderation: args.moderation.clone(),
        n: args.n,
        image_urls: args.images.clone(),
        mask_url: args.mask.clone(),
        google_search: args.google_search,
        google_image_search: args.google_image_search,
        official_fallback: args.official_fallback,
        out: None,
    })
}
fn run_image(args: &Generate, edit: bool, dir: &Path, conf: &Config) -> Result<()> {
    if edit && args.images.is_empty() {
        bail!("edit requires at least one --image");
    }
    let payload = models::build(
        &from_args(args)?,
        conf,
        model_env().as_deref(),
        args.use_default_size,
    )?;
    let paths = jobs::paths(
        args.output.out.as_deref(),
        args.output.out_dir.as_deref(),
        payload["n"].as_u64().unwrap_or(1) as u8,
        payload["output_format"].as_str().unwrap_or("png"),
    )?;
    jobs::preflight(&paths, args.output.force, !args.dry_run)?;
    let api = if args.dry_run {
        None
    } else {
        Some(api_for(dir)?)
    };
    emit(&jobs::execute(
        api.as_ref(),
        &Job { payload, paths },
        &options(&args.output, args.dry_run, dir, conf),
    )?)
}
fn run_fetch(args: &Fetch, dir: &Path, conf: &Config) -> Result<()> {
    let receipt = jobs::read_receipt(dir, &args.task_id)?;
    let paths = if args.output.out.is_none() && args.output.out_dir.is_none() && args.n.is_none() {
        receipt
            .as_ref()
            .map(|r| r.outputs.clone())
            .unwrap_or(jobs::paths(None, None, 1, "png")?)
    } else {
        jobs::paths(
            args.output.out.as_deref(),
            args.output.out_dir.as_deref(),
            args.n
                .unwrap_or(receipt.as_ref().map(|r| r.outputs.len() as u8).unwrap_or(1)),
            "png",
        )?
    };
    jobs::preflight(&paths, args.output.force, true)?;
    let api = api_for(dir)?;
    let key = config::require_key(dir)?;
    let opts = options(&args.output, false, dir, conf);
    let data = api.poll(&args.task_id, &key, opts.timeout, opts.interval)?;
    emit(
        &json!({"task_id":args.task_id,"outputs":jobs::save(&api,&data,&paths,args.output.force)?,"cost":data["cost"],"usage":data["usage"]}),
    )
}
fn batch_path(root: &Path, out: &str) -> Result<PathBuf> {
    let path = Path::new(out);
    if out.is_empty()
        || path.is_absolute()
        || path.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        bail!("Batch out must be a relative filename without parent traversal");
    }
    let mut result = root.to_path_buf();
    for part in path.components() {
        result.push(part);
        if std::fs::symlink_metadata(&result).is_ok_and(|m| m.file_type().is_symlink()) {
            bail!("Batch output may not traverse a symlink");
        }
    }
    if result == root {
        bail!("Batch out needs a filename");
    }
    Ok(result)
}
fn run_batch(args: &Batch, dir: &Path, conf: &Config) -> Result<()> {
    let concurrency = args.concurrency.or(conf.defaults.concurrency).unwrap_or(2);
    if !(1..=8).contains(&concurrency) {
        bail!("concurrency must be 1..8");
    }
    let root = config::expand(&args.out_dir)?;
    let text = config::read_text(&config::expand(&args.input)?)?;
    let mut prepared = Vec::new();
    for (line_no, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut input: Input = serde_json::from_str(line)
            .with_context(|| format!("Invalid JSONL job at line {}", line_no + 1))?;
        if input.model.is_none() {
            input.model = args.model.clone();
        }
        let payload = models::build(&input, conf, model_env().as_deref(), false)?;
        let fallback = format!("job-{}", prepared.len() + 1);
        let out = batch_path(&root, input.out.as_deref().unwrap_or(&fallback))?;
        let paths = jobs::paths(
            Some(&out),
            None,
            payload["n"].as_u64().unwrap_or(1) as u8,
            payload["output_format"].as_str().unwrap_or("png"),
        )?;
        prepared.push(Job { payload, paths });
    }
    if !(1..=500).contains(&prepared.len()) {
        bail!("Batch requires 1..500 jobs");
    }
    let all_paths = prepared
        .iter()
        .flat_map(|j| j.paths.clone())
        .collect::<Vec<_>>();
    jobs::preflight(&all_paths, args.force, !args.dry_run)?;
    let opts = RunOptions {
        dry_run: args.dry_run,
        force: args.force,
        timeout: args.timeout.or(conf.defaults.timeout).unwrap_or(600),
        interval: args
            .poll_interval
            .or(conf.defaults.poll_interval)
            .unwrap_or(4),
        config_dir: dir.into(),
    };
    if args.dry_run {
        return emit(
            &json!({"jobs":prepared.iter().map(|j|jobs::execute(None,j,&opts)).collect::<Result<Vec<_>>>()?}),
        );
    }
    config::require_key(dir)?;
    let api = api_for(dir)?;
    let next = AtomicUsize::new(0);
    let failures = AtomicUsize::new(0);
    let prepared = Arc::new(prepared);
    std::thread::scope(|scope| {
        for _ in 0..concurrency {
            let (api, prepared, next, failures, opts) = (&api, &prepared, &next, &failures, &opts);
            scope.spawn(move || {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(job) = prepared.get(index) else {
                        break;
                    };
                    match jobs::execute(Some(api), job, opts)
                        .and_then(|result| emit(&json!({"job":index+1,"result":result})))
                    {
                        Ok(()) => (),
                        Err(err) => {
                            failures.fetch_add(1, Ordering::Relaxed);
                            eprintln!("Job {} failed: {err:#}", index + 1);
                        }
                    }
                }
            });
        }
    });
    if failures.load(Ordering::Relaxed) > 0 {
        bail!(
            "{} job(s) failed; successful outputs remain. Resume tasks instead of repeating the batch.",
            failures.load(Ordering::Relaxed)
        );
    }
    Ok(())
}
fn print_price(info: &Value) {
    println!(
        "{}: {}",
        info["model"].as_str().unwrap_or(""),
        info["status"].as_str().unwrap_or("unavailable")
    );
    if let Some(rates) = info["pricing"]["effective_rates"].as_object() {
        println!(
            "  unit: {} | group: {}",
            info["pricing"]["unit"], info["pricing"]["group"]
        );
        for (key, value) in rates {
            println!("  {key}: {value}");
        }
    } else if info["legacy_quote"].is_object() {
        println!("  {}", info["legacy_quote"]);
    }
    println!(
        "  Public quote; account charges may differ.\n  {}",
        info["source"]
    );
}
fn run_catalog(live: bool, prices: bool, json_output: bool, dir: &Path) -> Result<()> {
    let api = if live || prices {
        Some(api_for(dir)?)
    } else {
        None
    };
    let mut rows = if live {
        api.as_ref()
            .context("Client required")?
            .catalog(&config::require_key(dir)?)?
    } else {
        json!(
            models::MODELS
                .iter()
                .map(|id| json!({"id":id,"category":"image","runnable":true}))
                .collect::<Vec<_>>()
        )
    };
    let mut failures = 0;
    for row in rows.as_array_mut().context("Expected models")? {
        if prices {
            row["price_info"] = match api
                .as_ref()
                .context("Client required")?
                .pricing(row["id"].as_str().unwrap_or(""))
            {
                Ok(price) => price,
                Err(err) => {
                    failures += 1;
                    json!({"status":"unavailable","error":err.to_string()})
                }
            };
        }
        if !json_output {
            println!(
                "{} [{}]",
                row["id"].as_str().unwrap_or(""),
                if row["runnable"] == true {
                    "runnable"
                } else {
                    "discovery only"
                }
            );
            if prices {
                print_price(&row["price_info"]);
            }
        }
    }
    if json_output {
        emit(&json!({"models":rows,"price_failures":failures}))?;
    }
    if failures > 0 {
        bail!("{failures} price lookups failed; unknown does not mean free");
    }
    Ok(())
}
fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "-".into(),
        other => other.to_string(),
    }
}
fn usd(v: &Value) -> String {
    v.as_f64().map_or_else(
        || text(v),
        |n| {
            let s = format!("{n:.6}");
            let s = s.trim_end_matches('0');
            let s = if s.ends_with('.') {
                format!("{s}00")
            } else {
                s.to_string()
            };
            let decimals = s.len() - s.find('.').map_or(s.len(), |i| i + 1);
            if decimals < 2 {
                format!("${s}0")
            } else {
                format!("${s}")
            }
        },
    )
}
fn run_balance(account: bool, as_json: bool, dir: &Path) -> Result<()> {
    let key = config::require_key(dir)?;
    let path = if account {
        ["v1", "user", "balance"].as_slice()
    } else {
        ["v1", "balance"].as_slice()
    };
    let v = api_for(dir)?.account_get(path, &[], &key)?;
    if as_json {
        return emit(&v);
    }
    let data = v.get("data").filter(|d| d.is_object()).unwrap_or(&v);
    let scope = if account { "account" } else { "key" };
    if data["unlimited_quota"] == true {
        println!("Remaining ({scope}): unlimited");
    } else {
        println!(
            "Remaining ({scope}): {} ({} credits)",
            usd(&data["remain_balance"]),
            text(&data["remain_credits"])
        );
    }
    println!(
        "Used ({scope}): {} ({} credits)",
        usd(&data["used_balance"]),
        text(&data["used_credits"])
    );
    Ok(())
}
fn run_usage(args: &UsageArgs, dir: &Path) -> Result<()> {
    let key = config::require_key(dir)?;
    let mut query = Vec::new();
    for (name, value) in [
        ("start", &args.start),
        ("end", &args.end),
        ("model", &args.model),
        ("group_by", &args.group_by),
        ("tz", &args.tz),
    ] {
        if let Some(v) = value {
            query.push((name, v.clone()));
        }
    }
    if args.account {
        query.push(("scope", "account".into()));
    }
    let v = api_for(dir)?.account_get(&["v1", "usage"], &query, &key)?;
    if args.json {
        return emit(&v);
    }
    let data = v.get("data").filter(|d| d.is_object()).unwrap_or(&v);
    println!(
        "Usage ({}) {} .. {} tz={} group_by={}",
        text(&data["scope"]),
        text(&data["start"]),
        text(&data["end"]),
        text(&data["tz"]),
        text(&data["group_by"])
    );
    let total = &data["total"];
    println!(
        "Total: {} | {} credits | {} requests | {} prompt + {} completion tokens",
        usd(&total["amount_usd"]),
        text(&total["credits"]),
        text(&total["requests"]),
        text(&total["prompt_tokens"]),
        text(&total["completion_tokens"])
    );
    if let Some(items) = data["items"].as_array().filter(|i| !i.is_empty()) {
        println!("date\tmodel\tusd\tcredits\trequests\tprompt\tcompletion");
        for item in items {
            println!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                text(&item["date"]),
                text(&item["model"]),
                usd(&item["amount_usd"]),
                text(&item["credits"]),
                text(&item["requests"]),
                text(&item["prompt_tokens"]),
                text(&item["completion_tokens"])
            );
        }
    }
    Ok(())
}
fn print_log(r: &Value) {
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}/{}\t{}\t{}ms",
        text(&r["id"]),
        text(&r["created_at"]),
        text(&r["type"]),
        text(&r["status"]),
        text(&r["model_name"]),
        text(&r["prompt_tokens"]),
        text(&r["completion_tokens"]),
        usd(&r["cost_usd"]),
        text(&r["latency_ms"])
    );
}
fn run_logs(args: &LogsArgs, dir: &Path) -> Result<()> {
    let key = config::require_key(dir)?;
    let api = api_for(dir)?;
    if let Some(id) = &args.id {
        if id.is_empty()
            || id == "."
            || id == ".."
            || !id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_.:-".contains(c))
        {
            bail!("Invalid log ID: use a request or task ID of letters, digits and _ . : - only");
        }
        let v = api
            .account_get(&["v1", "logs", "requests", id], &[], &key)
            .map_err(|e| {
                if e.to_string().contains("HTTP 404") {
                    e.context("Logs appear a few seconds after a request completes, and tasks only once finished; retry later")
                } else {
                    e
                }
            })?;
        if args.json {
            return emit(&v);
        }
        let rows: Vec<&Value> = match v.get("data") {
            Some(Value::Array(a)) => a.iter().collect(),
            Some(d) if d.is_object() => vec![d],
            _ => vec![&v],
        };
        println!("id\tcreated_at\ttype\tstatus\tmodel\ttokens in/out\tcost\tlatency");
        for r in rows {
            print_log(r);
        }
        return Ok(());
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let start = args.start.unwrap_or(now.saturating_sub(86400));
    let end = args.end.unwrap_or(now);
    let mut query = vec![
        ("start_timestamp", start.to_string()),
        ("end_timestamp", end.to_string()),
    ];
    if let Some(t) = &args.token_id {
        query.push(("token_id", t.clone()));
    }
    for (name, value) in [
        ("type", &args.kind),
        ("model", &args.model),
        ("cursor", &args.cursor),
    ] {
        if let Some(v) = value {
            query.push((name, v.clone()));
        }
    }
    if let Some(limit) = args.limit {
        query.push(("limit", limit.to_string()));
    }
    let v = api.account_get(&["v1", "logs", "requests"], &query, &key)?;
    if args.json {
        return emit(&v);
    }
    let rows = v["data"]
        .as_array()
        .context("Logs returned no data array")?;
    println!("id\tcreated_at\ttype\tstatus\tmodel\ttokens in/out\tcost\tlatency");
    for r in rows {
        print_log(r);
    }
    println!("{} record(s)", rows.len());
    if v["has_more"] == true {
        println!(
            "More: --start {start} --end {end} --cursor {}",
            text(&v["next_cursor"])
        );
    }
    Ok(())
}
fn run_chroma(args: &Chroma) -> Result<()> {
    if !args.tolerance.is_finite()
        || !args.softness.is_finite()
        || args.tolerance < 0.0
        || args.softness <= 0.0
    {
        bail!("tolerance must be >=0 and softness >0, both finite");
    }
    let key = args.key.trim_start_matches('#');
    if key.len() != 6 || !key.is_ascii() {
        bail!("Key must be a six-digit RGB hex color");
    }
    let k = (0..3)
        .map(|i| u8::from_str_radix(&key[i * 2..i * 2 + 2], 16))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let input = config::expand(&args.input)?;
    let out = config::expand(&args.out)?;
    if out
        .extension()
        .is_none_or(|e| !e.eq_ignore_ascii_case("png"))
    {
        bail!("Chroma-key output must end in .png");
    }
    jobs::preflight(std::slice::from_ref(&out), args.force, true)?;
    let mut rgba = image::open(input)?.to_rgba8();
    let key_channel = (0..3).max_by_key(|i| k[*i]).unwrap_or(1);
    for pixel in rgba.pixels_mut() {
        let distance = (0..3)
            .map(|i| (pixel[i] as f32 - k[i] as f32).powi(2))
            .sum::<f32>()
            .sqrt();
        let matte = ((distance - args.tolerance) / args.softness).clamp(0.0, 1.0);
        pixel[3] = (pixel[3] as f32 * matte).round() as u8;
        if args.despill && matte < 1.0 {
            let other = (0..3)
                .filter(|i| *i != key_channel)
                .map(|i| pixel[i])
                .max()
                .unwrap_or(0);
            pixel[key_channel] = pixel[key_channel].min(other);
        }
    }
    let full = jobs::absolute(&out)?;
    let mut temp = tempfile::NamedTempFile::new_in(full.parent().context("Output needs parent")?)?;
    rgba.write_to(temp.as_file_mut(), image::ImageFormat::Png)?;
    if args.force {
        temp.persist(&full).map_err(|e| e.error)?;
    } else {
        temp.persist_noclobber(&full).map_err(|e| e.error)?;
    }
    emit(&json!({"output":full}))
}
fn run() -> Result<()> {
    let cli = Cli::parse();
    let dir = config::config_dir()?;
    // Locate/reset broken config and manage secrets without needing valid defaults.
    match &cli.command {
        Command::Config {
            command: ConfigCommand::Path,
        } => {
            println!("{}", dir.join("config.toml").display());
            return Ok(());
        }
        Command::Config {
            command: ConfigCommand::Reset,
        } => {
            Config::default().save(&dir)?;
            println!("Defaults reset; API key retained.");
            return Ok(());
        }
        Command::Auth {
            command: AuthCommand::Set,
        } => {
            if !io::stdin().is_terminal() {
                bail!("auth set requires an interactive terminal. Use APIMART_API_KEY for CI.");
            }
            let key = rpassword::prompt_password("APIMART API key (hidden): ")?;
            if key.trim().is_empty() || key.contains(['\r', '\n']) {
                bail!("Key must be non-empty and single-line");
            }
            config::private_write(
                &dir.join("apimart.key"),
                format!("{}\n", key.trim()).as_bytes(),
            )?;
            println!("Key saved outside the project. Next: imagegen auth check.");
            return Ok(());
        }
        Command::Auth {
            command: AuthCommand::Check,
        } => {
            let (key, _) = config::resolve_key(&dir)?.with_context(|| format!("Key missing. Register at {}; get a key at https://apimart.ai/keys; run imagegen auth set.", config::SIGNUP))?;
            api_for(&dir)?.check_auth(&key)?;
            println!(
                "Authentication verified: model-list access allowed. No image submitted. Balance and generation permissions are not verified."
            );
            return Ok(());
        }
        Command::Auth {
            command: AuthCommand::Status,
        }
        | Command::CheckKey => {
            let (_,source) = config::resolve_key(&dir)?.with_context(||format!("Key missing. Register at {}; get a key at https://apimart.ai/keys; run imagegen auth set.",config::SIGNUP))?;
            println!("API key configured via {source}. Local check only.");
            return Ok(());
        }
        _ => (),
    }
    let mut conf = Config::load(&dir)?;
    match cli.command {
        Command::Generate(args) => run_image(&args, false, &dir, &conf),
        Command::Edit(args) => run_image(&args, true, &dir, &conf),
        Command::Fetch(args) => run_fetch(&args, &dir, &conf),
        Command::Batch(args) => run_batch(&args, &dir, &conf),
        Command::Models { live, prices, json } => run_catalog(live, prices, json, &dir),
        Command::Prices {
            model,
            json: as_json,
        } => {
            let model = model
                .or(model_env())
                .or(conf.defaults.model.clone())
                .unwrap_or(config::DEFAULT_MODEL.into());
            let info = api_for(&dir)?.pricing(&model)?;
            if as_json {
                emit(&info)
            } else {
                print_price(&info);
                Ok(())
            }
        }
        Command::Config {
            command: ConfigCommand::Show,
        } => emit(
            &json!({"path":dir.join("config.toml"),"effective_api_base":config::resolve_api_base(&dir)?,"saved":conf,"effective_model":model_env().or(conf.defaults.model.clone()).unwrap_or(config::DEFAULT_MODEL.into()),"built_in_defaults":{"aspect_ratio":"1:1","resolution":"1k","quality":"medium (GPT Image 2.5)","output_format":"png","n":1,"timeout":600,"poll_interval":4,"concurrency":2}}),
        ),
        Command::Config {
            command: ConfigCommand::Set { key, value },
        } => {
            conf.set(&key, Some(&value))?;
            conf.save(&dir)?;
            println!("Saved {key}={value}");
            Ok(())
        }
        Command::Config {
            command: ConfigCommand::Unset { key },
        } => {
            conf.set(&key, None)?;
            conf.save(&dir)?;
            println!("Unset {key}");
            Ok(())
        }
        Command::Doctor => emit(
            &json!({"version":env!("CARGO_PKG_VERSION"),"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"config_dir":dir,"key_configured":config::resolve_key(&dir)?.is_some(),"api":config::resolve_api_base(&dir)?,"runtime_dependencies":"none"}),
        ),
        Command::Balance { account, json } => run_balance(account, json, &dir),
        Command::Usage(args) => run_usage(&args, &dir),
        Command::Logs(args) => run_logs(&args, &dir),
        Command::RemoveChromaKey(args) => run_chroma(&args),
        _ => unreachable!("handled before configuration load"),
    }
}
fn main() {
    if let Err(err) = run() {
        if err
            .downcast_ref::<io::Error>()
            .is_some_and(|e| e.kind() == io::ErrorKind::BrokenPipe)
        {
            return;
        }
        eprintln!("Error: {err:#}");
        std::process::exit(1);
    }
}
