"""Refresh the page snapshots the tests run on: python tests/fixtures/update_fixtures.py

Each snapshot is a reference account's profile page reduced to what SnapIntel reads: the embedded page
JSON with only the props.pageProps sections in config.json, lists cut to their first few items, and none
of the page's HTML. When Snapchat changes its page, refresh the snapshots and run the tests to see what
broke (`snapintel --self-check` tells you the same against the live site).
"""
import json
import os
import re
import sys

import requests

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from snapintel.ssd import Snap_Story_Downloader  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
ACCOUNTS = {"public_profile.html": "spectacles", "nonpublic_profile.html": "bitmoji"}
KEEP = [
	"pageMetadata", "pageLinks", "locale", "userProfile", "story",
	"curatedHighlights", "curatedHighlightsCursor",
	"spotlightHighlights", "spotlightHighlightsCursor", "spotlightStoryMetadata",
	"lenses", "lensCursor",
]
ITEMS = 3


def trim(page_props):
	kept = {key: page_props[key] for key in KEEP if key in page_props}
	for key in ("curatedHighlights", "spotlightHighlights", "spotlightStoryMetadata", "lenses"):
		if isinstance(kept.get(key), list):
			kept[key] = kept[key][:ITEMS]
	for highlight in kept.get("curatedHighlights") or []:
		if isinstance(highlight.get("snapList"), list):
			highlight["snapList"] = highlight["snapList"][:ITEMS]
	if "encodedSpotlightComments" in page_props:
		kept["encodedSpotlightComments"] = stand_in_comments(page_props["encodedSpotlightComments"], kept.get("spotlightHighlights") or [])
	return kept


def stand_in_comments(encoded, spotlights):
	"""The page's comments, reduced to their shape: other people wrote them, so their words and
	everything naming them are replaced before anything is saved to the repository."""
	wanted = set()
	for spotlight in spotlights:
		wanted.add(((spotlight or {}).get("storyId") or {}).get("value"))
		for snap in (spotlight or {}).get("snapList") or []:
			wanted.add((snap.get("snapId") or {}).get("value"))
	try:
		groups = json.loads(encoded or "[]")
	except ValueError:
		groups = []
	stand_ins, number = [], 0
	for group in groups:
		kept = []
		for comment in group[:ITEMS]:
			if comment.get("snapId") not in wanted:
				continue
			number += 1
			kept.append({
				"snapId": comment["snapId"],
				"replyText": f"Sample comment {number}",
				"replyTimestampMs": comment.get("replyTimestampMs", "0"),
				"threadedReplyCount": comment.get("threadedReplyCount", "0"),
				"reactCounts": comment.get("reactCounts", []),
				"replyPosterDisplayName": "Removed",
				"replyPosterId": {"highBits": "0", "lowBits": "0"},
			})
		if kept:
			stand_ins.append(kept)
	return json.dumps(stand_ins)


def page(page_json):
	body = json.dumps(page_json, ensure_ascii=False, indent=1)
	return f'<!DOCTYPE html>\n<html><head><script id="__NEXT_DATA__" type="application/json">{body}</script></head></html>\n'


def main():
	snap = Snap_Story_Downloader()
	for file_name, account in ACCOUNTS.items():
		response = requests.get("https://www.snapchat.com/add/" + account, headers=snap.headers, timeout=30)
		match = re.search(snap.json_regex, response.text, re.DOTALL)
		if not match:
			sys.exit(f"{account}: no page JSON (HTTP {response.status_code})")
		data = json.loads(match.group(1))
		snapshot = {"props": {"pageProps": trim(data["props"]["pageProps"])}}
		with open(os.path.join(HERE, file_name), "w", encoding="utf-8", newline="\n") as file:
			file.write(page(snapshot))
		print(f"{file_name}: {account}, {os.path.getsize(os.path.join(HERE, file_name))} bytes")
	# A username that doesn't exist: the page has no profile metadata.
	with open(os.path.join(HERE, "not_found.html"), "w", encoding="utf-8", newline="\n") as file:
		file.write(page({"props": {"pageProps": {"statusCode": 404}}}))
	print("not_found.html written")


if __name__ == "__main__":
	main()
