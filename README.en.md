# Imagegen

[![Rust build, test and install](https://github.com/okooo5km/imagegen/actions/workflows/ci.yml/badge.svg)](https://github.com/okooo5km/imagegen/actions/workflows/ci.yml)

**A native Rust CLI and standard Agent Skill for APIMART image generation, editing, persistent defaults, model discovery and public pricing.**

[中文](README.md) · English

Default: **gpt-image-2.5-flare**, with **gpt-image-2.5-sunburst** support. Prebuilt binaries cover macOS, Linux and Windows on x64 and ARM64. No Python, Node.js, Rust compiler, Pillow or OpenSSL installation is required to run them.

By **okooo5km(十里)**. [More skills and the author's YouMind work](https://sink.5km.tech/skills).

## Install the CLI

macOS / Linux:

```sh
curl --proto '=https' --tlsv1.2 -fLsS https://github.com/okooo5km/imagegen/releases/download/v2.0.1/install.sh -o /tmp/imagegen-install.sh
sh /tmp/imagegen-install.sh
export PATH="$HOME/.local/bin:$PATH"
imagegen --version
```

Installs to `~/.local/bin` without sudo. Add the export to your shell profile for new terminals; the installer does not edit shell profiles. Agents can use the absolute binary path immediately.

Windows PowerShell:

```powershell
Invoke-WebRequest https://github.com/okooo5km/imagegen/releases/download/v2.0.1/install.ps1 -OutFile "$env:TEMP\imagegen-install.ps1"
powershell -NoProfile -ExecutionPolicy Bypass -File "$env:TEMP\imagegen-install.ps1"
$env:Path = "$env:LOCALAPPDATA\imagegen\bin;$env:Path"
imagegen --version
```

Installs to `%LOCALAPPDATA%\imagegen\bin`, adds it to the user PATH, and requires no administrator privileges. The PATH assignment also updates the current terminal.

Installers detect the architecture and verify SHA-256 before replacing a binary. A corrupt archive does not damage the previous installation. Linux packages use static musl; Windows packages statically link the C runtime. macOS binaries are ad-hoc signed, not Developer ID notarized; Windows binaries are not Authenticode signed. Managed-device execution policies remain authoritative. 32-bit systems are not targeted.

Manual and offline installation: download a matching archive and `.sha256` from [Releases](https://github.com/okooo5km/imagegen/releases/latest), verify, extract, and place the executable on PATH. `IMAGEGEN_ARCHIVE_DIR` lets the installer use local files. Other overrides: `IMAGEGEN_INSTALL_DIR`, `IMAGEGEN_VERSION`; Windows can skip persistent PATH changes with `IMAGEGEN_NO_MODIFY_PATH=1`.

## Install the skill

The CLI works independently. To use it from an agent:

```sh
npx skills add okooo5km/imagegen --skill imagegen
```

Node.js is only required for this optional installer. Download [imagegen.skill](https://github.com/okooo5km/imagegen/releases/latest/download/imagegen.skill) for applications accepting `.skill` imports. It is a ZIP archive containing the standard `imagegen/SKILL.md`, references and agent metadata, with a SHA-256 checksum. An identical [ZIP compatibility package](https://github.com/okooo5km/imagegen/releases/latest/download/imagegen-skill.zip) is available for importers that require `.zip`.

Claude documents **Customize → Skills → + Create skill → Upload a skill**. ChatGPT skill availability and import controls depend on the account and client; the extension does not guarantee OS double-click installation. See [Claude's import instructions](https://support.claude.com/en/articles/12512180-use-skills-in-claude) and [OpenAI's skill structure documentation](https://learn.chatgpt.com/docs/build-skills).

For directory-based agents, extract the archive as ZIP and copy the complete `imagegen/` folder to the skill directory. Decide whether to replace any existing same-name skill. Importing the skill does not install the CLI: its execution environment must support command execution, the matching CLI, network access and APIMART key configuration. A chat environment that only reads attachments cannot execute image generation.

## Configure credentials and defaults

Register at [APIMART](https://apimart.ai), create a key on the [API keys page](https://apimart.ai/keys), and ensure your account has a usable balance. The author's referral link is pending approval; the official website is used meanwhile.

```sh
imagegen auth set
imagegen auth status
imagegen auth check
imagegen doctor
imagegen config set model gpt-image-2.5-flare
imagegen config set aspect_ratio 16:9
imagegen config set resolution 2k
imagegen config set quality high
imagegen config show
```

`auth check` verifies model-list access with a read-only request and generates no image. Success does not verify balance or all model permissions. Replace rejected credentials on 401; investigate permissions on 403; retain the key on rate limits/network failures. Public prices, local configuration and dry runs need no key.

Enter your key in your own terminal using hidden input, never in chat. It is stored separately from defaults and outside the project. CI can provide `APIMART_API_KEY` through a secret environment variable. `auth status` checks local presence, not authentication or balance. Project `.env` files are not loaded automatically.

Settings live in `config.toml`, keys in `apimart.key`. Config directory: `$XDG_CONFIG_HOME/imagegen` or `~/.config/imagegen` on macOS/Linux; `%APPDATA%\imagegen` on Windows. Override with `IMAGEGEN_CONFIG_DIR`. Writes are atomic; Unix files use mode 600, Windows uses the user profile's access controls.

```sh
imagegen config set size 1600x1200
imagegen config unset size
imagegen config path
imagegen config reset
```

Saved exact `size` overrides saved ratio/resolution. Explicit `--aspect-ratio` or `--resolution` overrides a saved exact size; explicit `--size` cannot be combined with explicit ratio/resolution. Edits ignore saved shape/exact size to preserve input proportions unless `--use-default-size` or explicit geometry is provided.

Other defaults: `output_format`, `background`, `n`, `concurrency`, `timeout`, `poll_interval`. Built-ins: Flare, 1:1, 1K, medium, one image, two workers. Model precedence: explicit `--model` > `APIMART_IMAGE_MODEL` > saved model > Flare. `auto` never tries several models. Reset retains the API key. [Full configuration and migration](imagegen/references/configuration.md).

## Generate and edit

```sh
imagegen generate --prompt "A ceramic teapot in soft studio light" --out output/teapot.png
imagegen generate --prompt-file prompt.txt --aspect-ratio 1:1 --out output/square.png
imagegen edit --model gpt-image-2.5-sunburst --image "photos/my product.png" --prompt "Change only the background to warm gray; preserve the product and lettering" --out output/product.png
imagegen generate --prompt "A folded paper fox, complete subject" --background transparent --output-format png --out output/fox.png
imagegen generate --prompt "A ceramic mug" --size 1600x1200 --dry-run
```

Dry runs do not require keys, network calls, uploads or output directory creation. UTF-8 BOM/CRLF and Unicode/space paths are supported. Unsupported parameters fail before billing. Existing outputs are protected unless `--force` is provided.

Local PNG/JPEG/WebP/GIF references (at most 20 MiB each) upload as temporary public URLs before generation. GPT Image 2.5 supports up to 16 references. Only upload authorized material. Inspect content, text and alpha after download; a successful request does not prove visual correctness.

Optional native local processing:

```sh
imagegen remove-chroma-key --input keyed.png --out transparent.png --key 00ff00 --despill
```

## Models and prices

```sh
imagegen models
imagegen models --prices
imagegen models --live --prices --json
imagegen prices --model gpt-image-2.5-flare --json
```

Bundled runnable profiles: Flare, Sunburst, gpt-image-2, gpt-image-2-official, gemini-3.1-flash-image-preview. Live discovery requires a key and can include additional **discovery-only** models. Discovery is not a promise of generation support.

Public pricing requests never send credentials. Use returned `effective_rates` with their units and timestamps; missing fields are not free and discounts are not applied twice. GPT Image 2.5 bills actual token usage. Public default-group prices can differ from account charges. Legacy per-image quotes are explicitly labeled unverified. [Model and pricing details](imagegen/references/models-and-pricing.md).

## Batches and recovery

Each line of `jobs.jsonl` is a request; omitted fields inherit saved defaults:

```jsonl
{"prompt":"A red ceramic teapot", "out":"teapot.png"}
{"prompt":"A paper fox sticker", "background":"transparent", "out":"fox.png"}
```

```sh
imagegen batch --input jobs.jsonl --out-dir output/batch --dry-run
imagegen batch --input jobs.jsonl --out-dir output/batch --concurrency 2
imagegen fetch TASK_ID
```

All jobs/destinations validate before API calls. Limits: 500 jobs, 1–8 workers. Successful outputs survive partial failures. Task receipts store model and output paths/count, not prompts or credentials. Fetch restores a local receipt and never resubmits generation; on another machine provide output/count flags. If submission fails without an ID, check APIMART task history first. [CLI reference](imagegen/references/cli.md).

## Development and verification

Rust 2024, MSRV 1.88, checked-in `Cargo.lock`, no unsafe project code. reqwest/rustls, clap, serde and image libraries compile into the executable. Rust improves distribution consistency; remote generation latency remains dominated by the API.

```sh
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo audit
cargo build --release --locked
```

CI builds/tests on six native OS/architecture combinations, packages the executable, runs the installer and installed CLI, and rejects deliberately corrupt archives. It also checks the minimum compiler, lints and dependency advisories. CI uses local HTTP fixtures and no paid API calls. [Portability decisions](docs/portability.md).

v1's Python implementation remains in history; v2 ships only the Rust CLI. Old default-model JSON and key locations remain readable; new config writes use TOML. Developers can install from source with `cargo install --git https://github.com/okooo5km/imagegen --tag v2.0.1 --locked`.

Apache-2.0; see [LICENSE](LICENSE) and [NOTICE](NOTICE). Independent community project, not affiliated with OpenAI, APIMART or YouMind. Model contributions should include official API sources and behavioral tests. Remove secrets and private references from public reports.
