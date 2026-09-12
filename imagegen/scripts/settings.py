"""Portable local configuration. Author: okooo5km(十里)."""
from __future__ import annotations

import getpass
import json
import os
from pathlib import Path
import sys

DEFAULT_MODEL = 'gpt-image-2.5-flare'
SIGNUP_URL = 'https://apimart.ai'


def config_dir() -> Path:
    override = os.environ.get('IMAGEGEN_CONFIG_DIR')
    if override:
        return Path(override).expanduser()
    if sys.platform == 'win32':
        return Path(os.environ.get('APPDATA') or Path.home() / 'AppData' / 'Roaming') / 'imagegen'
    return Path(os.environ.get('XDG_CONFIG_HOME') or Path.home() / '.config') / 'imagegen'


def read_config() -> dict:
    path = config_dir() / 'config.json'
    if not path.exists():
        return {}
    data = json.loads(path.read_text(encoding='utf-8-sig'))
    if not isinstance(data, dict):
        raise ValueError(f'Expected a JSON object in {path}')
    return data


def write_private(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    # O_EXCL prevents replacing existing files through symlinks for temporary writes.
    import tempfile
    fd, name = tempfile.mkstemp(prefix='.imagegen-', dir=path.parent)
    temp = Path(name)
    try:
        with os.fdopen(fd, 'w', encoding='utf-8', newline='\n') as stream:
            stream.write(text)
        if os.name != 'nt':
            temp.chmod(0o600)
        temp.replace(path)
    finally:
        temp.unlink(missing_ok=True)


def resolve_key() -> tuple[str | None, str]:
    value = os.environ.get('APIMART_API_KEY', '').strip()
    if value:
        return value, 'APIMART_API_KEY'
    paths = [config_dir() / 'apimart.key']
    # Backward compatibility with the original macOS/Linux skill, also on Windows.
    legacy = Path.home() / '.config' / 'imagegen' / 'apimart.key'
    if not os.environ.get('IMAGEGEN_CONFIG_DIR') and legacy not in paths:
        paths.append(legacy)
    for path in paths:
        if path.is_file():
            if os.name != 'nt' and path.stat().st_mode & 0o077:
                print(f'Warning: restrict key file permissions to 600: {path}', file=sys.stderr)
            for line in path.read_text(encoding='utf-8-sig').splitlines():
                if line.strip():
                    return line.strip(), str(path)
    return None, ''


def require_key() -> str:
    key, _ = resolve_key()
    if not key:
        raise ValueError(f'API key missing. Register: {SIGNUP_URL} ; create a key at https://apimart.ai/keys ; run this CLI with configure --key (hidden input), or set APIMART_API_KEY.')
    if '\n' in key or '\r' in key:
        raise ValueError('API key must be a single line.')
    return key


def configure_key() -> Path:
    if not sys.stdin.isatty():
        raise ValueError('configure --key requires an interactive terminal; use APIMART_API_KEY in CI.')
    key = getpass.getpass('APIMART API key (hidden): ').strip()
    if not key or '\n' in key or '\r' in key:
        raise ValueError('Expected a non-empty, single-line API key.')
    path = config_dir() / 'apimart.key'
    write_private(path, key + '\n')
    return path


def default_model() -> str:
    return os.environ.get('APIMART_IMAGE_MODEL') or read_config().get('default_model') or DEFAULT_MODEL
