"""APIMart HTTP, uploads and async tasks. Author: okooo5km(十里)."""
from __future__ import annotations

import json
from pathlib import Path
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid

API_BASE = 'https://api.apimart.ai'
MAX_UPLOAD = 20 * 1024 * 1024
MAX_DOWNLOAD = 100 * 1024 * 1024


class APIError(RuntimeError):
    pass


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        # Authenticated API endpoints must not forward credentials on redirects.
        return None


def request(path: str, *, key: str | None = None, payload=None,
            body: bytes | None = None, content_type: str | None = None, timeout: float = 60) -> dict:
    headers = {'Accept': 'application/json', 'User-Agent': 'apimart-imagegen/1.0'}
    if key:
        headers['Authorization'] = 'Bearer ' + key
    if payload is not None:
        body = json.dumps(payload, ensure_ascii=False).encode('utf-8')
        content_type = 'application/json'
    if content_type:
        headers['Content-Type'] = content_type
    req = urllib.request.Request(API_BASE + path, data=body, headers=headers)
    try:
        with urllib.request.build_opener(NoRedirect()).open(req, timeout=timeout) as response:
            raw = response.read(8 * 1024 * 1024 + 1)
        if len(raw) > 8 * 1024 * 1024:
            raise APIError('API response exceeds 8 MiB.')
        result = json.loads(raw)
    except urllib.error.HTTPError as exc:
        # Do not echo server bodies: they may reflect prompts, credentials or uploaded URLs.
        exc.close()
        raise APIError(f'HTTP {exc.code} from {path.split("?")[0]}. Check credentials, balance, parameters or service status. No automatic resubmission.') from None
    except (urllib.error.URLError, TimeoutError, OSError) as exc:
        raise APIError(f'Network failure ({type(exc).__name__}). Submission may have reached the server; check APIMart task history before retrying.') from None
    except (ValueError, UnicodeError):
        raise APIError('API returned invalid JSON.') from None
    if not isinstance(result, dict):
        raise APIError('API response must be a JSON object.')
    if result.get('success') is False or result.get('error') or result.get('code', 200) not in (0, 200):
        raise APIError('API reported an error. Check APIMart task history; no automatic resubmission.')
    return result


def unwrap(response: dict) -> dict:
    data = response.get('data', response)
    if isinstance(data, list):
        data = data[0] if data else None
    if not isinstance(data, dict):
        raise APIError('Unexpected API data shape.')
    return data


def image_format(data: bytes) -> str:
    if data.startswith(b'\x89PNG\r\n\x1a\n'):
        return 'png'
    if data.startswith(b'\xff\xd8\xff'):
        return 'jpeg'
    if data[:6] in (b'GIF87a', b'GIF89a'):
        return 'gif'
    if data[:4] == b'RIFF' and data[8:12] == b'WEBP':
        return 'webp'
    raise ValueError('Expected a PNG, JPEG, WebP or GIF image; unsupported or invalid file.')


def public_url(value: str) -> bool:
    parts = urllib.parse.urlsplit(value)
    return parts.scheme in ('http', 'https') and bool(parts.netloc) and not parts.username and not parts.password


def validate_ref(value: str) -> None:
    if public_url(value):
        return
    if value.startswith(('data:', 'http:', 'https:')):
        raise ValueError('Reference images must be public HTTP(S) URLs or local image files; data URIs are no longer supported.')
    path = Path(value).expanduser()
    if not path.is_file():
        raise ValueError(f'Image file not found: {path}')
    if path.stat().st_size > MAX_UPLOAD:
        raise ValueError(f'Image exceeds the 20 MiB upload limit: {path}')
    with path.open('rb') as stream:
        image_format(stream.read(16))


def upload(value: str, key: str) -> str:
    validate_ref(value)
    if public_url(value):
        return value
    data = Path(value).expanduser().read_bytes()
    fmt = image_format(data)
    boundary = 'imagegen' + uuid.uuid4().hex
    # Generic ASCII filename avoids Unicode/quote/newline differences in multipart headers.
    head = (f'--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="image.{fmt}"\r\nContent-Type: image/{fmt}\r\n\r\n').encode('ascii')
    body = head + data + f'\r\n--{boundary}--\r\n'.encode('ascii')
    result = unwrap(request('/v1/uploads/images', key=key, body=body,
                            content_type=f'multipart/form-data; boundary={boundary}'))
    url = result.get('url')
    if not isinstance(url, str) or not public_url(url):
        raise APIError('Upload did not return a public HTTP(S) URL.')
    return url


def submit(payload: dict, key: str) -> str:
    data = unwrap(request('/v1/images/generations', key=key, payload=payload))
    task = data.get('task_id') or data.get('id')
    if not isinstance(task, str) or not task:
        raise APIError('Submission returned no task ID. Check APIMart task history before retrying.')
    return task


def poll(task: str, key: str, *, timeout: float = 600, interval: float = 4) -> dict:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        remaining = deadline - time.monotonic()
        data = unwrap(request('/v1/tasks/' + urllib.parse.quote(task, safe=''), key=key,
                              timeout=max(0.1, min(60, remaining))))
        status = str(data.get('status', '')).lower()
        if status in ('completed', 'succeeded', 'success'):
            return data
        if status in ('failed', 'error', 'cancelled', 'canceled'):
            raise APIError(f'Task {task} ended with status {status}. Check its details in APIMart.')
        time.sleep(max(0, min(interval, deadline - time.monotonic())))
    raise APIError(f'Task {task} is still pending after {timeout:g}s. Resume with: image_gen.py fetch {task}. Do not submit it again.')


def result_urls(data: dict) -> list[str]:
    images = (data.get('result') or {}).get('images') or data.get('images') or []
    urls = []
    for item in images:
        field = item.get('url') if isinstance(item, dict) else None
        if isinstance(field, str):
            field = [field]
        if isinstance(field, list):
            urls.extend(url for url in field if isinstance(url, str) and public_url(url))
    if not urls:
        raise APIError('Completed task returned no valid image URLs.')
    return urls


def download(url: str) -> bytes:
    if not public_url(url):
        raise ValueError('Invalid image download URL.')
    # Separate unauthenticated opener: never attach APIMart credentials to image storage.
    try:
        with urllib.request.urlopen(url, timeout=60) as response:
            data = response.read(MAX_DOWNLOAD + 1)
    except (urllib.error.URLError, TimeoutError, OSError):
        raise APIError('Image download failed; resume the same task with fetch.') from None
    if len(data) > MAX_DOWNLOAD:
        raise APIError('Image download exceeds 100 MiB.')
    image_format(data)
    return data


def validate_mask(mask: str, first: str, *, allow_network: bool = False) -> None:
    """Validate local masks offline; defer remote inputs until an actual run."""
    if not allow_network and (public_url(mask) or public_url(first)):
        return
    try:
        from PIL import Image
        from io import BytesIO
    except ImportError:
        raise ValueError('Mask validation requires Pillow: python -m pip install Pillow') from None
    mask_bytes = download(mask) if public_url(mask) else Path(mask).expanduser().read_bytes()
    first_bytes = download(first) if public_url(first) else Path(first).expanduser().read_bytes()
    with Image.open(BytesIO(mask_bytes)) as matte, Image.open(BytesIO(first_bytes)) as reference:
        if matte.format != 'PNG' or 'A' not in matte.getbands() or matte.size != reference.size:
            raise ValueError('Mask must be a PNG with an alpha channel and match the first reference dimensions.')
