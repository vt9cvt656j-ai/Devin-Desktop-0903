import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest import mock


MODULE_PATH = Path(__file__).with_name("michael-remote-agent.py")
SPEC = importlib.util.spec_from_file_location("michael_remote_agent", MODULE_PATH)
REMOTE_AGENT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(REMOTE_AGENT)


class SearchTests(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.TemporaryDirectory()
        self.root = Path(self.temp_dir.name)
        self.previous_root = REMOTE_AGENT.CFG["root"]
        self.previous_max_read_bytes = REMOTE_AGENT.CFG["max_read_bytes"]
        REMOTE_AGENT.CFG["root"] = str(self.root)

    def tearDown(self):
        REMOTE_AGENT.CFG["root"] = self.previous_root
        REMOTE_AGENT.CFG["max_read_bytes"] = self.previous_max_read_bytes
        self.temp_dir.cleanup()

    def test_literal_default_and_regex_mode_support_single_file_scope(self):
        source = self.root / "source.txt"
        source.write_text("alpha[1]\nALPHA 42\nalpha111\n", encoding="utf-8")

        literal = REMOTE_AGENT.h_fs_search({
            "root": str(source),
            "query": "alpha[1]",
            "case_sensitive": False,
        })
        self.assertNotIn("error", literal)
        self.assertEqual(literal["scanned_files"], 1)
        self.assertEqual(len(literal["hits"]), 1)
        self.assertEqual(literal["hits"][0]["rel"], "source.txt")
        self.assertEqual(literal["hits"][0]["start"], 0)

        regex = REMOTE_AGENT.h_fs_search({
            "root": str(source),
            "query": r"alpha(?:\[\d+\]|\s+\d+)",
            "mode": "regex",
            "case_sensitive": False,
        })
        self.assertNotIn("error", regex)
        self.assertEqual(regex["scanned_files"], 1)
        self.assertEqual([hit["line"] for hit in regex["hits"]], [1, 2])

    def test_invalid_regex_mode_and_scope_are_explicit_errors(self):
        source = self.root / "source.txt"
        source.write_text("content\n", encoding="utf-8")

        bad_regex = REMOTE_AGENT.h_fs_search({
            "root": str(source), "query": "[", "mode": "regex"
        })
        self.assertIn("[INVALID_SEARCH_PATTERN]", bad_regex["error"])

        bad_mode = REMOTE_AGENT.h_fs_search({
            "root": str(source), "query": "content", "mode": "glob"
        })
        self.assertIn("[INVALID_SEARCH_MODE]", bad_mode["error"])

        bad_limit = REMOTE_AGENT.h_fs_search({
            "root": str(source), "query": "content", "max": 0
        })
        self.assertIn("[INVALID_SEARCH_LIMIT]", bad_limit["error"])

        bad_scope = REMOTE_AGENT.h_fs_search({
            "root": str(self.root / "missing"), "query": "content"
        })
        self.assertIn("[INVALID_SEARCH_SCOPE]", bad_scope["error"])
        self.assertEqual(bad_scope["scanned_files"], 0)

    def test_no_matches_and_no_scanned_files_are_distinct(self):
        (self.root / "one.txt").write_text("first\n", encoding="utf-8")
        (self.root / "two.txt").write_text("second\n", encoding="utf-8")

        no_match = REMOTE_AGENT.h_fs_search({
            "root": str(self.root), "query": "absent"
        })
        self.assertNotIn("error", no_match)
        self.assertEqual(no_match["hits"], [])
        self.assertEqual(no_match["scanned_files"], 2)

        empty = self.root / "empty"
        empty.mkdir()
        no_files = REMOTE_AGENT.h_fs_search({
            "root": str(empty), "query": "anything"
        })
        self.assertIn("[NO_SEARCHABLE_FILES]", no_files["error"])
        self.assertEqual(no_files["scanned_files"], 0)

    def test_write_rejects_missing_or_non_string_content_without_truncating(self):
        target = self.root / "keep.txt"
        target.write_text("keep me", encoding="utf-8")

        missing = REMOTE_AGENT.h_fs_write({"path": str(target)})
        self.assertIn("[INVALID_WRITE_CONTENT]", missing["error"])
        self.assertEqual(target.read_text(encoding="utf-8"), "keep me")

        for invalid in (None, 0, False, [], {}):
            with self.subTest(content=invalid):
                result = REMOTE_AGENT.h_fs_write({
                    "path": str(target), "content": invalid
                })
                self.assertIn("[INVALID_WRITE_CONTENT]", result["error"])
                self.assertEqual(target.read_text(encoding="utf-8"), "keep me")

        empty_target = self.root / "empty.txt"
        created = REMOTE_AGENT.h_fs_write({
            "path": str(empty_target), "expected_content": None, "content": ""
        })
        self.assertEqual(created["ok"], True)
        self.assertEqual(empty_target.read_text(encoding="utf-8"), "")

    def test_atomic_write_has_a_windows_compatible_permission_fallback(self):
        target = self.root / "windows.txt"
        with mock.patch.object(REMOTE_AGENT.os, "fchmod", None):
            result = REMOTE_AGENT.h_fs_write({
                "path": str(target), "content": "complete content\n"
            })

        self.assertEqual(result["ok"], True)
        self.assertEqual(target.read_text(encoding="utf-8"), "complete content\n")

    def test_atomic_write_ignores_unsupported_unix_metadata_operations(self):
        target = self.root / "windows-metadata.txt"
        target.write_text("old", encoding="utf-8")

        with mock.patch.object(
            REMOTE_AGENT.os, "fchown", side_effect=NotImplementedError, create=True
        ), mock.patch.object(
            REMOTE_AGENT.os, "fchmod", side_effect=NotImplementedError, create=True
        ), mock.patch.object(
            REMOTE_AGENT.os, "chmod", side_effect=NotImplementedError
        ):
            result = REMOTE_AGENT.h_fs_write({
                "path": str(target), "content": "new"
            })

        self.assertEqual(result["ok"], True)
        self.assertEqual(target.read_text(encoding="utf-8"), "new")

    def test_large_files_can_be_read_in_bounded_ranges_and_bad_ranges_fail(self):
        target = self.root / "large.txt"
        target.write_text("one\ntwo\nthree\nfour\nfive\n", encoding="utf-8")
        REMOTE_AGENT.CFG["max_read_bytes"] = 12

        whole = REMOTE_AGENT.h_fs_read({"path": str(target)})
        self.assertIn("文件过大", whole["error"])

        segment = REMOTE_AGENT.h_fs_read({
            "path": str(target), "offset": 2, "limit": 2
        })
        self.assertEqual(segment["content"], "two\nthree")
        self.assertEqual(segment["from"], 2)
        self.assertEqual(segment["to"], 3)
        self.assertEqual(segment["total_lines"], 6)
        self.assertEqual(segment["truncated"], True)

        negative = REMOTE_AGENT.h_fs_read({
            "path": str(target), "offset": 1, "limit": -1
        })
        self.assertIn("[INVALID_READ_RANGE]", negative["error"])

        negative_offset = REMOTE_AGENT.h_fs_read({
            "path": str(target), "offset": -1, "limit": 1
        })
        self.assertIn("[INVALID_READ_RANGE]", negative_offset["error"])


if __name__ == "__main__":
    unittest.main()
