use imagegen::{
    config::{Config, Defaults},
    jobs,
    models::{self, Input},
};
use serde_json::{Value, json};
use std::{
    path::Path,
    process::{Command, Output},
};
fn cli(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_imagegen"))
        .args(args)
        .current_dir(root)
        .env("IMAGEGEN_CONFIG_DIR", root.join("config"))
        .env_remove("APIMART_API_KEY")
        .env_remove("APIMART_IMAGE_MODEL")
        .output()
        .expect("run binary")
}
fn ok(root: &Path, args: &[&str]) -> Output {
    let o = cli(root, args);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    o
}
fn payload(input: Input, conf: Config) -> Value {
    models::build(&input, &conf, None, false).unwrap()
}
fn prompt() -> Input {
    Input {
        prompt: Some("tea".into()),
        ..Input::default()
    }
}
#[test]
fn default_flare() {
    let p = payload(prompt(), Config::default());
    assert_eq!(p["model"], "gpt-image-2.5-flare");
    assert_eq!(p["quality"], "medium");
}
#[test]
fn saved_defaults_persist() {
    let t = tempfile::tempdir().unwrap();
    ok(t.path(), &["config", "set", "aspect_ratio", "16:9"]);
    ok(t.path(), &["config", "set", "resolution", "2k"]);
    let o = ok(t.path(), &["generate", "--prompt", "茶杯", "--dry-run"]);
    let j: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(j["payload"]["size"], "16:9");
    assert_eq!(j["payload"]["resolution"], "2k");
    assert!(!t.path().join("output").exists());
}
#[test]
fn explicit_geometry_overrides_saved_exact_size() {
    let conf = Config {
        defaults: Defaults {
            size: Some("1600x1200".into()),
            aspect_ratio: Some("16:9".into()),
            resolution: Some("2k".into()),
            ..Defaults::default()
        },
        ..Config::default()
    };
    let p = payload(prompt(), conf.clone());
    assert_eq!(p["size"], "1600x1200");
    assert!(p.get("resolution").is_none());
    let p = payload(
        Input {
            resolution: Some("4k".into()),
            ..prompt()
        },
        conf,
    );
    assert_eq!(p["size"], "16:9");
    assert_eq!(p["resolution"], "4k");
}
#[test]
fn model_precedence() {
    let conf = Config {
        defaults: Defaults {
            model: Some("gpt-image-2.5-sunburst".into()),
            ..Defaults::default()
        },
        ..Config::default()
    };
    assert_eq!(
        models::build(&prompt(), &conf, Some("gpt-image-2"), false).unwrap()["model"],
        "gpt-image-2"
    );
    let input = Input {
        model: Some("gpt-image-2.5-flare".into()),
        ..prompt()
    };
    assert_eq!(
        models::build(&input, &conf, Some("gpt-image-2"), false).unwrap()["model"],
        "gpt-image-2.5-flare"
    );
}
#[test]
fn edit_preserves_input_ratio() {
    let conf = Config {
        defaults: Defaults {
            size: Some("1600x1200".into()),
            ..Defaults::default()
        },
        ..Config::default()
    };
    let input = Input {
        image_urls: vec!["https://example.com/a.png".into()],
        ..prompt()
    };
    assert!(payload(input.clone(), conf.clone()).get("size").is_none());
    assert_eq!(
        models::build(&input, &conf, None, true).unwrap()["size"],
        "1600x1200"
    );
}
#[test]
fn config_invalid_set_does_not_write() {
    let t = tempfile::tempdir().unwrap();
    assert!(
        !cli(t.path(), &["config", "set", "resolution", "8k"])
            .status
            .success()
    );
    assert!(!t.path().join("config/config.toml").exists());
}
#[test]
fn unset_and_reset_keep_key() {
    let t = tempfile::tempdir().unwrap();
    ok(t.path(), &["config", "set", "n", "3"]);
    std::fs::write(t.path().join("config/apimart.key"), "fake-key").unwrap();
    ok(t.path(), &["config", "unset", "n"]);
    ok(t.path(), &["config", "reset"]);
    assert_eq!(
        std::fs::read_to_string(t.path().join("config/apimart.key")).unwrap(),
        "fake-key"
    );
}
#[test]
fn legacy_json_and_bom_key() {
    let t = tempfile::tempdir().unwrap();
    std::fs::create_dir(t.path().join("config")).unwrap();
    std::fs::write(
        t.path().join("config/config.json"),
        "\u{feff}{\"default_model\":\"gpt-image-2.5-sunburst\"}",
    )
    .unwrap();
    std::fs::write(
        t.path().join("config/apimart.key"),
        "\u{feff}\r\nfake-private-key\r\n",
    )
    .unwrap();
    let o = ok(t.path(), &["auth", "status"]);
    assert!(!String::from_utf8_lossy(&o.stdout).contains("fake-private-key"));
    let o = ok(t.path(), &["generate", "--prompt", "test", "--dry-run"]);
    let j: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(j["payload"]["model"], "gpt-image-2.5-sunburst");
}
#[test]
fn no_key_and_noninteractive_auth_are_actionable() {
    let t = tempfile::tempdir().unwrap();
    let o = cli(t.path(), &["auth", "status"]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("imagegen auth set"));
    assert!(!cli(t.path(), &["auth", "set"]).status.success());
}
#[test]
fn exact_sizes_are_validated() {
    for s in ["1600x1200", "3840x2160", "auto"] {
        assert!(models::validate_size(s).is_ok());
    }
    for s in [
        "16:9",
        "1x1",
        "3840x3840",
        "1000x1000",
        "18446744073709551615x1",
    ] {
        assert!(models::validate_size(s).is_err());
    }
}
#[test]
fn reject_incompatible_parameters() {
    for input in [
        Input {
            background: Some("transparent".into()),
            output_format: Some("jpeg".into()),
            ..prompt()
        },
        Input {
            model: Some("gpt-image-2".into()),
            quality: Some("high".into()),
            ..prompt()
        },
        Input {
            google_search: Some(true),
            ..prompt()
        },
        Input {
            n: Some(5),
            ..prompt()
        },
    ] {
        assert!(models::build(&input, &Config::default(), None, false).is_err());
    }
}
#[test]
fn reject_missing_images_and_data_uris() {
    for s in ["missing.png", "data:image/png;base64,abc"] {
        assert!(imagegen::api::validate_reference(s).is_err());
    }
}
#[test]
fn output_collision_and_protection() {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("a.png");
    std::fs::write(&p, b"original").unwrap();
    assert!(jobs::preflight(std::slice::from_ref(&p), false, false).is_err());
    assert!(jobs::preflight(&[p.clone(), p.with_extension("webp")], true, false).is_err());
    assert_eq!(std::fs::read(p).unwrap(), b"original");
}
#[test]
fn unicode_file_edit_dry_run() {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("产品 图.png");
    image::RgbaImage::new(16, 16).save(&p).unwrap();
    std::fs::write(t.path().join("提示词.txt"), "\u{feff}只改背景\r\n保留主体").unwrap();
    let o = ok(
        t.path(),
        &[
            "edit",
            "--image",
            p.to_str().unwrap(),
            "--prompt-file",
            "提示词.txt",
            "--dry-run",
        ],
    );
    let j: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert!(
        j["payload"]["prompt"]
            .as_str()
            .unwrap()
            .contains("保留主体")
    );
    assert!(j["payload"].get("size").is_none());
}
#[test]
fn batch_preflight_and_unicode() {
    let t = tempfile::tempdir().unwrap();
    std::fs::write(
        t.path().join("jobs.jsonl"),
        "\u{feff}{\"prompt\":\"茶杯\",\"out\":\"茶杯.png\"}\r\n{\"prompt\":\"狐狸\"}\r\n",
    )
    .unwrap();
    let o = ok(
        t.path(),
        &[
            "batch",
            "--input",
            "jobs.jsonl",
            "--out-dir",
            "图像 空格",
            "--dry-run",
        ],
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&o.stdout).unwrap()["jobs"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(!t.path().join("图像 空格").exists());
}
#[test]
fn batch_rejects_escape_and_invalid_second_job() {
    let t = tempfile::tempdir().unwrap();
    for value in [
        json!({"prompt":"x","out":"../bad.png"}),
        json!({"prompt":"x","quality":"invalid"}),
    ] {
        std::fs::write(
            t.path().join("jobs.jsonl"),
            format!("{{\"prompt\":\"ok\"}}\n{value}\n"),
        )
        .unwrap();
        assert!(
            !cli(
                t.path(),
                &["batch", "--input", "jobs.jsonl", "--out-dir", "out"]
            )
            .status
            .success()
        );
        assert!(!t.path().join("out").exists());
    }
}
#[test]
fn chroma_removal_is_native() {
    let t = tempfile::tempdir().unwrap();
    let mut im = image::RgbaImage::from_pixel(16, 16, image::Rgba([0, 255, 0, 255]));
    im.put_pixel(8, 8, image::Rgba([255, 0, 0, 255]));
    im.save(t.path().join("绿幕 图.png")).unwrap();
    ok(
        t.path(),
        &[
            "remove-chroma-key",
            "--input",
            "绿幕 图.png",
            "--out",
            "透明 图.png",
        ],
    );
    let out = image::open(t.path().join("透明 图.png"))
        .unwrap()
        .to_rgba8();
    assert_eq!(out.get_pixel(0, 0)[3], 0);
    assert_eq!(out.get_pixel(8, 8)[3], 255);
}
#[test]
fn config_path_and_reset_recover_invalid_file() {
    let t = tempfile::tempdir().unwrap();
    std::fs::create_dir(t.path().join("config")).unwrap();
    std::fs::write(t.path().join("config/config.toml"), "broken[").unwrap();
    ok(t.path(), &["config", "path"]);
    ok(t.path(), &["config", "reset"]);
    ok(t.path(), &["doctor"]);
}
