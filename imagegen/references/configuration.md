# Persistent defaults

```text
imagegen config show
imagegen config path
imagegen config set model gpt-image-2.5-flare
imagegen config set aspect_ratio 16:9
imagegen config set resolution 2k
imagegen config set quality high
imagegen config set size 1600x1200
imagegen config unset size
imagegen config reset
```

The file is `config.toml` in the platform configuration directory. Writes validate the settings and replace the file atomically. `unset` clears one override; `reset` clears all defaults but retains the API key. `path` and `reset` also work when the current config is malformed.

Example file:

```toml
version = 1

[defaults]
model = "gpt-image-2.5-flare"
aspect_ratio = "16:9"
resolution = "2k"
quality = "medium"
output_format = "png"
n = 1
concurrency = 2
timeout = 600
poll_interval = 4
```

| Setting | Accepted values | Built-in behavior |
| --- | --- | --- |
| model | Bundled runnable model ID | gpt-image-2.5-flare |
| aspect_ratio | auto or one of the 15 documented ratios | 1:1 for generation |
| size | auto or valid WIDTHxHEIGHT | unset |
| resolution | 1k, 2k, 4k | 1k |
| quality | low, medium, high, xhigh, max, auto | medium on GPT Image 2.5; provider default on legacy profiles |
| output_format | png, jpeg, webp | png |
| background | auto, opaque, transparent | provider default |
| n | 1–4 (model-specific caps apply) | 1 |
| concurrency | 1–8 | 2 |
| timeout | 1–86400 seconds | 600 |
| poll_interval | 1–60 seconds | 4 |

CLI parameters override saved defaults. Model precedence is explicit `--model` > `APIMART_IMAGE_MODEL` > saved model > built-in Flare. `auto` uses the same precedence, never a fallback sequence. Config files do not contain secrets.

## Geometry rules

- `aspect_ratio` is a shape such as 16:9; `size` is exact dimensions such as 1600x1200.
- Saved exact `size` wins over saved ratio/resolution. No resolution field is sent for an exact size.
- An explicit `--aspect-ratio` or `--resolution` selects ratio/resolution mode and overrides a saved exact size.
- An explicit `--size` cannot be combined with explicit `--aspect-ratio` or `--resolution`.
- Edits ignore saved ratio and exact size, preserving reference proportions. Pass explicit geometry or `--use-default-size` to opt in. Saved resolution and quality still apply.
- Settings unsupported by the chosen model fail before submission rather than disappearing. For example, a saved `quality=high` must be unset before using the minimal legacy `gpt-image-2` profile.

Exact dimensions require multiples of 16, sides at most 3840, a ratio at most 3:1 and 655360–8294400 total pixels. `auto` delegates dimensions to the model; it is not a fixed pixel size.

## Migration from v1

The Rust binary reads v1's `config.json` default_model when no `config.toml` exists. A subsequent config change writes the new TOML file; the JSON and key are not deleted. Key precedence remains environment → current configuration directory → legacy `~/.config/imagegen/apimart.key`. Setting `IMAGEGEN_CONFIG_DIR` disables the legacy fallback for isolation.

Python scripts are removed in v2. Use `imagegen auth set` instead of `configure --key`, `config set model MODEL` instead of `configure --default-model`, and `--aspect-ratio` instead of passing ratios to `--size`. `batch` retains `generate-batch` as an alias. `fetch` now restores paths/count from Rust receipts when available; v1 receipts can be inspected manually and their task ID used with explicit output flags.
