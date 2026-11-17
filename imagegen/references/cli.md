# Commands and recovery

Run `imagegen --help` or `<command> --help`. Invoke from the desired output workspace, using an absolute binary path if PATH is not available.

```text
imagegen generate --prompt-file prompt.txt --aspect-ratio 16:9 --resolution 2k --quality high --out output/hero.png
imagegen edit --model gpt-image-2.5-sunburst --image "photos/my product.png" --prompt "Replace only the background with white" --out output/product.png
imagegen generate --prompt "A fox sticker" --background transparent --output-format webp --out output/fox.webp
imagegen generate --prompt "A teapot" --size 1600x1200 --dry-run
```

`--image` also works on `generate` for reference-based variations. UTF-8 prompt/JSONL files accept BOM and CRLF. `--n` creates variants of one prompt. Distinct prompts belong in a batch. Downloaded signatures determine actual file extensions. Existing files are protected before billing and again during atomic writes; `--force` opts into replacement.

## Batch

```jsonl
{"prompt":"A red ceramic teapot", "out":"teapot.png"}
{"prompt":"A fox sticker", "background":"transparent", "output_format":"webp", "out":"fox.webp"}
{"prompt":"Preserve the product, change only the background", "model":"gpt-image-2.5-sunburst", "image_urls":["photos/product.png"], "out":"product.png"}
```

```text
imagegen batch --input jobs.jsonl --out-dir output/batch --dry-run
imagegen batch --input jobs.jsonl --out-dir output/batch --concurrency 2
```

API-style parameter names are used, plus `aspect_ratio` and relative `out`. Inputs are relative to the working directory. All jobs and output destinations are validated before API calls. Outputs cannot traverse parent directories or symlinks; case/extension collisions are rejected for portability. 1–500 jobs and 1–8 worker threads. A failure does not remove successful outputs. Every submitted job can incur a charge.

## Account balance, usage and logs

Read-only, need a key, honor `api_base`. Add `--json` for raw API JSON. A `success:false` reply (unknown key) is an error; server messages are not echoed.

```text
imagegen balance [--account] [--json]
imagegen usage [--start X] [--end X] [--model A,B] [--group-by none|model|date|model,date] [--tz ZONE] [--account] [--json]
imagegen logs [--start UNIX] [--end UNIX] [--type all|consume|error] [--model M] [--token-id ID] [--limit 1-1000] [--cursor C] [--json]
imagegen logs REQUEST_OR_TASK_ID [--json]
```

`balance` reports the current key (`GET /v1/balance`; "unlimited" when the key has unlimited quota) or, with `--account`, the whole account (`/v1/user/balance`). `usage` aggregates spend (`/v1/usage`): `--start`/`--end` take unix seconds or RFC3339, default the last 24 hours, max 31 days; `--account` widens scope from key to account. `logs` lists per-request records (`/v1/logs/requests`); `--start`/`--end` are unix seconds only, start defaults to 24 hours ago, and the end defaults to now. Logs cover the whole account, not just the current key (filter with `--token-id`). Paging: rerun with the printed `More: --start .. --end .. --cursor ..` line (`--cursor` requires `--start`); IDs may contain only letters, digits and `_ . : -`. Logs appear a few seconds after a request completes; a 404 on lookup means retry later. A lookup can return several rows (e.g. consume and error). Use `logs ID` to look up one image task after a failed or unclear generation.

## Recovery

Submission prints a task ID and writes a receipt under the config directory's `tasks/` with model and absolute output paths, excluding prompts, credentials and reference URLs.

```text
imagegen fetch TASK_ID
imagegen fetch TASK_ID --out output/recovered.png
imagegen fetch TASK_ID --n 4 --out-dir output/recovered
```

Without output/count flags, fetch restores the Rust receipt if it exists. If working on another machine, or recovering a v1 task, provide the original count and intended path explicitly. Already downloaded files remain protected; use `--force` only when replacing them is intended. Fetch never creates another generation task.

The default timeout is 600 seconds per task, polling every 4 seconds; settings and flags can override both. Stopping the CLI does not cancel a remote task. If submission fails without a task ID, check APIMART history before submitting again. No generation POST is automatically retried, including transport retry policies.

Exit codes: 0 success, 1 runtime/configuration/API failure, 2 CLI syntax failure (clap). Partial batches return 1. Ctrl-C terminates the process; retain submitted task IDs for recovery.
