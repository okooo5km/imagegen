# Setup

Requires Python 3.10 or later and outbound HTTPS. Core generation, upload, model listing and pricing use Python's standard library.

Register at [APIMART](https://apimart.ai), then create a key on the [API keys page](https://apimart.ai/keys). Generation uses the user's APIMART account and balance.

Run the following in a terminal the user controls, replacing the path with the installed skill's actual path:

```text
python "<skill-path>/scripts/image_gen.py" configure --key
python "<skill-path>/scripts/image_gen.py" check-key
```

Use `python3` on macOS/Linux, or `py -3` on Windows if `python` is unavailable. `configure --key` uses hidden input, writes outside the repository, and never accepts a key as a command-line argument. Run this command yourself; do not paste your key into an AI conversation.

Configuration directory:

| Platform | Default |
| --- | --- |
| macOS / Linux | `$XDG_CONFIG_HOME/imagegen`, otherwise `~/.config/imagegen` |
| Windows | `%APPDATA%\imagegen`, otherwise `~/AppData/Roaming/imagegen` |
| Override | `IMAGEGEN_CONFIG_DIR` on any platform |

Key precedence: `APIMART_API_KEY`, current config directory's `apimart.key`, legacy `~/.config/imagegen/apimart.key`. The legacy fallback is disabled when `IMAGEGEN_CONFIG_DIR` is set. UTF-8 BOM and CRLF are accepted. Unix key files are created with mode 600. Windows access is governed by the user profile's ACL; Unix chmod semantics are not assumed.

For CI, supply `APIMART_API_KEY` through your CI's secret environment facility. The CLI never reads a project `.env` automatically.

`configure --default-model gpt-image-2.5-sunburst` persists a model choice in `config.json`. Precedence: explicit `--model` > `APIMART_IMAGE_MODEL` > config.json > `gpt-image-2.5-flare`. `configure` without flags reports paths and effective default without revealing credentials.

Optional dependency for local chroma-key processing and alpha-mask validation: `python -m pip install Pillow`. On distributions that protect system Python, use a virtual environment or `uv run --with pillow ...`. No shell activation is necessary when you invoke the environment's interpreter by its full path.
