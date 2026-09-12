"""Distribution invariants required by the Agent Skills specification."""
from pathlib import Path
import re
import unittest
import yaml

ROOT = Path(__file__).resolve().parents[1]
SKILL = ROOT / 'imagegen'


class SkillStructureTests(unittest.TestCase):
    def test_frontmatter_and_name(self):
        text = (SKILL / 'SKILL.md').read_text(encoding='utf-8')
        self.assertTrue(text.startswith('---\n'))
        data = yaml.safe_load(text.split('---', 2)[1])
        self.assertEqual(data['name'], SKILL.name)
        self.assertRegex(data['name'], r'^[a-z0-9]+(?:-[a-z0-9]+)*$')
        self.assertLessEqual(len(data['name']), 64)
        self.assertTrue(0 < len(data['description']) <= 1024)
        self.assertLess(len(text.splitlines()), 500)
        self.assertTrue(set(data) <= {'name', 'description', 'license', 'compatibility', 'metadata', 'allowed-tools'})
        self.assertTrue(all(isinstance(k, str) and isinstance(v, str) for k, v in data['metadata'].items()))

    def test_local_markdown_links_resolve(self):
        for doc in SKILL.rglob('*.md'):
            for target in re.findall(r'\]\(([^)]+)\)', doc.read_text(encoding='utf-8')):
                if '://' in target or target.startswith('#'):
                    continue
                target = target.split('#')[0]
                with self.subTest(doc=doc.name, target=target):
                    self.assertTrue((doc.parent / target).is_file())

    def test_metadata_and_packaged_licenses(self):
        data = yaml.safe_load((SKILL / 'agents' / 'openai.yaml').read_text(encoding='utf-8'))
        ui = data['interface']
        self.assertTrue(25 <= len(ui['short_description']) <= 64)
        self.assertIn('$imagegen', ui['default_prompt'])
        self.assertEqual((ROOT / 'LICENSE').read_bytes(), (SKILL / 'LICENSE.txt').read_bytes())
        self.assertEqual((ROOT / 'NOTICE').read_bytes(), (SKILL / 'NOTICE').read_bytes())


if __name__ == '__main__':
    unittest.main()
