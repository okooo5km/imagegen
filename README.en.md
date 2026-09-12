# APIMart Imagegen

[![Cross-platform validation](https://github.com/okooo5km/apimart-imagegen/actions/workflows/ci.yml/badge.svg)](https://github.com/okooo5km/apimart-imagegen/actions/workflows/ci.yml)

**Generate and edit images, discover image models, and inspect current APIMART prices from an Agent Skill or standalone Python CLI.**

[中文](README.md) · English

Default: **gpt-image-2.5-flare**. Also supports **gpt-image-2.5-sunburst**. Runs on macOS, Linux and Windows with Python 3.10+. API operations use only the standard library.

By **okooo5km(十里)**. Explore more skills and the author's YouMind work at [sink.5km.tech/skills](https://sink.5km.tech/skills).

## Install

With Node.js/npm available, use the [skills CLI](https://github.com/vercel-labs/skills):

```sh
npx skills add okooo5km/apimart-imagegen --skill imagegen
```

Choose your agent and installation scope. Node.js is only needed for this installer, not for running the skill. If another `imagegen` skill is already installed, decide which version to keep before replacing it.

Alternatively, clone the repository and copy the **entire `imagegen/` folder** to your agent's supported skill directory:

```sh
git clone https://github.com/okooo5km/apimart-imagegen.git
```

The agent must be able to read skills, execute Python and access the network. The script can also be used independently. The examples below run from the cloned repository root; for an installed skill, substitute its absolute script path and run from your project directory.

## Get an API key

Register on the [APIMART website](https://apimart.ai), create a key on the [API keys page](https://apimart.ai/keys), and ensure the account has sufficient balance. Enter the key in your own terminal, never in an AI conversation.

macOS / Linux:

```sh
python3 imagegen/scripts/image_gen.py configure --key
python3 imagegen/scripts/image_gen.py check-key
```

Windows PowerShell:

```powershell
py -3 imagegen/scripts/image_gen.py configure --key
py -3 imagegen/scripts/image_gen.py check-key
```

Use `python` if that is the Python 3.10+ interpreter available on your system. Hidden input displays no characters. `check-key` verifies local presence only, not authentication or balance.

Key lookup: `APIMART_API_KEY` environment variable, then the platform config directory's `apimart.key`, then the original `~/.config/imagegen/apimart.key` for compatibility. The config directory is `$XDG_CONFIG_HOME/imagegen` or `~/.config/imagegen` on macOS/Linux, and `%APPDATA%\imagegen` on Windows. Override it with `IMAGEGEN_CONFIG_DIR` to isolate environments; this disables legacy fallback.

UTF-8 BOM and CRLF are supported. Unix key files use mode 600; Windows uses the user directory's access controls. For CI, use a secret environment variable. Project `.env` files are not loaded automatically.

## Compare models and prices

Replace `python` below with `python3` or `py -3` as needed:

```sh
python imagegen/scripts/image_gen.py models
python imagegen/scripts/image_gen.py models --prices
python imagegen/scripts/image_gen.py models --live --prices --json
python imagegen/scripts/image_gen.py prices --model gpt-image-2.5-flare --json
```

`models` lists offline runnable profiles. `--live` fetches account-visible image models and parameter metadata, requiring an API key. Unadapted models are marked **discovery only**. They cannot be selected for generation until a tested profile is added.

Runnable models:

| Model | Purpose | Images/request |
| --- | --- | --- |
| **gpt-image-2.5-flare** | Default; routine generation/editing | 1–4 |
| **gpt-image-2.5-sunburst** | Precision editing | 1–4 |
| gpt-image-2 | Legacy minimal parameters | 1 |
| gpt-image-2-official | Legacy quality controls and masks | 1–4 |
| gemini-3.1-flash-image-preview | Google search/image grounding | 1–4 |

Prices come from a public endpoint without sending credentials. Token models expose `effective_rates`, units and retrieval time; missing fields are not treated as free. Default-group quotes may differ from account billing. Legacy per-image quotes are preserved with **payable rate unverified** labeling. A token rate is not a fixed per-image total: actual charges depend on input/output tokens, quality, dimensions, references and count. See [models and pricing](imagegen/references/models-and-pricing.md).

```sh
python imagegen/scripts/image_gen.py configure --default-model gpt-image-2.5-sunburst
```

Precedence: explicit `--model` > `APIMART_IMAGE_MODEL` > saved default > `gpt-image-2.5-flare`. `auto` follows the same precedence and never tries multiple models. Defaults are **1K, medium quality, one image**. Both 2.5 models support low/medium/high/xhigh/max/auto quality, 15 aspect ratios, validated exact pixel dimensions and native transparency. `gpt-image-2.5-ext` is a separate route, not an alias.

## Generate and edit

```sh
python imagegen/scripts/image_gen.py generate --prompt "A ceramic teapot in soft studio light" --size 16:9 --resolution 2k --out output/teapot.png
python imagegen/scripts/image_gen.py edit --image "photos/my product.png" --prompt "Change only the background to warm gray; preserve the product and lettering" --out output/product-edited.png
python imagegen/scripts/image_gen.py generate --prompt "A folded paper fox, complete subject" --background transparent --output-format png --out output/fox.png
python imagegen/scripts/image_gen.py generate --prompt-file prompt.txt --size 1600x1200 --dry-run
```

Use UTF-8 `--prompt-file` for long, quoted or multiline prompts. Repeat `--image` for up to 16 GPT Image 2.5 references. Edits omit size by default, allowing the service to infer it from the input aspect ratio. An explicit size overrides that behavior.

Local inputs upload to APIMART as temporary **public URLs** before generation. Supported inputs: PNG/JPEG/WebP/GIF, at most 20 MiB each. Base64 input is no longer accepted by the service. Use only material you are authorized to upload. Outputs are downloaded promptly because result URLs expire.

Dry runs perform local validation without uploads, network access, API keys, output directory creation or charges. Unsupported parameters fail locally. Existing output files are protected unless `--force` is supplied. Inspect generated text, content and alpha rather than assuming a successful HTTP response proves visual correctness.

## Batch and resume

Create UTF-8 `jobs.jsonl`:

```jsonl
{"prompt":"A red ceramic teapot", "out":"teapot.png"}
{"prompt":"A paper fox sticker", "background":"transparent", "out":"fox.png"}
```

```sh
python imagegen/scripts/image_gen.py generate-batch --input jobs.jsonl --out-dir output/batch --dry-run
python imagegen/scripts/image_gen.py generate-batch --input jobs.jsonl --out-dir output/batch --concurrency 2
python imagegen/scripts/image_gen.py fetch TASK_ID --out output/recovered.png
python imagegen/scripts/image_gen.py fetch TASK_ID --n 4 --out-dir output/recovered
```

Batch validation checks every job before API work, including paths and output collisions. Limits: 1–500 jobs, concurrency 1–8, default 2. Partial failures return nonzero while keeping successful outputs.

Submission prints a task ID and saves a receipt under the config directory's `tasks/`, without prompts or credentials. On timeout or download failure, use `fetch` with the original count and intended paths. It does not submit another generation. If submission fails without an ID, check APIMART task history before retrying. See [CLI and recovery](imagegen/references/cli.md).

## Validation and development

The [CI matrix](https://github.com/okooo5km/apimart-imagegen/actions/workflows/ci.yml) runs on **macOS, Ubuntu and Windows × Python 3.10–3.14**. Tests cover Unicode/space paths, BOM/CRLF, configuration, multipart uploads, task polling/downloads, recovery, price semantics, overwrite protection, batches and skill structure. Local HTTP fixtures and isolated configuration mean CI requires no API key and incurs no generation charges.

```sh
python -m pip install -r requirements-dev.txt
python -m ruff check .
python -m unittest discover -s tests -v
```

The skill also passes skill-creator's `quick_validate.py` and an independent agent usage check. CI cannot guarantee upstream availability, account permissions or generated content quality. [Portability design and primary sources](docs/portability.md).

Optional local chroma-key removal and alpha-mask validation require Pillow; core API operations do not. On a protected system Python, use a virtual environment or `uv run --with pillow`. Invoking the chosen interpreter by path avoids shell activation differences.

## Contribute and license

Model contributions should include authoritative APIMART parameter sources, behavioral tests and green three-platform CI. Remove keys, private prompts and temporary image URLs from public reports. Live paid tests need maintainer authorization and an appropriate account.

Apache-2.0; see [LICENSE](LICENSE) and [NOTICE](NOTICE). Derived from the original imagegen skill, with revised networking, portable configuration, model discovery, pricing, docs and tests. This independent community project is not affiliated with OpenAI, APIMART or YouMind.

Explore [more skills and YouMind work by 十里](https://sink.5km.tech/skills), and star the repository if it helps your workflow.
