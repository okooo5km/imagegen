use imagegen::{
    api::{Api, result_urls},
    config,
    jobs::{self, Job, RunOptions},
    models::{self, Input},
};
use serde_json::json;
use std::{
    io::Cursor,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tiny_http::{Header, Response, Server};
#[derive(Clone)]
struct Call {
    method: String,
    path: String,
    auth: bool,
    body: Vec<u8>,
}
struct Fixture {
    base: String,
    calls: Arc<Mutex<Vec<Call>>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
fn fixture_bin(root: &std::path::Path, base: &str, args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_imagegen"))
        .args(args)
        .current_dir(root)
        .env("IMAGEGEN_CONFIG_DIR", root.join("config"))
        .env("APIMART_API_BASE", base)
        .env("APIMART_API_KEY", "fake-key")
        .env_remove("APIMART_IMAGE_MODEL")
        .output()
        .expect("run binary")
}
impl Fixture {
    fn new() -> Self {
        let server = Server::http("127.0.0.1:0").unwrap();
        let base = format!("http://{}", server.server_addr());
        let calls = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&calls);
        let stop = Arc::new(AtomicBool::new(false));
        let done = Arc::clone(&stop);
        let url = base.clone();
        let mut png = Cursor::new(Vec::new());
        image::RgbaImage::new(16, 16)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let bytes = png.into_inner();
        let thread = std::thread::spawn(move || {
            while !done.load(Ordering::Relaxed) {
                let Some(mut request) = server.recv_timeout(Duration::from_millis(50)).unwrap()
                else {
                    continue;
                };
                let path = request.url().to_owned();
                let method = request.method().as_str().to_owned();
                let auth = request
                    .headers()
                    .iter()
                    .any(|h| h.field.equiv("Authorization"));
                let mut body = Vec::new();
                request.as_reader().read_to_end(&mut body).unwrap();
                log.lock().unwrap().push(Call {
                    method,
                    path: path.clone(),
                    auth,
                    body,
                });
                let (status, data) = if path == "/v1/uploads/images" {
                    (200, json!({"url":format!("{url}/image.png")}))
                } else if path == "/v1/images/generations" {
                    (200, json!({"data":[{"task_id":"test-task"}]}))
                } else if path == "/v1/tasks/test-task" {
                    (
                        200,
                        json!({"data":{"status":"completed","cost":0.01,"result":{"images":[{"url":[format!("{url}/image.png")]}]}}}),
                    )
                } else if path == "/v1/tasks/pending" {
                    (200, json!({"data":{"status":"processing"}}))
                } else if path == "/v1/tasks/failed" {
                    (200, json!({"data":{"status":"failed"}}))
                } else if path.starts_with("/api/pricing/model") {
                    (
                        200,
                        json!({"data":{"token_price":{"output":999},"pricing":{"unit":"usd_per_million_tokens","effective_rates":{"image_output":24,"cached_text_input":0},"rates":{"image_output":30},"group_ratio":0.8}}}),
                    )
                } else if path == "/v1/balance" {
                    (
                        200,
                        json!({"success":true,"remain_balance":12.5,"remain_credits":1250,"used_balance":3.25,"used_credits":325,"unlimited_quota":false}),
                    )
                } else if path == "/v1/user/balance" {
                    (
                        200,
                        json!({"success":true,"remain_balance":99.0,"remain_credits":9900,"used_balance":1.0,"used_credits":100}),
                    )
                } else if path.starts_with("/v1/usage") {
                    (
                        200,
                        json!({"success":true,"data":{"scope":"key","start":1,"end":2,"tz":"Asia/Shanghai","group_by":"model","total":{"amount_usd":0.5,"credits":50,"requests":2,"prompt_tokens":10,"completion_tokens":20},"items":[{"model":"gpt-image-2.5-flare","amount_usd":0.5,"credits":50,"requests":2,"prompt_tokens":10,"completion_tokens":20}]}}),
                    )
                } else if path.starts_with("/v1/logs/requests/") {
                    (
                        200,
                        json!({"data":[{"id":"req-1","type":"consume","status":"ok","created_at":1700000000,"model_name":"m","prompt_tokens":1,"completion_tokens":2,"cost_usd":0.004125,"latency_ms":900},{"id":"req-1","type":"error","status":"failed","created_at":1700000001,"model_name":"m","prompt_tokens":1,"completion_tokens":0,"cost_usd":0,"latency_ms":10}]}),
                    )
                } else if path.starts_with("/v1/logs/requests") {
                    (
                        200,
                        json!({"object":"list","data":[{"id":"r1","type":"consume","status":"ok","created_at":1700000000,"model_name":"m","prompt_tokens":1,"completion_tokens":2,"cost_usd":0.01,"latency_ms":900}],"has_more":true,"next_cursor":"CUR1"}),
                    )
                } else if path.starts_with("/v1/models") {
                    (
                        200,
                        json!({"data":[{"id":"gpt-image-2.5-flare","category":"image"},{"id":"future-image","category":"image"},{"id":"chat","category":"chat"}]}),
                    )
                } else if path == "/image.png" {
                    request.respond(Response::from_data(bytes.clone())).unwrap();
                    continue;
                } else if path.starts_with("/redirect") {
                    request
                        .respond(Response::empty(302).with_header(
                            Header::from_bytes("Location", format!("{url}/leaked")).unwrap(),
                        ))
                        .unwrap();
                    continue;
                } else {
                    (500, json!({"error":"reflected-secret"}))
                };
                request
                    .respond(Response::from_string(data.to_string()).with_status_code(status))
                    .unwrap();
            }
        });
        Self {
            base,
            calls,
            stop,
            thread: Some(thread),
        }
    }
    fn api(&self) -> Api {
        Api::with_base(&self.base).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}
#[test]
fn multipart_submit_poll_download() {
    let f = Fixture::new();
    let api = f.api();
    let t = tempfile::tempdir().unwrap();
    let input = t.path().join("输入 图.png");
    image::RgbaImage::new(16, 16).save(&input).unwrap();
    let url = api.upload(input.to_str().unwrap(), "fake").unwrap();
    let id = api
        .submit(&json!({"prompt":"test","image_urls":[url]}), "fake")
        .unwrap();
    assert_eq!(id, "test-task");
    let data = api.poll(&id, "fake", 5, 1).unwrap();
    let paths = vec![t.path().join("result.webp")];
    let saved = jobs::save(&api, &data, &paths, false).unwrap();
    assert_eq!(saved[0].extension().unwrap(), "png");
    image::open(&saved[0]).unwrap();
    let calls = f.calls.lock().unwrap();
    assert_eq!(calls.iter().filter(|c| c.method == "POST").count(), 2);
    let upload = calls
        .iter()
        .find(|c| c.path == "/v1/uploads/images")
        .unwrap();
    assert!(String::from_utf8_lossy(&upload.body).contains("name=\"file\""));
    assert!(upload.auth);
    assert!(!calls.iter().find(|c| c.path == "/image.png").unwrap().auth);
}
#[test]
fn prices_use_effective_values_without_authorization() {
    let f = Fixture::new();
    let info = f.api().pricing("gpt-image-2.5-flare").unwrap();
    assert_eq!(info["pricing"]["effective_rates"]["image_output"], 24);
    assert_eq!(info["pricing"]["effective_rates"]["cached_text_input"], 0);
    assert!(
        info["pricing"]["effective_rates"]
            .get("text_input")
            .is_none()
    );
    assert!(!f.calls.lock().unwrap()[0].auth);
}
#[test]
fn catalog_distinguishes_runnable() {
    let f = Fixture::new();
    let rows = f.api().catalog("fake").unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 2);
    assert_eq!(rows[0]["runnable"], true);
    assert_eq!(rows[1]["runnable"], false);
}
#[test]
fn errors_do_not_retry_or_echo_body() {
    let f = Fixture::new();
    let api = Api::with_base(&format!("{}/bad", f.base)).unwrap();
    let e = api.submit(&json!({"prompt":"test"}), "fake").unwrap_err();
    assert!(!format!("{e:#}").contains("reflected-secret"));
    assert_eq!(f.calls.lock().unwrap().len(), 1);
}
#[test]
fn authenticated_redirects_are_not_followed() {
    let f = Fixture::new();
    let api = Api::with_base(&format!("{}/redirect", f.base)).unwrap();
    assert!(api.submit(&json!({}), "fake").is_err());
    assert!(!f.calls.lock().unwrap().iter().any(|c| c.path == "/leaked"));
}
#[test]
fn pending_and_failed_tasks_keep_identity() {
    let f = Fixture::new();
    let api = f.api();
    let e = api.poll("pending", "fake", 1, 1).unwrap_err();
    assert!(format!("{e:#}").contains("imagegen fetch pending"));
    assert!(api.poll("failed", "fake", 1, 1).is_err());
    assert!(!f.calls.lock().unwrap().iter().any(|c| c.method == "POST"));
}
#[test]
fn result_url_shapes() {
    assert_eq!(result_urls(&json!({"result":{"images":[{"url":"https://example.com/a.png"},{"url":["https://example.com/b.png"]}]}})).unwrap().len(),2);
    assert!(result_urls(&json!({})).is_err());
}
#[test]
fn task_receipt_and_recovery() {
    let f = Fixture::new();
    let t = tempfile::tempdir().unwrap();
    config::private_write(&t.path().join("apimart.key"), b"fake-key\n").unwrap();
    let payload = models::build(
        &Input {
            prompt: Some("private prompt".into()),
            ..Input::default()
        },
        &config::Config::default(),
        None,
        false,
    )
    .unwrap();
    let out = t.path().join("out.png");
    let opts = RunOptions {
        dry_run: false,
        force: false,
        timeout: 5,
        interval: 1,
        config_dir: t.path().into(),
    };
    let result = jobs::execute(
        Some(&f.api()),
        &Job {
            payload,
            paths: vec![out.clone()],
        },
        &opts,
    )
    .unwrap();
    assert_eq!(result["task_id"], "test-task");
    let receipt = jobs::read_receipt(t.path(), "test-task").unwrap().unwrap();
    assert_eq!(receipt.outputs, vec![out.clone()]);
    let raw = std::fs::read_to_string(jobs::receipt_path(t.path(), "test-task")).unwrap();
    assert!(!raw.contains("private prompt"));
    assert!(!raw.contains("fake-key"));
    let before = f
        .calls
        .lock()
        .unwrap()
        .iter()
        .filter(|c| c.method == "POST")
        .count();
    std::fs::remove_file(&out).unwrap();
    let data = f.api().poll("test-task", "fake", 5, 1).unwrap();
    jobs::save(&f.api(), &data, &receipt.outputs, false).unwrap();
    assert_eq!(
        f.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.method == "POST")
            .count(),
        before
    );
}

#[test]
fn auth_check_is_read_only_and_classifies_failures() {
    for (status, body, expected) in [
        (200, r#"{"data":[]}"#, None),
        (401, r#"{"error":"secret-sentinel"}"#, Some("HTTP 401")),
        (403, "secret-sentinel", Some("HTTP 403")),
        (429, "secret-sentinel", Some("HTTP 429")),
        (503, "secret-sentinel", Some("HTTP 503")),
        (302, "secret-sentinel", Some("HTTP 302")),
        (
            200,
            r#"{"data":[],"success":false}"#,
            Some("did not confirm"),
        ),
        (
            200,
            "secret-sentinel",
            Some("Unexpected authentication response"),
        ),
    ] {
        let server = Server::http("127.0.0.1:0").unwrap();
        let base = format!("http://{}", server.server_addr());
        let thread = std::thread::spawn(move || {
            let request = server
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .unwrap();
            assert_eq!(request.method().as_str(), "GET");
            assert_eq!(request.url(), "/v1/models");
            assert!(request.headers().iter().any(
                |h| h.field.equiv("Authorization") && h.value.as_str() == "Bearer fixture-key"
            ));
            request
                .respond(Response::from_string(body).with_status_code(status))
                .unwrap();
        });
        let result = Api::with_base(&base).unwrap().check_auth("fixture-key");
        match expected {
            None => result.unwrap(),
            Some(message) => {
                let error = result.unwrap_err().to_string();
                assert!(error.contains(message), "{error}");
                assert!(!error.contains("secret-sentinel"));
                assert!(!error.contains("fixture-key"));
            }
        }
        thread.join().unwrap();
    }
    let error = Api::with_base("http://[invalid")
        .unwrap()
        .check_auth("fixture-key")
        .unwrap_err()
        .to_string();
    assert!(error.contains("network"), "{error}");
    assert!(!error.contains("fixture-key"));
}

#[test]
fn cli_uses_custom_api_base_end_to_end() {
    let f = Fixture::new();
    let t = tempfile::tempdir().unwrap();
    let out = t.path().join("custom-base.webp");
    let o = fixture_bin(
        t.path(),
        &f.base,
        &[
            "generate",
            "--prompt",
            "endpoint check",
            "--out",
            out.to_str().unwrap(),
            "--poll-interval",
            "1",
            "--timeout",
            "10",
        ],
    );
    assert!(
        o.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    let payload: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(payload["task_id"], "test-task");
    let saved = payload["outputs"][0].as_str().unwrap();
    assert!(std::path::Path::new(saved).exists(), "missing {saved}");
    let calls = f.calls.lock().unwrap();
    assert!(
        calls.iter().any(|c| c.path == "/v1/images/generations"),
        "submission did not reach the custom endpoint: {:?}",
        calls.iter().map(|c| c.path.clone()).collect::<Vec<_>>()
    );
    assert!(calls.iter().any(|c| c.path == "/v1/tasks/test-task"));
}

fn one_shot(body: &'static str, status: u16) -> (String, std::thread::JoinHandle<()>) {
    let server = Server::http("127.0.0.1:0").unwrap();
    let base = format!("http://{}", server.server_addr());
    let thread = std::thread::spawn(move || {
        let request = server
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap();
        request
            .respond(Response::from_string(body).with_status_code(status))
            .unwrap();
    });
    (base, thread)
}
#[test]
fn balance_cli_prints_and_calls_expected_endpoints() {
    let f = Fixture::new();
    let t = tempfile::tempdir().unwrap();
    let o = fixture_bin(t.path(), &f.base, &["balance"]);
    assert!(o.status.success());
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(
        out.contains("$12.50") && out.contains("1250 credits"),
        "{out}"
    );
    assert!(out.contains("$3.25"), "{out}");
    let o = fixture_bin(t.path(), &f.base, &["balance", "--account", "--json"]);
    assert!(o.status.success());
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["remain_credits"], 9900);
    let calls = f.calls.lock().unwrap();
    assert_eq!(calls[0].path, "/v1/balance");
    assert_eq!(calls[1].path, "/v1/user/balance");
    assert!(calls.iter().all(|c| c.method == "GET" && c.auth));
}
#[test]
fn balance_unlimited_quota_and_success_false() {
    let (base, thread) = one_shot(
        r#"{"success":true,"remain_balance":-1,"remain_credits":-1,"used_balance":2,"used_credits":20,"unlimited_quota":true}"#,
        200,
    );
    let t = tempfile::tempdir().unwrap();
    let o = fixture_bin(t.path(), &base, &["balance"]);
    thread.join().unwrap();
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(
        o.status.success() && out.contains("unlimited") && !out.contains("-1"),
        "{out}"
    );
    let (base, thread) = one_shot(r#"{"success":false,"message":"secret-sentinel"}"#, 200);
    let o = fixture_bin(t.path(), &base, &["balance"]);
    thread.join().unwrap();
    assert!(!o.status.success());
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("success=false"), "{err}");
    assert!(!err.contains("secret-sentinel") && !err.contains("fake-key"));
    for (status, expected) in [(401, "HTTP 401"), (429, "HTTP 429")] {
        let (base, thread) = one_shot(
            r#"{"error":{"code":401,"message":"secret-sentinel"}}"#,
            status,
        );
        let o = fixture_bin(t.path(), &base, &["balance"]);
        thread.join().unwrap();
        let err = String::from_utf8_lossy(&o.stderr);
        assert!(!o.status.success() && err.contains(expected), "{err}");
        assert!(!err.contains("secret-sentinel"));
    }
}
#[test]
fn usage_sends_query_parameters() {
    let f = Fixture::new();
    let t = tempfile::tempdir().unwrap();
    let o = fixture_bin(
        t.path(),
        &f.base,
        &[
            "usage",
            "--start",
            "1700000000",
            "--end",
            "2024-01-02T00:00:00Z",
            "--model",
            "a,b",
            "--group-by",
            "model,date",
            "--tz",
            "UTC",
            "--account",
        ],
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(
        out.contains("Total: $0.50") && out.contains("gpt-image-2.5-flare"),
        "{out}"
    );
    let path = f.calls.lock().unwrap()[0].path.clone();
    let url = reqwest::Url::parse(&format!("http://x{path}")).unwrap();
    assert_eq!(url.path(), "/v1/usage");
    let q: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(q["start"], "1700000000");
    assert_eq!(q["end"], "2024-01-02T00:00:00Z");
    assert_eq!(q["model"], "a,b");
    assert_eq!(q["group_by"], "model,date");
    assert_eq!(q["tz"], "UTC");
    assert_eq!(q["scope"], "account");
    let bad = fixture_bin(t.path(), &f.base, &["usage", "--group-by", "week"]);
    assert_eq!(bad.status.code(), Some(2));
}
#[test]
fn logs_default_start_cursor_and_by_id() {
    let f = Fixture::new();
    let t = tempfile::tempdir().unwrap();
    let o = fixture_bin(
        t.path(),
        &f.base,
        &["logs", "--type", "error", "--limit", "5"],
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(
        out.contains("--start ") && out.contains("--end ") && out.contains("--cursor CUR1"),
        "{out}"
    );
    let o = fixture_bin(t.path(), &f.base, &["logs", "req-1"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(
        out.contains("consume") && out.contains("error") && out.contains("$0.004125"),
        "{out}"
    );
    for bad in [["logs", ".."], ["logs", "a b"]] {
        assert!(!fixture_bin(t.path(), &f.base, &bad).status.success());
    }
    let o = fixture_bin(t.path(), &f.base, &["logs", "--cursor", "x"]);
    assert_eq!(o.status.code(), Some(2));
    let calls = f.calls.lock().unwrap();
    let url = reqwest::Url::parse(&format!("http://x{}", calls[0].path)).unwrap();
    assert_eq!(url.path(), "/v1/logs/requests");
    let q: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
    let start: u64 = q["start_timestamp"].parse().unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!(now - start >= 86390 && now - start <= 86500);
    assert_eq!((q["type"].as_str(), q["limit"].as_str()), ("error", "5"));
    assert_eq!(calls[1].path, "/v1/logs/requests/req-1");
}
