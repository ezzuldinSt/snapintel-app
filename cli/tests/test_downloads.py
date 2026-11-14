import copy
import hashlib
import json
import os
import shutil
import tempfile
import unittest

from helpers import FakeWeb, page_from_json, page_json, plain, run
from snapintel.downloads import MANIFEST

PUBLIC = page_json("public_profile.html")["props"]["pageProps"]


def expected_media():
	urls = {s["snapUrls"]["mediaUrl"] for h in PUBLIC["curatedHighlights"] for s in h["snapList"]}
	urls |= {s["snapUrls"]["mediaUrl"] for h in PUBLIC["spotlightHighlights"] for s in h["snapList"]}
	urls |= {lens["lensPreviewVideoUrl"] for lens in PUBLIC["lenses"]}
	return urls


def manifest(folder):
	with open(os.path.join(folder, MANIFEST), encoding="utf-8") as file:
		return {entry["file"]: entry for entry in json.load(file)["files"]}


def sha256(path):
	with open(path, "rb") as file:
		return hashlib.sha256(file.read()).hexdigest()


class Downloads(unittest.TestCase):
	def setUp(self):
		self.folder = tempfile.mkdtemp()

	def tearDown(self):
		shutil.rmtree(self.folder, ignore_errors=True)

	def download(self, web=None):
		web = web or FakeWeb()
		code, out, err = run(["-u", "spectacles", "-d", "a", "-o", self.folder], web=web)
		self.assertEqual(code, 0, out + err)
		return web, plain(out)

	def test_every_file_is_recorded_with_its_hash(self):
		self.download()
		entries = manifest(self.folder)
		self.assertEqual({e["source_url"] for e in entries.values()}, expected_media())
		for name, entry in entries.items():
			self.assertEqual(sha256(os.path.join(self.folder, name)), entry["sha256"])
			self.assertEqual(entry["username"], "spectacles")
			self.assertIn(entry["category"], {"highlight", "spotlight", "lens"})
			self.assertTrue(name.startswith(entry["category"] + "_"))

	def test_file_time_is_the_upload_time(self):
		self.download()
		snap = PUBLIC["curatedHighlights"][0]["snapList"][0]
		entry = next(e for e in manifest(self.folder).values() if e["source_url"] == snap["snapUrls"]["mediaUrl"])
		self.assertRegex(entry["uploaded_at"], r"Z$")
		mtime = os.path.getmtime(os.path.join(self.folder, entry["file"]))
		self.assertAlmostEqual(mtime, int(snap["timestampInSec"]["value"]), delta=1)

	def test_second_run_downloads_nothing(self):
		self.download()
		web, out = self.download()
		self.assertEqual(web.media_requests, [])
		self.assertIn(f"Downloaded 0, skipped {len(expected_media())}", out)

	def test_verify(self):
		self.download()
		code, out, _ = run(["--verify", self.folder])
		self.assertEqual(code, 0, out)
		self.assertIn(f"{len(expected_media())} OK, 0 altered, 0 missing", out)

	def test_altered_file_is_reported_and_kept(self):
		self.download()
		name = sorted(manifest(self.folder))[0]
		path = os.path.join(self.folder, name)
		with open(path, "ab") as file:
			file.write(b" edited")
		edited = sha256(path)

		code, out, _ = run(["--verify", self.folder])
		self.assertEqual(code, 1)
		self.assertIn(f"ALTERED  {name}", out)

		_, out = self.download()
		self.assertEqual(sha256(path), edited)                 # the edited file is left as it is
		self.assertIn("Changed on disk", out)
		fresh = [f for f in os.listdir(self.folder) if f.startswith(os.path.splitext(name)[0] + "-")]
		self.assertEqual(len(fresh), 1)                         # the downloaded copy is saved beside it

	def test_removed_file_is_downloaded_again(self):
		self.download()
		name = sorted(manifest(self.folder))[0]
		os.remove(os.path.join(self.folder, name))
		code, out, _ = run(["--verify", self.folder])
		self.assertEqual(code, 1)
		self.assertIn(f"MISSING  {name}", out)
		web, _ = self.download()
		self.assertEqual(len(web.media_requests), 1)
		self.assertTrue(os.path.exists(os.path.join(self.folder, name)))

	def test_bitmojis_are_recorded(self):
		page = copy.deepcopy(page_json("nonpublic_profile.html"))
		avatar = "https://example.invalid/avatar/abc_3-x-y-z-tail.webp"   # versions 1..3 are derived from it
		page["props"]["pageProps"]["userProfile"]["userInfo"]["bitmoji3d"] = {
			"avatarImage": {"url": avatar}, "backgroundImage": {"url": "https://example.invalid/background.webp"}}
		versions = {f"https://example.invalid/avatar/abc_{v}-tail.webp": content
		            for v, content in ((1, b"look A"), (2, b"look A"), (3, b"look B"))}
		web = FakeWeb(pages={"bitmoji": page_from_json(page)}, media=versions)
		code, out, err = run(["-u", "bitmoji", "-d", "b", "-o", self.folder], web=web)
		self.assertEqual(code, 0, out + err)
		entries = manifest(self.folder)
		self.assertEqual(len(entries), 2)                       # two distinct looks
		self.assertTrue(all(e["category"] == "bitmoji" for e in entries.values()))


if __name__ == "__main__":
	unittest.main()
