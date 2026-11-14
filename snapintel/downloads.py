"""Downloads you can verify later and resume.

Every file SnapIntel saves is recorded in snapintel-manifest.json in the output folder: the source URL, the
time it was downloaded, its SHA-256 and size, what it is (story, highlight, spotlight, lens, bitmoji) and,
when Snapchat gives one, its upload date, which also becomes the file's modification time.

Files are named after their Snapchat media ID (story_<id>.mp4 and so on), so a second run finds them:
a file whose hash still matches the manifest is skipped without downloading it again. If the file on disk
no longer matches what was recorded, it is left alone and reported, and the fresh copy is saved beside it
(<name>-<first 8 hex of its hash>.<ext>), so nothing already saved is ever overwritten.

`snapintel --verify DIR` re-hashes every file listed in a folder's manifest and reports any that are
missing or altered.
"""
import concurrent.futures
import datetime
import hashlib
import json
import os
import re
import threading
import urllib.parse

import requests

from . import __version__

MANIFEST = "snapintel-manifest.json"


def utc_now():
	return datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def media_id(url):
	"""A stable, file-name-safe ID for a media URL: its last path segment without extension, or a hash of the path."""
	path = urllib.parse.urlparse(url).path
	segment = [s for s in path.split("/") if s][-1:] or [""]
	candidate = re.sub(r"[^A-Za-z0-9_-]", "", segment[0].split(".")[0])
	return candidate if len(candidate) >= 8 else hashlib.sha1(path.encode()).hexdigest()[:16]


def sha256_file(path):
	digest = hashlib.sha256()
	with open(path, "rb") as file:
		for chunk in iter(lambda: file.read(1 << 20), b""):
			digest.update(chunk)
	return digest.hexdigest()


def _timestamp(iso):
	try:
		return datetime.datetime.strptime(iso, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=datetime.timezone.utc).timestamp()
	except (TypeError, ValueError):
		return None


def iso_from_unix(seconds):
	try:
		return datetime.datetime.fromtimestamp(float(seconds), datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
	except (TypeError, ValueError, OverflowError, OSError):
		return None


def load_manifest(directory):
	path = os.path.join(directory, MANIFEST)
	try:
		with open(path, encoding="utf-8") as file:
			data = json.load(file)
		return {entry["file"]: entry for entry in data.get("files", []) if "file" in entry}
	except FileNotFoundError:
		return {}


class DownloadManager:
	def __init__(self, directory, username, headers, timeout, threads):
		self.directory = os.path.normpath(directory or ".")
		self.username = username
		self.headers = headers
		self.timeout = timeout
		self.threads = threads
		self.lock = threading.Lock()
		self.queue = {}        # file name -> what to fetch
		self.queued_urls = set()
		self.entries = load_manifest(self.directory)
		self.counts = {"downloaded": 0, "skipped": 0, "already_present": 0, "kept_altered": 0, "failed": 0}
		self.altered = []

	# Queue a media URL; nothing is fetched until run().
	def add(self, url, ext, category, uploaded_at=None):
		if not url:
			return
		name = f"{category}_{media_id(url)}.{ext}"
		with self.lock:
			if name in self.queue or url in self.queued_urls:
				return
			self.queue[name] = {"url": url, "category": category, "uploaded_at": uploaded_at}
			self.queued_urls.add(url)

	def run(self):
		os.makedirs(self.directory, exist_ok=True)
		with concurrent.futures.ThreadPoolExecutor(max_workers=self.threads) as executor:
			futures = {executor.submit(self._fetch, name, item): name for name, item in self.queue.items()}
			for future in concurrent.futures.as_completed(futures):
				try:
					future.result()
				except Exception as error:
					with self.lock:
						self.counts["failed"] += 1
					print(f"Error downloading {self.queue[futures[future]]['url']}: {error}")
		self.save()
		return self.counts

	def _fetch(self, name, item):
		path = os.path.join(self.directory, name)
		recorded = self.entries.get(name)
		if os.path.exists(path) and recorded and sha256_file(path) == recorded.get("sha256"):
			with self.lock:
				self.counts["skipped"] += 1
			return
		response = requests.get(item["url"], headers=self.headers, timeout=self.timeout)
		response.raise_for_status()
		self.store(name, response.content, item["url"], item["category"], item["uploaded_at"])

	# Save content under name (or beside it, if a different file is already there) and record it.
	def store(self, name, content, url, category, uploaded_at=None):
		os.makedirs(self.directory, exist_ok=True)
		digest = hashlib.sha256(content).hexdigest()
		path = os.path.join(self.directory, name)
		with self.lock:
			if os.path.exists(path):
				existing = sha256_file(path)
				if existing == digest:
					self.counts["already_present"] += 1
					self._record(name, digest, len(content), url, category, uploaded_at, path)
					return name
				# The file on disk differs from this download: keep it untouched and save the new copy beside it.
				stem, ext = os.path.splitext(name)
				recorded = self.entries.get(name)
				if recorded and recorded.get("sha256") != existing:
					self.altered.append(name)
				name = f"{stem}-{digest[:8]}{ext}"
				path = os.path.join(self.directory, name)
				self.counts["kept_altered"] += 1
				if os.path.exists(path) and sha256_file(path) == digest:
					self._record(name, digest, len(content), url, category, uploaded_at, path)
					return name
			partial = path + ".part"
			with open(partial, "wb") as file:
				file.write(content)
			os.replace(partial, path)
			self.counts["downloaded"] += 1
			self._record(name, digest, len(content), url, category, uploaded_at, path)
			return name

	def _record(self, name, digest, size, url, category, uploaded_at, path):
		when = _timestamp(uploaded_at)
		if when is not None:
			os.utime(path, (when, when))
		self.entries[name] = {
			"file": name,
			"sha256": digest,
			"bytes": size,
			"category": category,
			"username": self.username,
			"media_id": os.path.splitext(name)[0].split("_", 1)[-1],
			"source_url": url,
			"uploaded_at": uploaded_at,
			"downloaded_at": utc_now(),
		}

	def save(self):
		if not self.entries:
			return
		os.makedirs(self.directory, exist_ok=True)
		document = {
			"snapintel_version": __version__,
			"updated_at": utc_now(),
			"files": sorted(self.entries.values(), key=lambda e: e["file"]),
		}
		path = os.path.join(self.directory, MANIFEST)
		with open(path + ".part", "w", encoding="utf-8", newline="\n") as file:
			json.dump(document, file, ensure_ascii=False, indent=2)
			file.write("\n")
		os.replace(path + ".part", path)

	def summary(self):
		return {"directory": self.directory, "manifest": os.path.join(self.directory, MANIFEST), **self.counts,
		        "altered_on_disk": list(self.altered)}


def verify(directory):
	"""Re-hash every file in a folder's manifest. Returns the exit code: 0 if all match, 1 otherwise."""
	entries = load_manifest(directory)
	if not entries:
		print(f"No {MANIFEST} in {directory}")
		return 1
	ok = missing = altered = 0
	for name, entry in sorted(entries.items()):
		path = os.path.join(directory, name)
		if not os.path.exists(path):
			missing += 1
			print(f"MISSING  {name}")
		elif sha256_file(path) != entry.get("sha256"):
			altered += 1
			print(f"ALTERED  {name}  (recorded {entry.get('sha256', '')[:12]}..., downloaded {entry.get('downloaded_at')})")
		else:
			ok += 1
	media = {f for f in os.listdir(directory) if os.path.splitext(f)[1].lower() in (".mp4", ".jpg", ".jpeg", ".png", ".webp")}
	untracked = sorted(media - set(entries))
	for name in untracked:
		print(f"NOT IN MANIFEST  {name}")
	print(f"{ok} OK, {altered} altered, {missing} missing, {len(untracked)} media files not in the manifest ({os.path.join(directory, MANIFEST)})")
	return 0 if not (altered or missing) else 1
