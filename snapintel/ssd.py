import requests
import re
import sys
import json
import datetime
import hashlib
import concurrent.futures
from pathlib import Path

from . import display, downloads, heatmap, snap_parser

class Snap_Story_Downloader:
	def __init__(self):
		self.parser = snap_parser.Parser()

		self.url = "https://www.snapchat.com/add/"
		self.url_regex = r'https://(bolt-gcdn\.sc-cdn\.net|cf-st\.sc-cdn\.net)/[^"]*uc=75'
		self.url_regex2 = r'\.256\.'
		self.headers = {'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3'}
		self.json_regex = r'<script[^>]+type="application/json"[^>]*>(.*?)</script>'
		self.json_data = None
		self.downloads = None   # a downloads.DownloadManager when -d is given
		self.heatmap = heatmap.Snap_Heatmap()
		self.json_data_paths = None
		self.json_data = None

		self.private_user = False

		self.display = display.Display(self.parser, self)


	def update_url(self):
		self.url = "https://www.snapchat.com/add/" + self.parser.username

	# Perform the web request to snapchat
	def make_request(self):
		try:
			response = requests.get(self.url, timeout=self.parser.timeout, headers=self.headers).text
			self.get_info(response)
			return True
		except requests.exceptions.Timeout:
			print("Timeout reached")
			return False


	def load_paths(self, response=None, file="config.json"):
		# config.json ships inside the package, next to this module, so the command works from any folder.
		with (Path(__file__).parent / file).open("r", encoding="utf-8") as config_file:
			config_data = json.load(config_file)

		self.json_data_paths = config_data.get("json_data_paths", {})

	# Returns the corresponding value given a path and a data
	def get_value(self, data_name, placeholders=None):
		path = self.json_data_paths.get(data_name)
		if path:
			data = self.json_data
			for key in path.split('.'):
				try:
					# Check if there is a keyword in a path in JSON file (such as test.{count}.test). In this case, we replace the 
					# keyword by the corresponding value from a variable in the code
					match = re.match(r"{(.*?)}", key)
					if(match):
						data = data[placeholders]
					else:
						data = data.get(key, {})
				except AttributeError:
					pass
		return data


	# Returns a list of all bitmoji versions for a user
	def bitmojis(self, url) -> list:
		if(self.parser.list_bitmojis or self.parser.list_all or self.parser.download_bitmojis or self.parser.download_all):
			result = set()  # Use a set to store unique md5 hashes

			# Some accounts have no 3D Bitmoji, so there is no avatar URL to derive versions from.
			if(not url):
				return []

			# Split the URL to get the base and end parts
			base_url = url.split("_")[0]
			last_version = url.split("_")[1].split("-")[0]
			end_url = '-'.join(url.split("-")[4:])

			# Use ThreadPoolExecutor to download and process bitmoji versions in parallel
			with concurrent.futures.ThreadPoolExecutor(max_workers=self.parser.threads) as executor:
				# Create a list of futures, one for each bitmoji version
				futures = [
					executor.submit(self.process_bitmoji_version, base_url, end_url, i)
					for i in reversed(range(1, int(last_version) + 1))
				]

				# As futures complete, collect the results
				for future in concurrent.futures.as_completed(futures):
					md5_hash, content, version_url = future.result()
					if md5_hash and md5_hash not in result:
						result.add(md5_hash)
						# Optional: Save content to a file (recorded in the download manifest)
						if(self.downloads is not None and (self.parser.download_bitmojis or self.parser.download_all)):
							try:
								self.downloads.store(f"bitmoji_{md5_hash}.webp", content, version_url, "bitmoji")
							except PermissionError:
								print("Error can't access the specified folder. Check permissions or run this program with sudo rights")
								exit(2)

			return list(result)

	# Function to process a single bitmoji version
	def process_bitmoji_version(self, base_url, end_url, version):
		try:
			# Construct the final URL for the current version
			final_url = '-'.join(['_'.join([base_url, str(version)]), end_url])

			# Make the HTTP request to get the bitmoji
			md5_hash = hashlib.md5()
			response = requests.get(final_url, timeout=self.parser.timeout, headers=self.headers)

			# Update the md5 hash with the response content
			for data in response.iter_content(8192):
				md5_hash.update(data)

			# Return the md5 hash, content and the URL it came from
			return md5_hash.hexdigest(), response.content, final_url
		except requests.exceptions.Timeout:
			print(f"Timeout reached for version {version}")
			return None, None, None

	#When calling the print function, check for the third item. If true then get some info else, get other ones
	def get_basic_information(self) -> list:
		#Get basic information
		result = list()

		page_type = self.get_value("pageType")
		page_title = self.get_value("pageTitle")
		
		if(not page_title):
			print("User doesn't exist")
			exit(1)

		#These first two results are always there
		result.append(page_title)
		result.append(self.get_value("pageDescription"))

		#If the snapchat account is public
		if(page_type == 18):
			result.append(self.private_user)
			result.append(self.get_value("username"))
			result.append(self.get_value("badge"))
			#Change profile picture size
			profilePictureUrl = re.sub(r'90(?=_FM(png|jpeg|gif)$)', "640", self.get_value("profilePictureUrl"))
			result.append(profilePictureUrl)
			result.append(self.get_value("squareHeroImageUrl"))
			result.append(self.get_value("subscriberCount"))
			result.append(self.get_value("bio"))
			result.append(self.get_value("websiteUrl"))
			result.append(self.get_value("snapcodeImageUrl"))
		# Not all private accounts have these pieces of information.
		else:
			self.private_user = True
			result.append(self.private_user)
			result.append(self.get_value("private_username"))
			result.append(self.get_value("displayName"))
			result.append(self.get_value("avatarImageUrl"))
			# Can't compute this for the stats option because it slows down the whole program only for that piece of information
			result.append(self.get_value("avatarImageUrl"))
			result.append(self.get_value("backgroundImageUrl"))
			result.append(self.get_value("private_snapcodeImageUrl"))

		return result

	def get_stories(self) -> list:
		# Story
		result = list()

		story = self.get_value("story")

		try:
			result.append(len(story))
			
			for snap in story:
				if(self.parser.download_stories or self.parser.download_all):
					self.queue_download(snap["snapUrls"]["mediaUrl"], snap["snapMediaType"], "story", self.snap_time(snap))
				if(self.parser.list_stories or self.parser.list_all):
					snap_item = list()
					snap_item.append(snap["snapIndex"])
					snap_item.append(snap["snapUrls"]["mediaUrl"])
					self.heatmap.fill_dates(datetime.datetime.utcfromtimestamp(int(snap["timestampInSec"]["value"])).strftime("%Y-%m-%d %H:%M:%S"))
					snap_item.append(datetime.datetime.utcfromtimestamp(int(snap["timestampInSec"]["value"])).strftime("%Y-%m-%d %H:%M:%S"))
					snap_item.append(snap["snapMediaType"])
					result.append(snap_item)
		except TypeError:
			result.append(0)

		
		#print(result)
		return result

	def get_curated_highlights(self) -> list:
		#Curated highlights
		result = list()
		#Check if user has uploaded highlights to avoid parsing for nothing.
		if(self.get_value("hasCuratedHighlights") != False):
			curated_highlights = self.get_value("curatedHighlights")


			result.append(len(curated_highlights))

			#List curated highlights
			for snap in curated_highlights:
				if(self.parser.list_highlights or self.parser.list_all or self.parser.download_highlights or self.parser.download_all or self.parser.stats):
					snap_item = list()
					snap_item.append(snap["storyTitle"]["value"])
					#Retrieve the same data as for basic snap stories
					for story in snap["snapList"]:
						if(self.parser.download_highlights or self.parser.download_all):
							self.queue_download(story["snapUrls"]["mediaUrl"], story["snapMediaType"], "highlight", self.snap_time(story))
						story_item = list()
						story_item.append(story["snapIndex"])
						story_item.append(story["snapUrls"]["mediaUrl"])

						try:
							story_item.append(datetime.datetime.utcfromtimestamp(int(story["timestampInSec"]["value"])).strftime("%Y-%m-%d %H:%M:%S"))
							self.heatmap.fill_dates(datetime.datetime.utcfromtimestamp(int(story["timestampInSec"]["value"])).strftime("%Y-%m-%d %H:%M:%S"))
						except KeyError:
							story_item.append("None")

						snap_item.append(story_item)

					result.append(snap_item)

			return result

	#Spotlights
	def get_spotlights(self) -> list:
		result = list()
		#Check if user has uploaded spotlights to avoid parsing for nothing.
		if(self.get_value("spotlightHighlights") != False):
			spotlight_highlights = self.get_value("spotlightHighlights")
			#print(len(spotlight_highlights))
			result.append(len(spotlight_highlights))

			total_engagements = 0

			#List spotlight highlights
			count = 0

			#Initialize a dictionary to count hashtags
			hashtag_counts = {}

			top_10_hashtags = dict()

			for spotlight_highlight in spotlight_highlights:

				if(self.parser.list_spotlights or self.parser.list_all or self.parser.download_spotlights or self.parser.download_all or self.parser.stats):
					try: 
						spotlight_item = list()
						spotlight_item.append(spotlight_highlight["thumbnailUrl"]["value"])
						#Get metadata
						#Pass the count variable as an argument to be able to convert it from the path to the actual value in the loop
						spotlight_name = self.get_value("spotlightName", count)
						spotlight_item.append(spotlight_name)
						#Convert ms to minutes:seconds
						minutes, seconds = self.ms_to_minutes_seconds(int(self.get_value("spotlightDuration", count)))
						spotlight_item.append(f"{minutes}m{seconds}s")

						spotlight_item.append(datetime.datetime.utcfromtimestamp(int(self.get_value("spotlightUploadDate", count))/1000).strftime("%Y-%m-%d %H:%M:%S"))
						self.heatmap.fill_dates(datetime.datetime.utcfromtimestamp(int(self.get_value("spotlightUploadDate", count))/1000).strftime("%Y-%m-%d %H:%M:%S"))

						engagement_stats = self.get_value("spotlightEngagementStats", count)
						total_engagements += int(engagement_stats)
						#Do not show information if negative because the data is not legitimate
						if(total_engagements > -1):
							spotlight_item.append(engagement_stats)

						hashtag_list = list()
						#Count hashtag occurrences
						for hashtag in self.get_value("spotlightHashtags", count):
							#print(hashtag)
							hashtag_list.append(hashtag)
							if hashtag in hashtag_counts:
								hashtag_counts[hashtag] += 1
							else:
								hashtag_counts[hashtag] = 1

						spotlight_item.append(hashtag_list)

						#Get the top 10 hashtags
						top_10_hashtags = sorted(hashtag_counts.items(), key=lambda x: x[1], reverse=True)[:10]

						spotlight_item.append(top_10_hashtags)
						stories = list()
						for story in spotlight_highlight["snapList"]:
							story_item = list()

							if(self.parser.download_spotlights or self.parser.download_all):
								self.queue_download(story["snapUrls"]["mediaUrl"], story["snapMediaType"], "spotlight", int(self.get_value("spotlightUploadDate", count)) / 1000)

							story_item.append(story["snapIndex"])
							story_item.append(story["snapUrls"]["mediaUrl"])

							stories.append(story_item)

						spotlight_item.append(stories)

						count += 1

						result.append(spotlight_item)
					except TypeError:
						pass

			#Print the top 10 hashtags and their counts
			# for hashtag, count in top_10_hashtags:
			# 	print(f"{hashtag}: {count} times")

			result.append(total_engagements)

		return result

	def get_lenses(self) -> list:
		#Lenses
		result = list()
		lenses = self.get_value("lenses")
		result.append(len(lenses))

		for lense in lenses:
			lense_item = list()
			if(self.parser.download_lenses or self.parser.download_all):
				self.queue_download(lense["lensPreviewVideoUrl"], 1, "lens")
			if(self.parser.list_lenses or self.parser.list_all or self.parser.stats):
				lense_item.append(lense["lensName"])
				lense_item.append(lense["isOfficialSnapLens"])
				lense_item.append(lense["lensPreviewVideoUrl"])
				result.append(lense_item)

		return result

	#Get information from the JSON data
	def get_info(self, response):
		#Load resources
		self.load_paths(response, "config.json")
		self.json_data = json.loads(self.extract_json(response))

		basic_information = self.get_basic_information()
		self.display.print_basic_information(basic_information)

		if(not self.private_user):
			stories = self.get_stories()
			self.display.print_stories(stories)

			curated_highlights = self.get_curated_highlights()
			self.display.print_curated_highlights(curated_highlights)

			spotlights = self.get_spotlights()
			self.display.print_spotlights(spotlights)

			lenses = self.get_lenses()
			self.display.print_lenses(lenses)

			self.display.print_stats([stories, curated_highlights, spotlights, lenses])

			if(self.parser.download_bool):
				self.download_files()

			if(self.parser.opt_heatmap):
				#Create the heatmap and visualize it
				self.heatmap.create_heatmap()
		else:
			bitmojis_information = self.bitmojis(basic_information[5])
			self.display.print_bitmojis(bitmojis_information, self.parser.download_bitmojis or self.parser.download_all)

			if(self.parser.download_bitmojis or self.parser.download_all):
				print(f"[+] All the {len(bitmojis_information)} bitmojis have been downloaded")
				

			print("\nUser is private, can't retrieve information about snaps")

	def time_str_to_seconds(self, time_str):
		minutes, seconds = map(int, re.findall(r'\d+', time_str))
		return minutes * 60 + seconds

	def time_str_list_to_seconds(self, time_list):
		#Compute seconds and minutes
		total_seconds = sum(self.time_str_to_seconds(time_str) for time_str in time_list)
		total_minutes = total_seconds // 60
		remaining_seconds = total_seconds % 60

		#Convert excess seconds into minutes
		total_minutes += remaining_seconds // 60
		remaining_seconds %= 60

		return f"{total_minutes}m{remaining_seconds}s"

	def ms_to_minutes_seconds(self, milliseconds):
	    # Convert milliseconds to seconds by dividing by 1000
	    seconds = milliseconds / 1000

	    # Calculate minutes and seconds
	    minutes = seconds // 60
	    seconds = seconds % 60

	    return int(minutes), int(seconds)

	#Extract the json from the response
	def extract_json(self, response):
		match = re.search(self.json_regex, response, re.DOTALL)

		# Check if a match was found
		if match:
			# Extract and print the JSON content
			json_data = match.group(1)
			try:
				return json.dumps(json.loads(json_data), indent=4)
			except json.JSONDecodeError as e:
				print("Failed to parse JSON:", e)
		else:
			print("No <script> tag with type 'application/json' found in the HTML response.")

		return None

	#Takes too much time for now
	#def compute_files_size(self):
	#	total_length = 0
	#	for url in self.mp4_files:
	#		try:
	#			response = requests.get(url, timeout=self.parser.timeout, headers=self.headers)
	#			length = response.headers.get("content-length")

	#			if(length != None):
	#				total_length += int(length)
	#		except requests.exceptions.Timeout:
	#			print("Timeout reached")

	#	print(f"Total size: {total_length/1000000:.2f} Mo")

	# Queue one media file for download; the manager names, de-duplicates and records it.
	def queue_download(self, url, media_type, category, uploaded_unix=None):
		if(self.downloads is not None):
			self.downloads.add(url, "jpg" if media_type == 0 else "mp4", category, downloads.iso_from_unix(uploaded_unix))

	# A snap's upload time in seconds since 1970, when Snapchat gives one.
	def snap_time(self, snap):
		try:
			return int(snap["timestampInSec"]["value"])
		except (KeyError, TypeError, ValueError):
			return None

	#Downloads the queued stories, highlights, spotlights and lenses. Files saved by an earlier run are skipped.
	def download_files(self):
		message = "stories, highlights, spotlights and lenses"

		if(self.parser.download_stories):
			message = "stories"
		elif(self.parser.download_highlights):
			message = "highlights"
		elif(self.parser.download_spotlights):
			message = "spotlights"

		print(f"Starting downloading {message} ...")
		counts = self.downloads.run()
		print(f"Downloaded {counts['downloaded']}, skipped {counts['skipped']} saved by an earlier run"
		      + (f", {counts['already_present']} identical to a file already there" if counts["already_present"] else "")
		      + (f", {counts['failed']} failed" if counts["failed"] else ""))
		if(self.downloads.altered):
			print(f"Changed on disk since they were downloaded, so left as they are (the new copy is saved beside each): {', '.join(self.downloads.altered)}")
		print(f"All {message} saved in {self.downloads.directory}, recorded in {downloads.MANIFEST}")

	#Main function
	def run(self):
		self.parser.build_arg_parser()
		if(self.parser.self_check):
			from . import selfcheck
			self.load_paths()
			sys.exit(selfcheck.run(self, self.parser.self_check_accounts))
		if(self.parser.verify_dir is not None):
			sys.exit(downloads.verify(self.parser.verify_dir))
		self.update_url()
		if(self.parser.download_bool):
			self.downloads = downloads.DownloadManager(self.parser.output_dir, self.parser.username, self.headers, self.parser.timeout, self.parser.threads)
		if(not self.parser.json_output):
			self.make_request()
			if(self.downloads is not None):
				self.downloads.save()   # bitmojis are stored as they are found, so record them too
			return

		# --json: collect into a JsonDisplay instead of printing, and send every other message (progress,
		# errors) to standard error, so standard output carries nothing but the JSON document.
		from . import jsonout
		json_stream = getattr(sys.stdout, "_stream", sys.stdout)   # past the RTL console wrapper: JSON keeps the original text
		self.display = jsonout.JsonDisplay(self.parser, self)
		sys.stdout = sys.stderr
		try:
			ok = self.make_request()
			if(self.downloads is not None):
				self.downloads.save()
		finally:
			sys.stdout = json_stream
		if(not ok or "profile" not in self.display.doc):
			sys.exit(1)
		if(self.downloads is not None):
			self.display.doc["downloads"] = self.downloads.summary()
		self.display.write(json_stream, self.parser.json_path)
		if(self.parser.json_path):
			print(f"JSON written to {self.parser.json_path}", file=sys.stderr)
