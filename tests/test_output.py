import io
import json
import os
import tempfile
import unittest

import arabic_reshaper
from bidi.algorithm import get_display

from helpers import FakeWeb, page_json, plain, run
from snapintel import __version__, display

PUBLIC = page_json("public_profile.html")["props"]["pageProps"]


class TextOutput(unittest.TestCase):
	def test_stats(self):
		code, out, _ = run(["-u", "spectacles", "-s"])
		text = plain(out)
		self.assertEqual(code, 0)
		self.assertIn("[+] Username: " + PUBLIC["userProfile"]["publicProfileInfo"]["username"], text)
		self.assertIn(f"[+] Number of curated highlight stories: {len(PUBLIC['curatedHighlights'])}", text)
		self.assertIn(f"[+] Number of spotlights: {len(PUBLIC['spotlightHighlights'])}", text)
		self.assertIn(f"[+] Number of lenses: {len(PUBLIC['lenses'])}", text)

	def test_report_printed_once(self):
		_, out, _ = run(["-u", "spectacles", "-s"])
		self.assertEqual(plain(out).count("[+] Page title:"), 1)

	def test_unknown_user(self):
		code, out, _ = run(["-u", "no-such-user-x", "-s"])
		self.assertEqual(code, 1)
		self.assertIn("User doesn't exist", out)

	def test_nonpublic_profile(self):
		code, out, _ = run(["-u", "bitmoji", "-l", "a"])
		self.assertEqual(code, 0)
		self.assertIn("[-] This is a private profile", plain(out))


class JsonOutput(unittest.TestCase):
	def test_list_all(self):
		code, out, _ = run(["-u", "spectacles", "-l", "a", "--json"], rtl="1")
		self.assertEqual(code, 0)
		doc = json.loads(out)          # standard output is exactly one JSON document
		info = PUBLIC["userProfile"]["publicProfileInfo"]
		self.assertEqual(doc["snapintel_version"], __version__)
		self.assertEqual(doc["profile"]["username"], info["username"])
		self.assertIsInstance(doc["profile"]["subscriber_count"], int)
		# The RTL console fix (forced on here) must not touch the JSON: text stays as Snapchat sent it.
		self.assertEqual(doc["profile"]["page_title"], PUBLIC["pageMetadata"]["pageTitle"])

		highlights = doc["curated_highlights"]
		self.assertEqual(highlights["count"], len(PUBLIC["curatedHighlights"]))
		self.assertEqual(highlights["total_snaps"], sum(len(h["snapList"]) for h in PUBLIC["curatedHighlights"]))
		first_snap = highlights["items"][0]["snaps"][0]
		self.assertRegex(first_snap["uploaded_at"], r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$")

		spotlights = doc["spotlights"]
		self.assertEqual(spotlights["count"], len(PUBLIC["spotlightHighlights"]))
		self.assertIsInstance(spotlights["items"][0]["duration_seconds"], int)
		self.assertEqual(doc["lenses"]["count"], len(PUBLIC["lenses"]))
		self.assertNotIn("[+]", out)

	def test_stats_to_file(self):
		with tempfile.TemporaryDirectory() as folder:
			path = os.path.join(folder, "stats.json")
			code, out, err = run(["-u", "spectacles", "-s", "--json", path])
			self.assertEqual((code, out), (0, ""))
			self.assertIn("JSON written to", err)
			with open(path, encoding="utf-8") as file:
				doc = json.load(file)
			self.assertEqual(doc["stats"]["spotlights"], len(PUBLIC["spotlightHighlights"]))
			self.assertNotIn("stories", doc)

	def test_nonpublic_profile(self):
		code, out, _ = run(["-u", "bitmoji", "-l", "a", "--json"])
		self.assertEqual(code, 0)
		doc = json.loads(out)
		self.assertFalse(doc["profile"]["public"])
		self.assertEqual(doc["profile"]["display_name"], "Bitmoji Temp")
		self.assertEqual(doc["bitmojis"], {"count": 0, "md5": []})

	def test_unknown_user(self):
		code, out, err = run(["-u", "no-such-user-x", "-s", "--json"])
		self.assertEqual((code, out), (1, ""))
		self.assertIn("doesn't exist", err)


class Cli(unittest.TestCase):
	def test_version(self):
		code, out, _ = run(["--version"])
		self.assertEqual((code, out.strip()), (0, f"SnapIntel {__version__}"))

	def test_option_needed(self):
		code, _, err = run(["-u", "spectacles"])
		self.assertEqual(code, 2)
		self.assertIn("-l/--list, -d/--download, -s/--stats", err)


class RtlConsole(unittest.TestCase):
	def render(self, text):
		out = io.StringIO()
		console = display.RtlConsole(out, arabic_reshaper.reshape, get_display)
		console.write(text)
		console.flush()
		return out.getvalue()

	def test_label_and_colour_kept(self):
		line = self.render("\x1b[32m[+] Bio:\x1b[0m مرحبا\n")
		self.assertTrue(line.startswith("\x1b[32m[+] Bio:\x1b[0m "))
		self.assertTrue(any(0xFE70 <= ord(c) <= 0xFEFC for c in line))   # letters joined

	def test_brackets_mirrored(self):
		self.assertIn("(sample_user@)", self.render("مرحبا (@sample_user) بالعالم\n"))

	def test_emoji_sequences_kept_whole(self):
		line = self.render("سطر ✌️ 👍🏽\n")
		self.assertIn("\u270c\ufe0f", line)
		self.assertIn("\U0001F44D\U0001F3FD", line)

	def test_lines_without_arabic_untouched(self):
		self.assertEqual(self.render("plain (text)\n"), "plain (text)\n")


if __name__ == "__main__":
	unittest.main()
