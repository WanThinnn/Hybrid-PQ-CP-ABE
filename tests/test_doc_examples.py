import importlib.util
import io
from pathlib import Path
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location('doc_examples', Path(__file__).resolve().parents[1] / 'scripts/check-doc-examples.py')
examples = importlib.util.module_from_spec(spec)
spec.loader.exec_module(examples)


class DocumentationExamples(unittest.TestCase):
    def test_real_docs_share_c_source_and_compiler_commands(self):
        code, _ = examples.extract_examples()
        self.assertIn('int main(void)', code)

    def test_divergent_example_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in ('native', 'bindings'):
                (root / 'docs' / name).mkdir(parents=True)
            (root / 'docs/native/api.md').write_text('```c\nint main(void) {}\n```\n-std=c17\n')
            (root / 'docs/bindings/download.md').write_text('```c\nint other(void) {}\n```\n-std=c17\n')
            with self.assertRaisesRegex(ValueError, 'C examples differ'):
                examples.extract_examples(root)

    def test_sdk_archive_cannot_escape_destination(self):
        data = io.BytesIO()
        with zipfile.ZipFile(data, 'w') as archive:
            archive.writestr('../escape', 'bad')
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(ValueError, 'Unsafe archive'):
                examples.extract_archive(data.getvalue(), Path(directory))

    def test_extracts_packaged_headers_and_library(self):
        data = io.BytesIO()
        with zipfile.ZipFile(data, 'w') as archive:
            archive.writestr('sdk/include/api.h', 'header')
            archive.writestr('sdk/lib/library.so', 'library')
        with tempfile.TemporaryDirectory() as directory:
            sdk = examples.extract_archive(data.getvalue(), Path(directory))
            self.assertEqual((sdk / 'include/api.h').read_text(), 'header')

    def test_sdk_archive_symlink_is_rejected(self):
        data = io.BytesIO()
        with zipfile.ZipFile(data, 'w') as archive:
            info = zipfile.ZipInfo('sdk/link')
            info.create_system = 3
            info.external_attr = 0o120777 << 16
            archive.writestr(info, '../outside')
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(ValueError, 'symlinks'):
                examples.extract_archive(data.getvalue(), Path(directory))


if __name__ == '__main__':
    unittest.main()
