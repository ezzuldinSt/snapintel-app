"""--self-check: does Snapchat's profile page still match config.json?

SnapIntel reads every field through a fixed path into the JSON that snapchat.com embeds in each profile
page (config.json, "json_data_paths"). When Snapchat renames or moves something, those fields quietly come
back empty or wrong. This loads reference profiles and reports, for every path, whether it still resolves;
for one that doesn't, it shows where the path stops and which keys are there instead, which is usually
enough to fix config.json. The exit code is 1 when a field that should be there no longer resolves.
"""
import json
import re

import requests
from colorama import Fore, Style

PUBLIC_PAGE = 18   # pageMetadata.pageType of a public profile (publicProfileInfo)
OTHER_PAGE = 17    # the non-public layout (userInfo with a 3D Bitmoji)

COMMON_FIELDS = {"pageType", "pageTitle", "pageDescription"}
OTHER_PAGE_FIELDS = {"private_username", "displayName", "avatarImageUrl", "backgroundImageUrl", "private_snapcodeImageUrl"}
# Fields a correct page may lack, because the account has no such content right now.
MAY_BE_EMPTY = {
	"story": "only present while the account has a story up",
	"websiteUrl": "only when the account lists a website",
}

_PLACEHOLDER = re.compile(r"\{.*\}")


def _resolve(data, path):
	"""Walk a config path. Returns (value, None) or (None, (keys walked, key that failed, node there))."""
	node, walked = data, []
	for key in path.split("."):
		if _PLACEHOLDER.fullmatch(key):
			if isinstance(node, list) and node:
				node, key = node[0], "0"
			else:
				return None, (walked, key, node)
		elif isinstance(node, dict) and key in node:
			node = node[key]
		else:
			return None, (walked, key, node)
		walked.append(key)
	return node, None


def _describe(value):
	if isinstance(value, list):
		return f"list[{len(value)}]"
	if isinstance(value, dict):
		return f"dict({len(value)} keys)"
	if isinstance(value, str):
		text = value.replace("\n", " ")
		return f'str "{text[:40]}{"..." if len(text) > 40 else ""}"'
	return f"{type(value).__name__} {value!r}"


def _fetch(snap_dl, account, timeout):
	try:
		response = requests.get("https://www.snapchat.com/add/" + account, headers=snap_dl.headers, timeout=timeout)
	except requests.exceptions.RequestException as error:
		return None, f"request failed: {error}"
	match = re.search(snap_dl.json_regex, response.text, re.DOTALL)
	if not match:
		return None, f"HTTP {response.status_code}, but no embedded page JSON (the page format itself changed?)"
	try:
		return json.loads(match.group(1)), f"HTTP {response.status_code}"
	except json.JSONDecodeError as error:
		return None, f"HTTP {response.status_code}, page JSON does not parse: {error}"


def run(snap_dl, accounts):
	paths = snap_dl.json_data_paths
	timeout = snap_dl.parser.timeout
	print(f"{Style.BRIGHT}SnapIntel self-check: {len(paths)} paths in config.json against snapchat.com{Style.RESET_ALL}")
	failures, totals = 0, {"ok": 0, "missing": 0, "empty": 0, "skipped": 0}
	checked = set()

	for account in accounts:
		data, status = _fetch(snap_dl, account, timeout)
		print(f"\n{Style.BRIGHT}[{account}]{Style.RESET_ALL} {status}")
		if data is None:
			failures += 1
			continue
		page_type, _ = _resolve(data, paths.get("pageType", ""))
		layout = {PUBLIC_PAGE: "public profile", OTHER_PAGE: "non-public profile"}.get(page_type, "unknown page type")
		print(f"  page type {page_type} ({layout})")
		if page_type not in (PUBLIC_PAGE, OTHER_PAGE):
			print(f"  {Fore.RED}Not a profile page: is '{account}' a real account, or did pageType move?{Style.RESET_ALL}")
			failures += 1
			continue

		for name, path in paths.items():
			applies = name in COMMON_FIELDS or (name in OTHER_PAGE_FIELDS) == (page_type == OTHER_PAGE)
			if not applies:
				continue
			value, stop = _resolve(data, path)
			if stop is None and value is None:
				totals["empty"] += 1
				print(f"  {Fore.YELLOW}EMPTY{Style.RESET_ALL}    {name:<26} present but null on this account")
				continue
			if stop is None:
				totals["ok"] += 1
				checked.add(name)
				print(f"  {Fore.GREEN}OK{Style.RESET_ALL}       {name:<26} {_describe(value)}")
				continue
			walked, key, node = stop
			where = ".".join(walked) or "(page root)"
			if _PLACEHOLDER.fullmatch(key) and isinstance(node, list):
				totals["skipped"] += 1
				print(f"  {Fore.YELLOW}SKIPPED{Style.RESET_ALL}  {name:<26} {where} is an empty list on this account, nothing to index")
				continue
			if node is None and walked:
				# The key is on the page but null: the layout is intact, this account just has nothing there.
				totals["empty"] += 1
				print(f"  {Fore.YELLOW}EMPTY{Style.RESET_ALL}    {name:<26} {where} is null on this account")
				continue
			if name in MAY_BE_EMPTY and len(walked) >= len(path.split(".")) - 2:
				totals["empty"] += 1
				print(f"  {Fore.YELLOW}EMPTY{Style.RESET_ALL}    {name:<26} not on this page ({MAY_BE_EMPTY[name]})")
				continue
			totals["missing"] += 1
			failures += 1
			print(f"  {Fore.RED}MISSING{Style.RESET_ALL}  {name:<26} no '{key}' under {where}")
			if isinstance(node, dict):
				keys = sorted(node.keys())
				print(f"           {'':<26} keys there: {', '.join(keys[:15])}{' ...' if len(keys) > 15 else ''}")
			else:
				print(f"           {'':<26} {where} is {_describe(node)}")

	unchecked = [n for n in paths if n not in checked]
	print(f"\n{Style.BRIGHT}Summary over {len(accounts)} account(s):{Style.RESET_ALL} {totals['ok']} OK, "
	      f"{totals['missing']} missing, {totals['empty']} empty, {totals['skipped']} skipped")
	if unchecked:
		print(f"Not confirmed, because they were empty or skipped on every account checked: {', '.join(unchecked)}")
		print(f"Check them against an account that has that content: snapintel --self-check USERNAME")
	if failures:
		print(f"{Fore.RED}config.json no longer matches Snapchat's page for the fields marked MISSING.{Style.RESET_ALL}")
		return 1
	print(f"{Fore.GREEN}config.json matches Snapchat's page.{Style.RESET_ALL}")
	return 0
