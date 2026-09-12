# CLI and recovery

Paths in these examples are relative to the skill directory. Agents should invoke the script by absolute path from the user's workspace. Use `python3`, `python`, or `py -3` according to the installed Python 3.10+ interpreter. Every command exposes `--help`.

```text
python scripts/image_gen.py generate --prompt-file prompt.txt --size 16:9 --resolution 2k --quality high --out output/hero.png
python scripts/image_gen.py edit --model gpt-image-2.5-sunburst --image "photos/my product.png" --prompt "Replace only the background with white" --out output/product.png
python scripts/image_gen.py generate --prompt "A paper fox" --background transparent --output-format webp --out output/fox.webp
python scripts/image_gen.py generate --prompt "A ceramic mug" --size 1600x1200 --dry-run
```

`generate --image` also supports references; supplying images activates image-to-image behavior. Prompt and prompt-file are mutually exclusive; text files use UTF-8 (BOM accepted). `--n` requests variants of one prompt. Multiple results use numbered filenames. A downloaded image's signature determines its extension. Existing files and extension collisions are checked before submission, then again at write time.

`--timeout` defaults to 600 seconds per task; `--poll-interval` defaults to 4 seconds. Network operations have bounded timeouts. A timeout stops waiting, not the remote task. Interrupted work may continue and incur a charge.

## Batch

Create a UTF-8 JSONL file with one request per line. Use API-style parameter names; `out` is a relative path inside `--out-dir`:

```jsonl
{"prompt":"A red ceramic teapot", "out":"teapot.png"}
{"prompt":"A fox sticker", "background":"transparent", "output_format":"webp", "out":"fox.webp"}
{"prompt":"Preserve the product, change only the background", "model":"gpt-image-2.5-sunburst", "image_urls":["photos/product.png"], "out":"product.png"}
```

```text
python scripts/image_gen.py generate-batch --input jobs.jsonl --out-dir output/batch --dry-run
python scripts/image_gen.py generate-batch --input jobs.jsonl --out-dir output/batch --concurrency 2
```

All jobs are validated before API work, including missing input files, unsupported parameters, output collisions and traversal outside the output directory. Reference paths are relative to the working directory, not the JSONL file. 1–500 jobs, concurrency 1–8. Independent failures do not delete completed results. There is no automatic retry; review task IDs before rerunning only failed jobs.

## Recovery

The CLI writes a receipt under the configuration directory's `tasks/`, containing task ID, model and intended absolute output paths, without prompts or secrets.

```text
python scripts/image_gen.py fetch TASK_ID --out output/recovered.png
python scripts/image_gen.py fetch TASK_ID --n 4 --out-dir output/recovered
```

Use the original `n` and intended paths from the receipt. Fetch polls/downloads and never submits a new generation. Already downloaded files are protected; pass `--force` only when replacing them is intended. If a submission fails without an ID, look in APIMART task history before retrying.

Exit codes: `0` success, `2` invalid arguments/configuration/input, `1` API/filesystem/task failure, `130` keyboard interrupt. A partial batch failure exits nonzero.

## Migration from the original skill

The old `--augment`/prompt-augmentation fields and downscale flags are removed. Put the final prompt in a UTF-8 prompt file; post-process through your preferred image tool. `auto` no longer suggests fallback models. Resolution defaults to 1K for every ratio, avoiding an implicit increase for wide images. Base64 input is rejected because APIMART now requires uploads. Old `~/.config/imagegen/apimart.key` files remain supported.
