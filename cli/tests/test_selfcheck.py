import copy
import unittest

from helpers import FakeWeb, page_from_json, page_json, plain, run


class SelfCheck(unittest.TestCase):
	def test_snapshots_match_config(self):
		code, out, _ = run(["--self-check"])
		self.assertEqual(code, 0, plain(out))
		self.assertIn("config.json matches Snapchat's page", plain(out))
		self.assertNotIn("MISSING", plain(out))

	def test_renamed_field_is_reported(self):
		page = copy.deepcopy(page_json("public_profile.html"))
		info = page["props"]["pageProps"]["userProfile"]["publicProfileInfo"]
		info["biography"] = info.pop("bio")
		code, out, _ = run(["--self-check", "spectacles"], web=FakeWeb(pages={"spectacles": page_from_json(page)}))
		text = plain(out)
		self.assertEqual(code, 1)
		self.assertIn("MISSING  bio", text)
		self.assertIn("biography", text)    # the keys found there point at the new name

	def test_moved_section_is_reported(self):
		page = copy.deepcopy(page_json("public_profile.html"))
		page["props"]["pageProps"]["profile"] = page["props"]["pageProps"].pop("userProfile")
		code, out, _ = run(["--self-check", "spectacles"], web=FakeWeb(pages={"spectacles": page_from_json(page)}))
		self.assertEqual(code, 1)
		self.assertIn("no 'userProfile' under props.pageProps", plain(out))


if __name__ == "__main__":
	unittest.main()
