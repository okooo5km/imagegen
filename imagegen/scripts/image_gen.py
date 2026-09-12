#!/usr/bin/env python3
"""Portable APIMart image generation CLI. Author: okooo5km(十里).

Revised from the original Apache-2.0 imagegen skill; no dependencies for API calls.
"""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import json
import math
from pathlib import Path
import sys

import client
import models
import settings


def emit(value) -> None:
    print(json.dumps(value, ensure_ascii=False, indent=2))


def positive(value: str) -> float:
    parsed = float(value)
    if not math.isfinite(parsed) or parsed <= 0:
        raise argparse.ArgumentTypeError('Expected a finite positive number.')
    return parsed


def load_prompt(args) -> str:
    if args.prompt_file:
        return Path(args.prompt_file).expanduser().read_text(encoding='utf-8-sig').strip()
    return args.prompt or ''


def build_payload(args) -> dict:
    model = models.resolve_model(args.model)
    payload = {'model': model, 'prompt': load_prompt(args), 'n': args.n,
               'resolution': args.resolution}
    # Omit size on edits unless explicitly requested, preserving input aspect ratio.
    if args.size or not getattr(args, 'image', None):
        payload['size'] = args.size or '1:1'
    for name in ('quality', 'background', 'output_format', 'output_compression', 'moderation',
                 'official_fallback', 'google_search', 'google_image_search'):
        value = getattr(args, name, None)
        if value is not None:
            payload[name] = value
    if model in models.GPT25 and 'quality' not in payload:
        payload['quality'] = 'medium'
    if getattr(args, 'image', None):
        payload['image_urls'] = args.image
    if getattr(args, 'mask', None):
        payload['mask_url'] = args.mask
    return models.validate_payload(payload)


def output_paths(args, count: int, fmt: str = 'png') -> list[Path]:
    if args.out_dir:
        return [Path(args.out_dir).expanduser() / f'image-{i}.{fmt}' for i in range(1, count + 1)]
    base = Path(args.out or f'output/imagegen/image.{fmt}').expanduser()
    if not base.suffix:
        base = base.with_suffix('.' + fmt)
    return [base] if count == 1 else [base.with_name(f'{base.stem}-{i}{base.suffix}') for i in range(1, count + 1)]


def preflight_outputs(paths: list[Path], force: bool) -> None:
    # Check all plausible actual extensions before a paid request.
    seen = set()
    for path in paths:
        canonical = str(path.resolve())
        if canonical in seen:
            raise ValueError(f'Duplicate output: {path}')
        seen.add(canonical)
        if path.exists() and path.is_dir():
            raise ValueError(f'Output is a directory: {path}')
        variants = {path} | {path.with_suffix('.' + ext) for ext in ('png', 'jpg', 'jpeg', 'webp', 'gif')}
        if not force:
            for candidate in variants:
                if candidate.exists():
                    raise ValueError(f'Output exists: {candidate}. Choose another path or use --force.')
        ancestor = path.parent
        while not ancestor.exists():
            ancestor = ancestor.parent
        if not ancestor.is_dir():
            raise ValueError(f'Output parent is not a directory: {ancestor}')


def persist_task(task: str, payload: dict, paths: list[Path]) -> Path:
    # Keep a receipt without prompt, key, input files, or reference URLs.
    import hashlib
    name = hashlib.sha256(task.encode()).hexdigest()[:24] + '.json'
    path = settings.config_dir() / 'tasks' / name
    settings.write_private(path, json.dumps({'task_id': task, 'model': payload['model'],
                                            'outputs': [str(p.resolve()) for p in paths]}, indent=2) + '\n')
    return path


def save_result(data: dict, paths: list[Path], force: bool) -> list[str]:
    urls = client.result_urls(data)
    saved = []
    for index, url in enumerate(urls):
        if index >= len(paths):
            break
        raw = client.download(url)
        fmt = client.image_format(raw)
        path = paths[index]
        if path.suffix.lower().lstrip('.') not in ({'jpg', 'jpeg'} if fmt == 'jpeg' else {fmt}):
            path = path.with_suffix('.' + fmt)
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open('wb' if force else 'xb') as stream:
            stream.write(raw)
        saved.append(str(path.resolve()))
        print(f'Saved: {path.resolve()}', file=sys.stderr)
    if len(urls) != len(paths):
        raise client.APIError(f'Task returned {len(urls)} images, expected {len(paths)}; saved {len(saved)}. Inspect task before requesting more.')
    return saved


def execute(payload: dict, paths: list[Path], args) -> dict:
    preflight_outputs(paths, args.force)
    if args.dry_run:
        return {'dry_run': True, 'payload': payload, 'outputs': [str(p) for p in paths],
                'note': 'Local references will be uploaded only during a real run.'}
    # Validate paths are writable before uploads/submission; no API call yet.
    import tempfile
    for path in paths:
        path.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryFile(dir=path.parent):
            pass
    key = settings.require_key()
    payload = dict(payload)
    if payload.get('mask_url'):
        client.validate_mask(payload['mask_url'], payload['image_urls'][0], allow_network=True)
    if payload.get('image_urls'):
        payload['image_urls'] = [client.upload(value, key) for value in payload['image_urls']]
    if payload.get('mask_url'):
        payload['mask_url'] = client.upload(payload['mask_url'], key)
    task = client.submit(payload, key)
    print(f'Task: {task} | model: {payload["model"]}', file=sys.stderr, flush=True)
    try:
        receipt = persist_task(task, payload, paths)
        print(f'Receipt: {receipt}', file=sys.stderr)
    except OSError:
        print(f'Warning: receipt could not be saved. Keep task ID {task}; use fetch to resume.', file=sys.stderr)
    data = client.poll(task, key, timeout=args.timeout, interval=args.poll_interval)
    saved = save_result(data, paths, args.force)
    return {'task_id': task, 'model': payload['model'], 'outputs': saved,
            'cost': data.get('cost'), 'usage': data.get('usage')}


def run_image(args) -> None:
    payload = build_payload(args)
    emit(execute(payload, output_paths(args, payload['n'], payload.get('output_format', 'png')), args))


def run_fetch(args) -> None:
    paths = output_paths(args, args.n)
    preflight_outputs(paths, args.force)
    data = client.poll(args.task_id, settings.require_key(), timeout=args.timeout, interval=args.poll_interval)
    emit({'task_id': args.task_id, 'outputs': save_result(data, paths, args.force),
          'cost': data.get('cost'), 'usage': data.get('usage')})


def run_batch(args) -> None:
    lines = Path(args.input).expanduser().read_text(encoding='utf-8-sig').splitlines()
    jobs = []
    root = Path(args.out_dir).expanduser().resolve()
    for line_no, line in enumerate(lines, 1):
        if not line.strip():
            continue
        job = json.loads(line)
        if not isinstance(job, dict):
            raise ValueError(f'Line {line_no}: expected a JSON object.')
        job = dict(job)
        out = job.pop('out', f'job-{len(jobs)+1}.png')
        if not isinstance(out, str):
            raise ValueError('Batch out must be a relative filename.')
        path = (root / out).resolve()
        if not path.is_relative_to(root) or path == root:
            raise ValueError('Batch output must remain inside --out-dir.')
        job['model'] = models.resolve_model(job.get('model', args.model))
        job.setdefault('n', 1)
        job.setdefault('resolution', '1k')
        if not job.get('image_urls'):
            job.setdefault('size', '1:1')
        if job['model'] in models.GPT25:
            job.setdefault('quality', 'medium')
        models.validate_payload(job)
        out_args = argparse.Namespace(out=str(path), out_dir=None)
        paths = output_paths(out_args, job['n'], job.get('output_format', 'png'))
        jobs.append((job, paths))
    if not 1 <= len(jobs) <= 500:
        raise ValueError('Batch must contain 1..500 non-empty JSONL jobs.')
    all_paths = [p for _, paths in jobs for p in paths]
    preflight_outputs(all_paths, args.force)
    # Different requested extensions may resolve to the same downloaded extension.
    stems = [str(p.with_suffix('').resolve()).casefold() for p in all_paths]
    if len(stems) != len(set(stems)):
        raise ValueError('Batch outputs collide after format normalization.')
    if args.dry_run:
        emit({'jobs': [execute(payload, paths, args) for payload, paths in jobs]})
        return
    settings.require_key()
    failures = 0
    with ThreadPoolExecutor(max_workers=args.concurrency) as pool:
        futures = {pool.submit(execute, payload, paths, args): i for i, (payload, paths) in enumerate(jobs, 1)}
        for future in as_completed(futures):
            try:
                emit({'job': futures[future], **future.result()})
            except (ValueError, OSError, client.APIError) as exc:
                failures += 1
                print(f'Job {futures[future]} failed: {exc}', file=sys.stderr)
    if failures:
        raise client.APIError(f'{failures} batch job(s) failed. Completed outputs remain saved; do not rerun successful jobs.')


def run_models(args) -> None:
    rows = models.catalog(args.live, settings.require_key() if args.live else None)
    failed = 0
    if args.prices:
        for row in rows:
            try:
                row['price_info'] = models.pricing(row['id'])
            except client.APIError as exc:
                row['price_info'] = {'status': 'unavailable', 'error': str(exc)}
                failed += 1
    if args.json:
        emit({'models': rows, 'price_failures': failed})
    else:
        for row in rows:
            label = 'runnable' if row['runnable'] else 'discovery only'
            print(f'{row["id"]} [{label}]')
            if 'price_info' in row:
                print_price(row['price_info'])
    if failed:
        raise client.APIError(f'{failed} price lookup(s) failed; missing prices are not zero.')


def print_price(info: dict) -> None:
    print(f'  {info.get("status", "unavailable")}')
    pricing = info.get('pricing', {})
    rates = pricing.get('effective_rates', {})
    if rates:
        print(f'  {pricing.get("unit", "unit unavailable")} | group={pricing.get("group", "unknown")}')
        for name, value in rates.items():
            print(f'  {name}: {value}')
    if info.get('legacy_quote'):
        print('  ' + json.dumps(info['legacy_quote'], ensure_ascii=False))
    print('  Public quote; account-specific charges may differ.')


def run_prices(args) -> None:
    info = models.pricing(args.model or settings.default_model())
    if args.json:
        emit(info)
    else:
        print(info['model'])
        print_price(info)
        print(info['source'])


def run_configure(args) -> None:
    if args.key:
        print(f'Key saved: {settings.configure_key()}')
    if args.default_model:
        model = models.resolve_model(args.default_model)
        conf = settings.read_config()
        conf['default_model'] = model
        settings.write_private(settings.config_dir() / 'config.json', json.dumps(conf, indent=2) + '\n')
        print(f'Default model saved: {model}')
    if not args.key and not args.default_model:
        emit({'config_dir': str(settings.config_dir()), 'default_model': settings.default_model(),
              'signup': settings.SIGNUP_URL})


def run_check(args) -> None:
    _, source = settings.resolve_key()
    if not source:
        settings.require_key()
    print(f'API key configured via {source}. Local check only; not authenticated.')


def add_outputs(parser) -> None:
    outputs = parser.add_mutually_exclusive_group()
    outputs.add_argument('--out')
    outputs.add_argument('--out-dir')
    parser.add_argument('--force', action='store_true')
    parser.add_argument('--timeout', type=positive, default=600)
    parser.add_argument('--poll-interval', type=positive, default=4)


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description='Generate/edit images via APIMart; discover models and current prices.')
    sub = root.add_subparsers(dest='command', required=True)
    for name in ('generate', 'edit'):
        p = sub.add_parser(name)
        p.add_argument('--model', help='Explicit model, or auto: env > saved default > gpt-image-2.5-flare')
        prompt = p.add_mutually_exclusive_group(required=True)
        prompt.add_argument('--prompt')
        prompt.add_argument('--prompt-file')
        p.add_argument('--n', type=int, default=1)
        p.add_argument('--size', help='Aspect ratio or GPT Image 2.5 exact dimensions; edit default preserves input ratio')
        p.add_argument('--resolution', choices=('1k', '2k', '4k'), default='1k')
        p.add_argument('--quality', choices=('auto', 'low', 'medium', 'high', 'xhigh', 'max'))
        p.add_argument('--background', choices=('auto', 'opaque', 'transparent'))
        p.add_argument('--output-format', choices=('png', 'jpeg', 'webp'))
        p.add_argument('--output-compression', type=int)
        p.add_argument('--moderation', choices=('auto', 'low'))
        for flag in ('official-fallback', 'google-search', 'google-image-search'):
            p.add_argument('--' + flag, action=argparse.BooleanOptionalAction, default=None)
        p.add_argument('--image', action='append', required=name == 'edit', help='Local image or public HTTP(S) URL; repeat for references')
        p.add_argument('--mask', help='gpt-image-2-official only; PNG alpha mask matching first input')
        p.add_argument('--dry-run', action='store_true')
        add_outputs(p)
        p.set_defaults(func=run_image)
    p = sub.add_parser('generate-batch', help='Validate every JSONL job, then run a bounded concurrent batch')
    p.add_argument('--input', required=True)
    p.add_argument('--out-dir', required=True)
    p.add_argument('--model')
    p.add_argument('--concurrency', type=int, choices=range(1, 9), default=2)
    p.add_argument('--dry-run', action='store_true')
    p.add_argument('--force', action='store_true')
    p.add_argument('--timeout', type=positive, default=600)
    p.add_argument('--poll-interval', type=positive, default=4)
    p.set_defaults(func=run_batch)
    p = sub.add_parser('fetch', help='Resume/download an existing task without submitting another generation')
    p.add_argument('task_id')
    p.add_argument('--n', type=int, choices=range(1, 5), default=1)
    add_outputs(p)
    p.set_defaults(func=run_fetch)
    p = sub.add_parser('models', help='List bundled runnable profiles, or use --live for account image catalog')
    p.add_argument('--live', action='store_true')
    p.add_argument('--prices', action='store_true')
    p.add_argument('--json', action='store_true')
    p.set_defaults(func=run_models)
    p = sub.add_parser('prices', help='Get public current pricing; no API key required')
    p.add_argument('--model')
    p.add_argument('--json', action='store_true')
    p.set_defaults(func=run_prices)
    p = sub.add_parser('configure', help='Configure hidden key input or a persistent default model')
    p.add_argument('--key', action='store_true')
    p.add_argument('--default-model')
    p.set_defaults(func=run_configure)
    p = sub.add_parser('check-key', help='Local presence check; no network request')
    p.set_defaults(func=run_check)
    return root


def main(argv=None) -> int:
    # Windows redirected consoles may otherwise default to a legacy code page.
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, 'reconfigure'):
            stream.reconfigure(encoding='utf-8', errors='replace')
    args = parser().parse_args(argv)
    try:
        args.func(args)
    except (ValueError, OSError, client.APIError) as exc:
        print(f'Error: {exc}', file=sys.stderr)
        return 2 if isinstance(exc, ValueError) else 1
    except KeyboardInterrupt:
        print('Interrupted. If submitted, use the printed task ID with fetch; do not resubmit.', file=sys.stderr)
        return 130
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
