# Install the native CLI

Supported release targets: macOS Intel/Apple Silicon, Linux x64/ARM64 (static musl), Windows x64/ARM64 (MSVC with a bundled C runtime). Download only the archive matching the OS/architecture. No Python, Node.js, Rust compiler or OpenSSL installation is needed to run it.

## Skill-first startup

Users may install only the Skill. On first invocation, the agent checks the CLI on PATH and in the configured/default installation directory. If missing, the agent runs the appropriate installation commands below, verifies the binary with `--version` and `doctor`, and resumes the original task. Do this automatically when the host permits it; follow any host approval prompts. A skill import is not an executable installation hook.

Install into the environment where the agent runs commands. A sandbox/container installation does not install onto the user's desktop and may disappear when the environment resets. If that environment cannot download or execute the binary, explain the limitation rather than claiming the Skill is ready. Existing CLI installations are reused; a missing PATH entry alone is not a reason to reinstall. For an override, use `IMAGEGEN_INSTALL_DIR` consistently in the installer and subsequent absolute executable path.

## macOS / Linux

```sh
curl --proto '=https' --tlsv1.2 -fLsS https://github.com/okooo5km/imagegen/releases/download/v2.0.1/install.sh -o /tmp/imagegen-install.sh
sh /tmp/imagegen-install.sh
export PATH="$HOME/.local/bin:$PATH"
imagegen --version
```

The installer uses system curl, tar and sha256sum (Linux) or shasum (macOS), verifies the archive, and installs to `~/.local/bin`. Add the export to the user's shell profile if they want it in future terminals. The installer does not rewrite shell profiles. The absolute binary path works immediately in agents even when PATH has not refreshed. macOS binaries are ad-hoc signed, not Developer ID notarized; managed-machine policies may require administrator approval.

## Windows PowerShell

```powershell
Invoke-WebRequest https://github.com/okooo5km/imagegen/releases/download/v2.0.1/install.ps1 -OutFile "$env:TEMP\imagegen-install.ps1"
powershell -NoProfile -ExecutionPolicy Bypass -File "$env:TEMP\imagegen-install.ps1"
$env:Path = "$env:LOCALAPPDATA\imagegen\bin;$env:Path"
imagegen --version
```

The installer uses built-in PowerShell download, SHA-256 and ZIP support, installs to `%LOCALAPPDATA%\imagegen\bin`, and adds this directory to the user PATH. The explicit assignment above updates the current terminal too. No administrator privilege is required. Windows binaries are not Authenticode signed; organization execution policies remain authoritative.

## Alternatives

Download an archive plus its `.sha256` from [GitHub Releases](https://github.com/okooo5km/imagegen/releases/latest), verify the checksum, extract `imagegen`/`imagegen.exe`, and put it in a directory on PATH. Offline deployment can use the same installers with `IMAGEGEN_ARCHIVE_DIR` pointing to a directory holding both files.

Installer overrides: `IMAGEGEN_INSTALL_DIR`, `IMAGEGEN_VERSION` (default v2.0.1), `IMAGEGEN_ARCHIVE_DIR`. Windows additionally supports `IMAGEGEN_NO_MODIFY_PATH=1`. Do not use a different version's checksum.

For developers with Rust 1.88+: `cargo install --git https://github.com/okooo5km/imagegen --tag v2.0.1 --locked`. This is optional; normal users should use a release binary.

## API key

Register at [APIMART](https://apimart.ai), create a key on the [API keys page](https://apimart.ai/keys), and ensure the account has a usable balance. The author's referral link is pending approval; use the official website for now.

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
