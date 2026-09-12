"""Behavioral tests: offline, isolated, no APIMart account or billed requests."""
from __future__ import annotations

import base64
from contextlib import redirect_stdout, redirect_stderr
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import unittest
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parents[1] / 'imagegen' / 'scripts'
sys.path.insert(0, str(SCRIPTS))
import client  # noqa: E402
import image_gen as cli  # noqa: E402
import models  # noqa: E402
import settings  # noqa: E402

PNG = base64.b64decode('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jJ1sAAAAASUVORK5CYII=')


class Isolated(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='imagegen 测试 ')
        self.root = Path(self.temp.name)
        self.env = patch.dict(os.environ, {'IMAGEGEN_CONFIG_DIR': str(self.root / 'config'),
                                          'APIMART_API_KEY': '', 'APIMART_IMAGE_MODEL': ''})
        self.env.start()

    def tearDown(self):
        self.env.stop()
        self.temp.cleanup()

    def invoke(self, *args):
        env = dict(os.environ)
        env['PYTHONIOENCODING'] = 'utf-8'
        return subprocess.run([sys.executable, str(SCRIPTS / 'image_gen.py'), *args], cwd=self.root,
                              env=env, capture_output=True, text=True, encoding='utf-8', timeout=30)

    def payload(self, **kwargs):
        return models.validate_payload({'model': settings.DEFAULT_MODEL, 'prompt': 'test', **kwargs})


class ConfigTests(Isolated):
    def test_missing_key_is_actionable_and_local(self):
        result = self.invoke('check-key')
        self.assertEqual(result.returncode, 2)
        self.assertIn('configure --key', result.stderr)

    def test_bom_crlf_key_and_env_override(self):
        path = settings.config_dir() / 'apimart.key'
        settings.write_private(path, '\ufeff\r\nfile-test-secret\r\n')
        self.assertEqual(settings.resolve_key()[0], 'file-test-secret')
        with patch.dict(os.environ, {'APIMART_API_KEY': ' env-test-secret '}):
            self.assertEqual(settings.resolve_key()[0], 'env-test-secret')
            result = self.invoke('check-key')
            self.assertEqual(result.returncode, 0)
            self.assertNotIn('env-test-secret', result.stdout + result.stderr)
        if os.name != 'nt':
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)

    def test_default_precedence(self):
        self.assertEqual(models.resolve_model(None), 'gpt-image-2.5-flare')
        settings.write_private(settings.config_dir() / 'config.json', '{"default_model":"gpt-image-2.5-sunburst"}')
        self.assertEqual(models.resolve_model('auto'), 'gpt-image-2.5-sunburst')
        with patch.dict(os.environ, {'APIMART_IMAGE_MODEL': 'gpt-image-2'}):
            self.assertEqual(models.resolve_model(None), 'gpt-image-2')
            self.assertEqual(models.resolve_model('gpt-image-2.5-flare'), 'gpt-image-2.5-flare')

    def test_platform_config_roots(self):
        with patch.dict(os.environ, {'IMAGEGEN_CONFIG_DIR': '', 'APPDATA': str(self.root / 'roaming')}):
            with patch.object(sys, 'platform', 'win32'):
                self.assertEqual(settings.config_dir(), self.root / 'roaming' / 'imagegen')
            with patch.object(sys, 'platform', 'linux'), patch.dict(os.environ, {'XDG_CONFIG_HOME': str(self.root / 'xdg')}):
                self.assertEqual(settings.config_dir(), self.root / 'xdg' / 'imagegen')

    def test_configure_hidden_input_requires_terminal(self):
        result = self.invoke('configure', '--key')
        self.assertEqual(result.returncode, 2)
        self.assertFalse((settings.config_dir() / 'apimart.key').exists())

    def test_persist_default_and_malformed_config(self):
        result = self.invoke('configure', '--default-model', 'gpt-image-2.5-sunburst')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(settings.default_model(), 'gpt-image-2.5-sunburst')
        settings.write_private(settings.config_dir() / 'config.json', '[]')
        result = self.invoke('configure')
        self.assertEqual(result.returncode, 2)
        self.assertNotIn('Traceback', result.stderr)


class PayloadTests(Isolated):
    def test_default_dry_run_has_no_side_effect(self):
        result = self.invoke('generate', '--prompt', '一只狐狸', '--out', '图像 空格/a.png', '--dry-run')
        self.assertEqual(result.returncode, 0, result.stderr)
        payload = json.loads(result.stdout)['payload']
        self.assertEqual(payload['model'], 'gpt-image-2.5-flare')
        self.assertEqual(payload['quality'], 'medium')
        self.assertFalse((self.root / '图像 空格').exists())
        self.assertFalse(settings.config_dir().exists())

    def test_unicode_prompt_file_and_reference(self):
        image = self.root / '参考 图.png'
        image.write_bytes(PNG)
        prompt = self.root / '提示 词.txt'
        prompt.write_text('\ufeff只修改背景\r\n保留主体', encoding='utf-8')
        result = self.invoke('edit', '--image', str(image), '--prompt-file', str(prompt), '--dry-run')
        self.assertEqual(result.returncode, 0, result.stderr)
        payload = json.loads(result.stdout)['payload']
        self.assertNotIn('size', payload)
        self.assertEqual(payload['image_urls'], [str(image)])
        self.assertIn('保留主体', payload['prompt'])

    def test_25_sizes_quality_and_transparency(self):
        for model in models.GPT25:
            for size in models.RATIOS25 | {'1600x1200'}:
                with self.subTest(model=model, size=size):
                    self.payload(model=model, size=size, resolution='4k', quality='max')
        self.payload(background='transparent', output_format='webp')
        for kwargs in ({'size': '1000x1000'}, {'size': '16x16'}, {'size': '3840x3840'},
                       {'background': 'transparent', 'output_format': 'jpeg'}, {'n': True}, {'n': 5},
                       {'output_compression': 80}, {'quality': 'ultra'}, {'google_search': True}):
            with self.subTest(kwargs=kwargs), self.assertRaises(ValueError):
                self.payload(**kwargs)

    def test_no_silent_parameter_drop_or_model_fallback(self):
        with self.assertRaises(ValueError):
            self.payload(model='gpt-image-2', quality='high')
        with self.assertRaises(ValueError):
            models.resolve_model('unadapted-future-model')

    def test_reject_data_uri_missing_input_and_ref_limit(self):
        for value in ('data:image/png;base64,abc', 'missing.png'):
            with self.subTest(value=value), self.assertRaises(ValueError):
                self.payload(image_urls=[value])
        with self.assertRaises(ValueError):
            self.payload(image_urls=['https://example.com/a.png'] * 17)

    def test_existing_output_blocks_api_call(self):
        target = self.root / 'existing.png'
        target.write_bytes(PNG)
        result = self.invoke('generate', '--prompt', 'test', '--out', str(target))
        self.assertEqual(result.returncode, 2)
        self.assertIn('Output exists', result.stderr)
        self.assertEqual(target.read_bytes(), PNG)

    def test_timeout_rejects_nonfinite_values(self):
        for value in ('nan', 'inf', '-1', '0'):
            result = self.invoke('fetch', 'task1', '--timeout', value)
            self.assertEqual(result.returncode, 2)

    def test_local_mask_alpha_and_dimensions(self):
        from PIL import Image
        reference = self.root / 'reference.png'
        mask = self.root / 'mask.png'
        Image.new('RGB', (16, 16), 'blue').save(reference)
        Image.new('RGBA', (16, 16), (0, 0, 0, 0)).save(mask)
        self.payload(model='gpt-image-2-official', image_urls=[str(reference)], mask_url=str(mask))
        Image.new('RGB', (16, 16), 'black').save(mask)
        with self.assertRaises(ValueError):
            self.payload(model='gpt-image-2-official', image_urls=[str(reference)], mask_url=str(mask))
        Image.new('RGBA', (8, 8), (0, 0, 0, 0)).save(mask)
        with self.assertRaises(ValueError):
            self.payload(model='gpt-image-2-official', image_urls=[str(reference)], mask_url=str(mask))

    def test_batch_validates_all_jobs_before_paid_work(self):
        jobs = self.root / 'jobs.jsonl'
        jobs.write_text('{"prompt":"valid"}\n{"prompt":"invalid","quality":"wrong"}\n', encoding='utf-8')
        args = cli.parser().parse_args(['generate-batch', '--input', str(jobs), '--out-dir', str(self.root / 'out')])
        with patch.object(client, 'submit') as submit, self.assertRaises(ValueError):
            cli.run_batch(args)
        submit.assert_not_called()

    def test_batch_traversal_and_extension_collisions(self):
        for lines in ([{'prompt': 'a', 'out': '../escape.png'}],
                      [{'prompt': 'a', 'out': 'same.png'}, {'prompt': 'b', 'out': 'same.webp'}]):
            jobs = self.root / 'jobs.jsonl'
            jobs.write_text('\n'.join(json.dumps(x) for x in lines), encoding='utf-8')
            result = self.invoke('generate-batch', '--input', str(jobs), '--out-dir', 'out', '--dry-run')
            self.assertEqual(result.returncode, 2, result.stdout)
            self.assertFalse((self.root / 'out').exists())

    def test_batch_dry_run(self):
        jobs = self.root / '作业.jsonl'
        jobs.write_text('\ufeff{"prompt":"茶杯"}\r\n{"prompt":"狐狸","model":"gpt-image-2.5-sunburst"}\r\n', encoding='utf-8')
        result = self.invoke('generate-batch', '--input', str(jobs), '--out-dir', 'out', '--dry-run')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(json.loads(result.stdout)['jobs']), 2)


class PricingTests(Isolated):
    def test_public_pricing_uses_effective_rates_and_omits_auth(self):
        response = {'data': {'token_price': {'output': 999}, 'pricing': {'unit': 'usd_per_million_tokens', 'effective_rates': {'image_output': 24, 'cached_text_input': 0}, 'rates': {'image_output': 30}}}}
        with patch.object(client, 'request', return_value=response) as request:
            info = models.pricing('gpt-image-2.5-flare')
        self.assertEqual(info['pricing']['effective_rates']['image_output'], 24)
        self.assertEqual(info['pricing']['effective_rates']['cached_text_input'], 0)
        self.assertNotIn('text_input', info['pricing']['effective_rates'])
        self.assertNotIn('key', request.call_args.kwargs)

    def test_legacy_price_is_not_reported_as_payable(self):
        with patch.object(client, 'request', return_value={'data': {'model_price': 0.01, 'quota_type': 1}}):
            info = models.pricing('gpt-image-2')
        self.assertIn('unverified', info['status'])
        self.assertNotIn('pricing', info)

    def test_catalog_separates_discovery_from_runnable(self):
        with patch.object(client, 'request', return_value={'data': [{'id': 'gpt-image-2.5-flare', 'category': 'image'}, {'id': 'future-image', 'category': 'image'}, {'id': 'chat', 'category': 'chat'}]}):
            rows = models.catalog(True, 'fake')
        self.assertEqual([r['runnable'] for r in rows], [True, False])


class HTTPFlowTests(Isolated):
    def setUp(self):
        super().setUp()
        self.calls = []
        calls = self.calls

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass

            def reply(self, data, code=200, content_type='application/json'):
                raw = data if isinstance(data, bytes) else json.dumps(data).encode()
                self.send_response(code)
                self.send_header('Content-Type', content_type)
                self.end_headers()
                self.wfile.write(raw)

            def do_POST(self):
                raw = self.rfile.read(int(self.headers.get('Content-Length', 0)))
                calls.append(('POST', self.path, dict(self.headers), raw))
                if self.path == '/v1/uploads/images':
                    self.reply({'url': f'http://127.0.0.1:{self.server.server_port}/image.png'})
                elif self.path == '/v1/images/generations':
                    self.reply({'code': 200, 'data': [{'task_id': 'test-task'}]})
                else:
                    self.reply({'error': 'bad'}, 400)

            def do_GET(self):
                calls.append(('GET', self.path, dict(self.headers), b''))
                if self.path == '/image.png':
                    self.reply(PNG, content_type='image/png')
                elif self.path == '/v1/tasks/test-task':
                    self.reply({'data': {'status': 'completed', 'cost': 0.012, 'result': {'images': [{'url': [f'http://127.0.0.1:{self.server.server_port}/image.png']}]}}})
                elif self.path == '/redirect':
                    self.send_response(302)
                    self.send_header('Location', '/leaked')
                    self.end_headers()
                else:
                    self.reply({'error': 'secret reflected'}, 500)

        self.server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.base = patch.object(client, 'API_BASE', f'http://127.0.0.1:{self.server.server_port}')
        self.base.start()

    def tearDown(self):
        self.base.stop()
        self.server.shutdown()
        self.server.server_close()
        self.thread.join()
        super().tearDown()

    def test_upload_submit_poll_download_and_receipt(self):
        ref = self.root / '产品 图.png'
        ref.write_bytes(PNG)
        out = self.root / '输出' / 'image.webp'
        args = cli.parser().parse_args(['edit', '--image', str(ref), '--prompt', 'change background', '--out', str(out)])
        with patch.dict(os.environ, {'APIMART_API_KEY': 'fake-test-key'}), redirect_stderr(io.StringIO()):
            result = cli.execute(cli.build_payload(args), [out], args)
        final = out.with_suffix('.png')
        self.assertEqual(final.read_bytes(), PNG)
        self.assertEqual(result['task_id'], 'test-task')
        self.assertEqual(len([c for c in self.calls if c[0] == 'POST']), 2)
        generation = next(c for c in self.calls if c[1] == '/v1/images/generations')
        self.assertTrue(json.loads(generation[3])['image_urls'][0].startswith('http://'))
        upload = next(c for c in self.calls if c[1] == '/v1/uploads/images')
        self.assertIn(b'name="file"', upload[3])
        self.assertIn(PNG, upload[3])
        download = next(c for c in self.calls if c[1] == '/image.png')
        self.assertNotIn('Authorization', download[2])
        receipt = next((settings.config_dir() / 'tasks').glob('*.json')).read_text()
        self.assertNotIn('fake-test-key', receipt)
        self.assertNotIn('change background', receipt)

    def test_fetch_does_not_submit(self):
        args = cli.parser().parse_args(['fetch', 'test-task', '--out', str(self.root / 'resumed.png')])
        with patch.dict(os.environ, {'APIMART_API_KEY': 'fake'}), redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            cli.run_fetch(args)
        self.assertTrue((self.root / 'resumed.png').exists())
        self.assertFalse(any(c[0] == 'POST' for c in self.calls))

    def test_http_error_no_retry_and_no_reflected_secret(self):
        with self.assertRaises(client.APIError) as ctx:
            client.request('/failure', key='fake', payload={'prompt': 'test'})
        self.assertNotIn('secret reflected', str(ctx.exception))
        self.assertEqual(len(self.calls), 1)

    def test_authenticated_redirect_not_followed(self):
        with self.assertRaises(client.APIError):
            client.request('/redirect', key='fake')
        self.assertEqual(len(self.calls), 1)

    def test_timeout_retains_id(self):
        with patch.object(client, 'request', return_value={'data': {'status': 'processing'}}):
            with self.assertRaisesRegex(client.APIError, 'fetch pending-task'):
                client.poll('pending-task', 'fake', timeout=0.01, interval=0.005)

    def test_non_image_download_is_rejected(self):
        with self.assertRaises(ValueError):
            client.image_format(b'<html>error</html>')


class ChromaTests(Isolated):
    def test_chroma_key_removes_background(self):
        from PIL import Image
        source = self.root / 'keyed.png'
        output = self.root / 'transparent.png'
        im = Image.new('RGB', (32, 32), (0, 255, 0))
        for x in range(8, 24):
            for y in range(8, 24):
                im.putpixel((x, y), (255, 0, 0))
        im.save(source)
        env = {**os.environ, 'PYTHONIOENCODING': 'cp1252'}
        run = subprocess.run([sys.executable, str(SCRIPTS / 'remove_chroma_key.py'), '--input', str(source), '--out', str(output)], capture_output=True, timeout=30, env=env)
        self.assertEqual(run.returncode, 0, run.stderr)
        with Image.open(output) as result:
            self.assertEqual(result.mode, 'RGBA')
            self.assertEqual(result.getpixel((0, 0))[3], 0)
            self.assertEqual(result.getpixel((16, 16))[3], 255)


if __name__ == '__main__':
    unittest.main()
