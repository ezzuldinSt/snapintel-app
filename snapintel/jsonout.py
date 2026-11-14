"""--json: the same information as the text output, as one JSON document.

JsonDisplay has the same methods as display.Display and is called at the same points, so a section is in
the JSON exactly when the text output would show it (-s, -l, -d choose as before); the profile is always
included. Values are typed: counts and durations are numbers, dates are ISO 8601 in UTC, and fields
Snapchat leaves empty are null. Text is written as UTF-8 with Arabic and other scripts left as they are.
"""
import datetime
import json
import re

from . import __version__

BADGES = {1: "creator", 3: "public_figure"}
MEDIA_TYPES = {0: "image", 1: "video"}


def _iso(date_str):
	"""'2026-09-27 14:05:00' (UTC, as SnapIntel stores it) -> '2026-09-27T14:05:00Z'."""
	try:
		return datetime.datetime.strptime(date_str, "%Y-%m-%d %H:%M:%S").strftime("%Y-%m-%dT%H:%M:%SZ")
	except (TypeError, ValueError):
		return None


def _int(value):
	try:
		return int(value)
	except (TypeError, ValueError):
		return None


def _engagement(value):
	number = _int(value)
	return number if number is not None and number >= 0 else None   # Snapchat sends -1 when hidden


def _text(value):
	return value if isinstance(value, str) and value != "" else None


def _seconds(duration):
	"""'1m19s' -> 79."""
	match = re.fullmatch(r"(\d+)m(\d+)s", duration or "")
	return int(match.group(1)) * 60 + int(match.group(2)) if match else None


def _spotlight_parts(item):
	"""SnapIntel's spotlight entries leave out the engagement figure in some cases, so they have 7 or 8 parts."""
	if len(item) >= 8:
		thumbnail, name, duration, uploaded, engagement, hashtags, top, snaps = item[:8]
	else:
		thumbnail, name, duration, uploaded, hashtags, top, snaps = item[:7]
		engagement = None
	return thumbnail, name, duration, uploaded, engagement, hashtags, top, snaps


class JsonDisplay:
	def __init__(self, parser, ssd):
		self.parser = parser
		self.ssd = ssd
		self.doc = {
			"snapintel_version": __version__,
			"retrieved_at": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
			"requested_username": parser.username,
			"url": "https://www.snapchat.com/add/" + parser.username,
		}

	def print_basic_information(self, data):
		profile = {"page_title": _text(data[0]), "page_description": _text(data[1]), "public": not data[2]}
		if not data[2]:
			profile.update({
				"username": _text(data[3]),
				"badge": BADGES.get(data[4]),
				"badge_code": data[4] if isinstance(data[4], int) else None,
				"profile_picture_url": _text(data[5]),
				"background_picture_url": _text(data[6]),
				"subscriber_count": _int(data[7]),
				"bio": _text(data[8]),
				"website_url": _text(data[9]),
				"snapcode_url": _text(data[10]),
			})
		else:
			profile.update({
				"username": _text(data[3]),
				"display_name": _text(data[4]),
				"avatar_url": _text(data[5]),
				"background_url": _text(data[7]),
				"snapcode_url": _text(data[8]),
			})
		self.doc["profile"] = profile

	def print_stories(self, data):
		if self.parser.list_stories or self.parser.list_all:
			self.doc["stories"] = {
				"count": data[0],
				"items": [{"id": s[0], "url": s[1], "uploaded_at": _iso(s[2]), "type": MEDIA_TYPES.get(s[3])} for s in data[1:]],
			}

	def print_curated_highlights(self, data):
		if (self.parser.list_highlights or self.parser.list_all):
			data = data or [0]
			items = [{
				"title": _text(story[0]),
				"snaps": [{"id": s[0], "url": s[1], "uploaded_at": _iso(s[2])} for s in story[1:]],
			} for story in data[1:]]
			self.doc["curated_highlights"] = {"count": data[0], "total_snaps": sum(len(i["snaps"]) for i in items), "items": items}

	def print_spotlights(self, data):
		if self.parser.list_spotlights or self.parser.list_all:
			if not data:
				self.doc["spotlights"] = {"count": 0, "total_snaps": 0, "total_duration_seconds": 0, "total_engagement": None, "top_hashtags": [], "items": []}
				return
			items, top = [], []
			for item in data[1:-1]:
				thumbnail, name, duration, uploaded, engagement, hashtags, top, snaps = _spotlight_parts(item)
				items.append({
					"name": _text(name),
					"thumbnail_url": _text(thumbnail),
					"duration_seconds": _seconds(duration),
					"uploaded_at": _iso(uploaded),
					"engagement": _engagement(engagement),
					"hashtags": list(hashtags or []),
					"snaps": [{"id": s[0], "url": s[1]} for s in snaps],
				})
			self.doc["spotlights"] = {
				"count": data[0],
				"total_snaps": sum(len(i["snaps"]) for i in items),
				"total_duration_seconds": sum(i["duration_seconds"] or 0 for i in items),
				"total_engagement": _engagement(data[-1]),
				"top_hashtags": [{"hashtag": h, "count": c} for h, c in (top or [])],
				"items": items,
			}

	def print_lenses(self, data):
		if self.parser.list_lenses or self.parser.list_all:
			self.doc["lenses"] = {
				"count": data[0],
				"items": [{"name": _text(l[0]), "official": l[1], "preview_video_url": _text(l[2])} for l in data[1:]],
			}

	def print_stats(self, data):
		if self.parser.stats:
			stories, highlights, spotlights, lenses = data
			highlights = highlights or [0]
			spotlight_items = [_spotlight_parts(item) for item in spotlights[1:-1]] if spotlights else []
			self.doc["stats"] = {
				"stories": stories[0],
				"curated_highlights": highlights[0],
				"curated_highlight_snaps": sum(len(story[1:]) for story in highlights[1:]),
				"spotlights": spotlights[0] if spotlights else 0,
				"spotlight_snaps": sum(len(parts[7]) for parts in spotlight_items),
				"spotlight_duration_seconds": sum(_seconds(parts[2]) or 0 for parts in spotlight_items),
				"spotlight_engagement": _engagement(spotlights[-1]) if spotlights else None,
				"lenses": lenses[0],
			}

	def print_bitmojis(self, data, download):
		if self.parser.list_bitmojis or self.parser.list_all:
			self.doc["bitmojis"] = {"count": len(data or []), "md5": sorted(data or [])}

	def write(self, stream, path=None):
		if self.parser.download_bool:
			self.doc["downloads_directory"] = self.parser.output_dir
		text = json.dumps(self.doc, ensure_ascii=False, indent=2) + "\n"
		if path:
			with open(path, "w", encoding="utf-8", newline="\n") as file:
				file.write(text)
			return
		# Bytes, so the text reaches files and pipes as UTF-8 whatever the console code page is.
		buffer = getattr(stream, "buffer", None)
		if buffer is not None:
			buffer.write(text.encode("utf-8"))
			buffer.flush()
		else:
			stream.write(text)
			stream.flush()
