---
name: imagegen
description: Generate and edit raster images through APIMart, and compare its image models and current prices. Use for image creation, reference-based variations, image editing, or APIMart image model selection. Prefer existing SVG or code assets when the requested change is native to those formats.
license: Apache-2.0; see LICENSE.txt
metadata:
  author: okooo5km(十里)
  version: "1.0.0"
---

# APIMart Imagegen

Use the bundled `scripts/image_gen.py` to generate/edit images and inspect models/prices. Resolve this script relative to this `SKILL.md`, then invoke its **absolute path** from the user's workspace so outputs belong to their project. Choose an available Python 3.10+ interpreter (`python3` on macOS/Linux; `py -3` or `python` on Windows). Quote paths containing spaces. No Bash, curl, Node.js, SDK or virtual environment is required for API operations.

## Setup and model choice

- Run `check-key` to check local configuration. It does not authenticate or charge the account.
- If missing, read [references/setup.md](references/setup.md) and show its registration link and `configure --key` command. The user enters their key in their terminal; do not ask them to paste it into chat. Never print a key or put it in a project file.
- The built-in default is **gpt-image-2.5-flare**, 1K, medium quality, one image. An explicit `--model` overrides `APIMART_IMAGE_MODEL`, then the saved default. `auto` uses that same precedence; it never tries multiple models.
- `models` lists runnable profiles offline. `models --live --prices --json` fetches account-visible image models and public prices. `prices --model MODEL --json` needs no key. Read [references/models-and-pricing.md](references/models-and-pricing.md) when comparing costs or selecting a default. Preserve user model choices; change a saved default only when requested.

## Generate or edit

```text
python "<absolute-skill-path>/scripts/image_gen.py" generate --prompt "A ceramic teapot, studio photograph" --size 16:9 --resolution 2k --out "output/teapot.png"
python "<absolute-skill-path>/scripts/image_gen.py" edit --image "product.png" --prompt "Change only the background to warm gray; preserve the product and its lettering" --out "output/product-edited.png"
```

Use `--prompt-file` for long prompts, quotes or multiline text. Preserve requested wording and editing invariants. Read [references/prompting.md](references/prompting.md) for substantial prompt development, or [references/sample-prompts.md](references/sample-prompts.md) for relevant examples; neither is required for every request.

Local references upload to APIMart and become temporary public URLs before generation. Use only user-authorized inputs. Repeat `--image` for references; GPT Image 2.5 accepts up to 16. Editing omits size by default so the service can preserve input aspect ratio. An explicit `--size` overrides that behavior.

GPT Image 2.5 supports native transparency: use `--background transparent --output-format png` (or WebP). Inspect the actual alpha channel and edges. Use [references/chroma-key.md](references/chroma-key.md) only for existing keyed images or when a chroma-key workflow is appropriate. Masks are supported by the bundled `gpt-image-2-official` profile, not the documented 2.5 endpoint.

`--dry-run` validates and previews a request without a key, upload, network call or output directory creation. Unsupported model parameters fail locally rather than disappearing silently.

For distinct prompts, use `generate-batch --input jobs.jsonl --out-dir output/images`; read [references/cli.md](references/cli.md) for batch format, recovery and examples. Validate with `--dry-run` first. Default concurrency is two; each job incurs its own charge.

## Completion and recovery

Report the model and absolute saved paths. Inspect the image for subject, composition, text and requested invariants before declaring the visual complete. Respect user scope when considering another paid iteration.

The CLI prints a task ID and saves a receipt outside the project after submission. It never retries a paid submission or switches models automatically. If polling or downloading fails, resume with `fetch TASK_ID --out path.png` (supply `--n` for multiple images). If submission fails without a task ID, check APIMart task history before retrying. Downloads must be saved before temporary URLs expire.

Existing outputs are refused unless `--force` was requested. Keep successful batch outputs and resume only unfinished tasks; do not blindly repeat a batch.
