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
