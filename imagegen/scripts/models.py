"""Validated image model profiles and live catalog/pricing. Author: okooo5km(十里)."""
from __future__ import annotations

from datetime import datetime, timezone
import re
import urllib.parse

import client
import settings

GPT25 = {'gpt-image-2.5-flare', 'gpt-image-2.5-sunburst'}
RATIOS25 = {'auto', '1:1', '3:2', '2:3', '4:3', '3:4', '5:4', '4:5', '16:9', '9:16', '2:1', '1:2', '21:9', '9:21', '3:1', '1:3'}
LEGACY_RATIOS = {'1:1', '16:9', '9:16', '4:3', '3:4', '3:2', '2:3', '21:9'}
COMMON = {'model', 'prompt', 'n', 'size', 'resolution', 'image_urls'}
GPT_OPTIONS = {'quality', 'background', 'output_format', 'output_compression', 'moderation'}
PROFILES = {
    'gpt-image-2.5-flare': COMMON | GPT_OPTIONS,
    'gpt-image-2.5-sunburst': COMMON | GPT_OPTIONS,
    'gpt-image-2-official': COMMON | GPT_OPTIONS | {'mask_url'},
    'gpt-image-2': COMMON | {'official_fallback'},
    'gemini-3.1-flash-image-preview': COMMON | {'google_search', 'google_image_search', 'official_fallback'},
}


def resolve_model(value: str | None) -> str:
    model = settings.default_model() if value in (None, '', 'auto') else value
    if model not in PROFILES:
        raise ValueError(f'No tested generation profile for {model}. Runnable models: {", ".join(PROFILES)}. models --live also lists discovery-only models.')
    return model


def validate_payload(payload: dict) -> dict:
    model = payload['model']
    if model not in PROFILES:
        raise ValueError(f'Unsupported model: {model}')
    unsupported = set(payload) - PROFILES[model]
    if unsupported:
        raise ValueError(f'{model} does not support: {", ".join(sorted(unsupported))}. Choose another model explicitly.')
    if not isinstance(payload.get('prompt'), str) or not payload['prompt'].strip():
        raise ValueError('A non-empty prompt is required.')
    n = payload.get('n', 1)
    cap = 1 if model == 'gpt-image-2' else 4
    if type(n) is not int or not 1 <= n <= cap:
        raise ValueError(f'n must be an integer between 1 and {cap} for {model}.')
    size = payload.get('size', 'auto' if model in GPT25 else '1:1')
    match = re.fullmatch(r'([1-9][0-9]*)x([1-9][0-9]*)', str(size))
    if match and model in GPT25:
        w, h = map(int, match.groups())
        if w % 16 or h % 16 or max(w, h) > 3840 or max(w, h) > 3 * min(w, h) or not 655360 <= w * h <= 8294400:
            raise ValueError('Exact size requires multiples of 16, sides <=3840, ratio <=3:1 and 655360..8294400 total pixels.')
    elif size not in (RATIOS25 if model in GPT25 else LEGACY_RATIOS):
        raise ValueError(f'Unsupported size for {model}: {size}')
    resolution = payload.get('resolution', '1k')
    if resolution not in ('1k', '2k', '4k'):
        raise ValueError('resolution must be 1k, 2k or 4k.')
    if model not in GPT25 and resolution == '4k' and size not in {'16:9', '9:16', '21:9'}:
        raise ValueError('Legacy model profile only supports 4k for 16:9, 9:16 or 21:9.')
    if 'quality' in payload:
        qualities = {'auto', 'low', 'medium', 'high'} | ({'xhigh', 'max'} if model in GPT25 else set())
        if payload['quality'] not in qualities:
            raise ValueError(f'quality must be one of {", ".join(sorted(qualities))}.')
    fmt = payload.get('output_format', 'png')
    if fmt not in ('png', 'jpeg', 'webp'):
        raise ValueError('output_format must be png, jpeg or webp.')
    if payload.get('background', 'auto') not in ('auto', 'opaque', 'transparent'):
        raise ValueError('background must be auto, opaque or transparent.')
    if payload.get('background') == 'transparent' and fmt == 'jpeg':
        raise ValueError('Transparent backgrounds require PNG or WebP.')
    if payload.get('moderation', 'auto') not in ('auto', 'low'):
        raise ValueError('moderation must be auto or low.')
    if 'output_compression' in payload:
        value = payload['output_compression']
        if type(value) is not int or not 0 <= value <= 100 or fmt not in ('jpeg', 'webp'):
            raise ValueError('output_compression is 0..100 and requires JPEG or WebP.')
    for flag in ('official_fallback', 'google_search', 'google_image_search'):
        if flag in payload and type(payload[flag]) is not bool:
            raise ValueError(f'{flag} must be a boolean.')
    if 'image_urls' in payload:
        refs = payload['image_urls']
        cap = 16 if model in GPT25 or model == 'gpt-image-2-official' else 8
        if not isinstance(refs, list) or not 1 <= len(refs) <= cap or not all(isinstance(x, str) for x in refs):
            raise ValueError(f'image_urls must contain 1..{cap} image paths or URLs.')
        for value in refs:
            client.validate_ref(value)
    if 'mask_url' in payload:
        if not payload.get('image_urls'):
            raise ValueError('A mask requires a reference image.')
        client.validate_ref(payload['mask_url'])
        client.validate_mask(payload['mask_url'], payload['image_urls'][0])
    return payload


def catalog(live: bool = False, key: str | None = None) -> list[dict]:
    if not live:
        return [{'id': model, 'category': 'image', 'runnable': True, 'source': 'bundled profile'} for model in PROFILES]
    response = client.request('/v1/models?expand=parameters&category=image', key=key)
    data = response.get('data')
    if not isinstance(data, list):
        raise client.APIError('Model catalog returned no list.')
    return [{**item, 'runnable': item.get('id') in PROFILES} for item in data
            if isinstance(item, dict) and item.get('category') == 'image' and isinstance(item.get('id'), str)]


def pricing(model: str) -> dict:
    path = '/api/pricing/model?' + urllib.parse.urlencode({'model': model})
    # Public endpoint: deliberately omit Authorization, even when a key is configured.
    data = client.unwrap(client.request(path))
    view = data.get('pricing')
    result = {'model': model, 'source': client.API_BASE + path,
              'retrieved_at': datetime.now(timezone.utc).isoformat(),
              'scope': 'public default group; account charges may differ'}
    if isinstance(view, dict):
        result['pricing'] = view
        result['status'] = 'available' if view.get('effective_rates') else 'rates unavailable'
    else:
        # Non-token contracts do not promise payable group-adjusted amounts.
        result['status'] = 'legacy quote; payable rate unverified'
        result['legacy_quote'] = {k: data[k] for k in ('quota_type', 'model_price', 'resolution_prices', 'size_quality_prices', 'discount_rate', 'discount_percent') if k in data}
        if not result['legacy_quote']:
            result['status'] = 'unavailable'
    return result
