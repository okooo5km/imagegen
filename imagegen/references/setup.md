# Install the native CLI

Supported release targets: macOS Intel/Apple Silicon, Linux x64/ARM64 (static musl), Windows x64/ARM64 (MSVC with a bundled C runtime). Download only the archive matching the OS/architecture. No Python, Node.js, Rust compiler or OpenSSL installation is needed to run it.

## Skill-first startup

Users may install only the Skill. On first invocation, the agent checks the CLI on PATH and in the configured/default installation directory. If missing, the agent runs the appropriate installation commands below, verifies the binary with `--version` and `doctor`, and resumes the original task. Do this automatically when the host permits it; follow any host approval prompts. A skill import is not an executable installation hook.

Install into the environment where the agent runs commands. A sandbox/container installation does not install onto the user's desktop and may disappear when the environment resets. If that environment cannot download or execute the binary, explain the limitation rather than claiming the Skill is ready. Existing CLI installations are reused; a missing PATH entry alone is not a reason to reinstall. For an override, use `IMAGEGEN_INSTALL_DIR` consistently in the installer and subsequent absolute executable path.

### Keep the CLI current

Existing installations are upgraded, not reused: run `imagegen --version` before use, compare it against the latest release listed at `https://github.com/okooo5km/imagegen/releases/latest`, and re-run the platform installer when it is older. The installer always downloads from `releases/latest` and replaces the binary in place, so re-running it is the supported upgrade path; it never touches the config or API key. Newer releases add models, endpoint settings and flags that this Skill documents, so an outdated binary can reject them. Report the installed version when an update is not possible, and verify with `--version` after upgrading.

## macOS / Linux

```sh
curl --proto '=https' --tlsv1.2 -fLsS https://github.com/okooo5km/imagegen/releases/latest/download/install.sh -o /tmp/imagegen-install.sh
sh /tmp/imagegen-install.sh
export PATH="$HOME/.local/bin:$PATH"
imagegen --version
```

The installer uses system curl, tar and sha256sum (Linux) or shasum (macOS), verifies the archive, and installs to `~/.local/bin`. Add the export to the user's shell profile if they want it in future terminals. The installer does not rewrite shell profiles. The absolute binary path works immediately in agents even when PATH has not refreshed. macOS binaries are ad-hoc signed, not Developer ID notarized; managed-machine policies may require administrator approval.

## Windows PowerShell

Use Windows PowerShell 5.1 or PowerShell 7 with the native Windows binary; WSL is not required. If the host executes in CMD or Git Bash, launch PowerShell for these examples instead of mixing shell syntax. If it executes in WSL, follow the Linux instructions and use Linux paths and configuration.

```powershell
Invoke-WebRequest -UseBasicParsing https://github.com/okooo5km/imagegen/releases/latest/download/install.ps1 -OutFile "$env:TEMP\imagegen-install.ps1"
powershell -NoProfile -ExecutionPolicy Bypass -File "$env:TEMP\imagegen-install.ps1"
if ($LASTEXITCODE -ne 0) { throw 'imagegen installation failed' }
$bin = if ($env:IMAGEGEN_INSTALL_DIR) { $env:IMAGEGEN_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'imagegen\bin' }
$env:Path = "$bin;$env:Path"
$exe = Join-Path $bin 'imagegen.exe'
& $exe --version
if ($LASTEXITCODE -ne 0) { throw 'imagegen could not run' }
& $exe doctor
if ($LASTEXITCODE -ne 0) { throw 'imagegen doctor failed' }
```

The installer uses built-in PowerShell download, SHA-256 and ZIP support, installs to `%LOCALAPPDATA%\imagegen\bin`, and adds this directory to the user PATH. The explicit assignment above updates the current terminal too. No administrator privilege is required. Windows binaries are not Authenticode signed; organization execution policies remain authoritative.

## Mainland China endpoints

The default API endpoint is `https://api.apimart.ai`, which is not reachable from mainland China without a proxy (TCP connections time out or reset). APIMart publishes three mainland-reachable entry points serving the same backend and API: [apib.ai](https://apib.ai), [aiuxu.com](https://aiuxu.com), [aishuch.com](https://aishuch.com). Set one to avoid a proxy:

```sh
imagegen config set api_base https://apib.ai
imagegen doctor
```

Precedence is `APIMART_API_BASE` > saved `api_base` > `https://api.apimart.ai`. All commands (generate, edit, batch, fetch, auth check, models --live, prices) honor it. The endpoint override does not redirect where result images are downloaded from; those URLs come from the response. See [configuration.md](configuration.md#api-endpoint).

### Existing installations and paths

Use `Get-Command imagegen -CommandType Application -ErrorAction SilentlyContinue` to resolve an executable on PATH. If absent, resolve `$bin` and `$exe` as above and check `Test-Path -LiteralPath $exe` before installing. A quoted executable path needs the call operator: `& $exe --version`. Keep using that path if the agent starts a fresh shell for each command; a previous shell's PATH assignment may not persist.

`%LOCALAPPDATA%` and `%APPDATA%` are directory notation/CMD syntax; in PowerShell expand them as `$env:LOCALAPPDATA` and `$env:APPDATA`. Pass absolute paths for inputs outside the current project. In JSONL, escape Windows backslashes (`"C:\\Users\\Alice\\Pictures\\reference.png"`) or use forward slashes (`"C:/Users/Alice/Pictures/reference.png"`). Shell quoting does not replace JSON escaping.

### UTF-8 prompts and batches

Run this from the project directory, using the resolved `$exe` above:

```powershell
@'
一只陶瓷茶壶，柔和的摄影棚光线。
保留表面的中文文字。
'@ | Set-Content -LiteralPath '.\prompt.txt' -Encoding UTF8

& $exe generate --prompt-file '.\prompt.txt' --out '.\output\teapot.png' --dry-run
if ($LASTEXITCODE -ne 0) { throw 'Prompt validation failed' }
```

Remove `--dry-run` for the requested generation after authentication is ready. A single-quoted here-string preserves literal quotes, dollar signs and backticks; its closing `'@` must start at the beginning of a line. An agent file-writing tool may also write UTF-8 directly. Use the same explicit encoding for `jobs.jsonl` and validate it with `batch --input jobs.jsonl --out-dir output/images --dry-run`.

Do not use bare `>` or `Out-File` to create these text inputs in Windows PowerShell 5.1: they default to UTF-16LE. `Set-Content -Encoding UTF8` works in both 5.1 (UTF-8 with BOM) and 7 (UTF-8 without BOM); the CLI accepts both, including CRLF. Check `$LASTEXITCODE` immediately after native CLI calls; `$ErrorActionPreference = 'Stop'` alone does not reliably stop a script on a native command's nonzero exit.

## Alternatives

Download an archive plus its `.sha256` from [GitHub Releases](https://github.com/okooo5km/imagegen/releases/latest), verify the checksum, extract `imagegen`/`imagegen.exe`, and put it in a directory on PATH. Offline deployment can use the same installers with `IMAGEGEN_ARCHIVE_DIR` pointing to a directory holding both files.

Installer overrides: `IMAGEGEN_INSTALL_DIR`, `IMAGEGEN_VERSION` (defaults to the latest release; pin a tag to reproduce an exact version), `IMAGEGEN_ARCHIVE_DIR`. Windows additionally supports `IMAGEGEN_NO_MODIFY_PATH=1`. Do not use a different version's checksum.

For developers with Rust 1.88+: `cargo install --git https://github.com/okooo5km/imagegen --tag v2.2.0 --locked`. This is optional; normal users should use a release binary.

## API key

Register at [APIMART](https://apimart.ai/register?aff=LEBeCc), create a key on the [API keys page](https://apimart.ai/keys), and ensure the account has a usable balance.

When the main site is unreachable (mainland China without a proxy), register through a backup domain instead — same account system, referral code included: [apib.ai](https://apib.ai/register?aff=LEBeCc), [aiuxu.com](https://aiuxu.com/register?aff=LEBeCc), [aishuch.com](https://aishuch.com/register?aff=LEBeCc).

```text
imagegen auth set
imagegen auth status
imagegen auth check
imagegen doctor
```

`auth set` reads hidden input from the user's terminal and stores the key outside the project. Do not paste keys into chat. In CI, provide `APIMART_API_KEY` through a secret environment facility. Presence checks do not authenticate or verify balance. Project `.env` files are not loaded.

Configuration location: `$XDG_CONFIG_HOME/imagegen` or `~/.config/imagegen` on macOS/Linux; `%APPDATA%\imagegen` on Windows; override with `IMAGEGEN_CONFIG_DIR`. Keys remain in `apimart.key`, separately from defaults in `config.toml`. Unix files are created with mode 600; Windows uses user-profile access controls. Old `config.json` default-model settings and legacy key locations remain readable. See [configuration.md](configuration.md).

## First-use authentication

For authenticated work, check local presence with `auth status`. If missing, show the signup and key-page links above and have the user run `imagegen auth set` in their own terminal. After saving/replacing the key, `imagegen auth check` makes one authenticated `GET /v1/models` request; no image is generated. Resume the requested task after success. Existing working credentials do not need a check before every generation.

`auth check` verifies catalog access only, not balance or all generation permissions. HTTP 401 means the key was rejected; guide replacement and check whether `APIMART_API_KEY` overrides the saved key. HTTP 403 means access was denied and may be an account/permission restriction. HTTP 429, server errors and network failures leave validity unknown: keep the key, do not replace it automatically. Unexpected response bodies are not echoed. All failures exit nonzero.

Public prices, offline models, defaults, dry runs and local chroma-key removal require no authentication. Complete them without requiring signup. `check-key` remains a legacy alias for the local presence check; use `auth check` for online verification.
