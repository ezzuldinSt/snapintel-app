"""Run SnapIntel in-process against the page snapshots in tests/fixtures (shared with the desktop app), with no network access."""
import io
import json
import os
import re
import sys
from unittest import mock

import snapintel.cli

FIXTURES = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "tests", "fixtures"))
PAGES = {"spectacles": "public_profile.html", "bitmoji": "nonpublic_profile.html"}
ANSI = re.compile(r"\x1b\[[0-9;]*m")


def fixture_text(name):
	with open(os.path.join(FIXTURES, name), encoding="utf-8") as file:
		return file.read()


def page_json(name):
	return json.loads(re.search(r"<script[^>]*>(.*)</script>", fixture_text(name), re.DOTALL).group(1))


def page_from_json(data):
	return f'<html><script id="__NEXT_DATA__" type="application/json">{json.dumps(data, ensure_ascii=False)}</script></html>'


def plain(text):
	return ANSI.sub("", text)


class FakeResponse:
	def __init__(self, text="", content=b"", status=200):
		self.text, self.content, self.status_code = text, content, status

	def raise_for_status(self):
		if self.status_code >= 400:
			raise RuntimeError(f"HTTP {self.status_code}")

	def iter_content(self, size):
		for start in range(0, len(self.content), size):
			yield self.content[start:start + size]


class FakeWeb:
	"""Profile pages come from the snapshots (or `pages`); any other URL is media, answered from `media` or
	with bytes derived from the URL. Media requests are recorded in `media_requests`."""

	def __init__(self, pages=None, media=None):
		self.pages = {user: fixture_text(name) for user, name in PAGES.items()}
		self.pages.update(pages or {})
		self.media = media or {}
		self.media_requests = []

	def get(self, url, headers=None, timeout=None, **kwargs):
		prefix = "https://www.snapchat.com/add/"
		if url.startswith(prefix):
			user = url[len(prefix):]
			if user in self.pages:
				return FakeResponse(text=self.pages[user])
			return FakeResponse(text=fixture_text("not_found.html"), status=404)
		self.media_requests.append(url)
		return FakeResponse(content=self.media.get(url, ("media for " + url).encode()))


def run(argv, web=None, rtl="0"):
	"""Run `snapintel <argv>`; returns (exit code, stdout, stderr) with stdout/stderr decoded as UTF-8."""
	web = web or FakeWeb()
	out_bytes, err_bytes = io.BytesIO(), io.BytesIO()
	out = io.TextIOWrapper(out_bytes, encoding="utf-8", write_through=True)
	err = io.TextIOWrapper(err_bytes, encoding="utf-8", write_through=True)
	code = 0
	with mock.patch.object(sys, "argv", ["snapintel", *argv]), \
			mock.patch("requests.get", web.get), \
			mock.patch.dict(os.environ, {"SNAPINTEL_RTL": rtl}), \
			mock.patch.object(sys, "stdout", out), \
			mock.patch.object(sys, "stderr", err):
		try:
			snapintel.cli.main()
		except SystemExit as exit_:
			code = exit_.code if isinstance(exit_.code, int) else (0 if exit_.code is None else 1)
		finally:
			flush = getattr(sys.stdout, "flush", None)
			if flush:
				flush()
	return code, out_bytes.getvalue().decode("utf-8"), err_bytes.getvalue().decode("utf-8")
