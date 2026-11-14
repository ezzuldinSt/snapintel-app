import os
import re
import sys
import threading

from colorama import Fore, Style

# Windows consoles (and most terminals) draw text left to right in stored order and don't join Arabic
# letters, so Arabic profile text comes out reversed and broken apart. When printing to a console,
# RtlConsole joins the letters (arabic_reshaper) and puts each right-to-left run in display order
# (python-bidi), one line and one colour segment at a time. Output sent to a file or pipe keeps the
# original text. SNAPINTEL_RTL=0 turns this off (for a terminal with its own RTL support), =1 forces it.
_RTL_CHARS = re.compile(r'[֐-ࣿיִ-﷿ﹰ-ﻼ]')
_ARABIC_CHARS = re.compile(r'[؀-ࣿﭐ-﷿ﹰ-ﻼ]')
_ANSI_ESCAPE = re.compile(r'(\x1b\[[0-9;?]*[ -/]*[@-~])')
# An emoji with what follows it (variation selector, skin tone, keycap, ZWJ sequence). Reordering would put
# those after-characters in front of the emoji, so each sequence rides through as one stand-in symbol
# (Control Pictures U+2400-2426: neutral like the emoji, and not expected in profile text).
_EMOJI = '[←-⯿〰〽㊗㊙\U0001F000-\U0001FAFF]'
_EMOJI_SEQUENCE = re.compile(_EMOJI + '(?:[︎️⃣\U0001F3FB-\U0001F3FF]|‍' + _EMOJI + ')+')
_STAND_INS = [chr(c) for c in range(0x2400, 0x2427)]


def _visual_segment(segment, reshape, get_display):
	if not _RTL_CHARS.search(segment):
		return segment
	core = segment.strip()
	lead = segment[:len(segment) - len(segment.lstrip())]
	trail = segment[len(segment.rstrip()):]
	if _ARABIC_CHARS.search(core):
		core = reshape(core)
	sequences = []
	if not any(s in core for s in _STAND_INS):
		def stash(match):
			if len(sequences) == len(_STAND_INS):
				return match.group(0)
			sequences.append(match.group(0))
			return _STAND_INS[len(sequences) - 1]
		core = _EMOJI_SEQUENCE.sub(stash, core)
	visual = get_display(core)
	for stand_in, sequence in zip(_STAND_INS, sequences):
		visual = visual.replace(stand_in, sequence)
	return lead + visual + trail


class RtlConsole:
	def __init__(self, stream, reshape, get_display):
		self._stream = stream
		self._reshape = reshape
		self._get_display = get_display
		self._pending = ""
		self._lock = threading.Lock()

	def _visual_line(self, line):
		if not _RTL_CHARS.search(line):
			return line
		parts = _ANSI_ESCAPE.split(line)
		return "".join(p if _ANSI_ESCAPE.fullmatch(p) else _visual_segment(p, self._reshape, self._get_display) for p in parts)

	def write(self, text):
		with self._lock:
			self._pending += text
			if "\n" in self._pending:
				*lines, self._pending = self._pending.split("\n")
				self._stream.write("".join(self._visual_line(l) + "\n" for l in lines))
		return len(text)

	def flush(self):
		with self._lock:
			if self._pending:
				self._stream.write(self._visual_line(self._pending))
				self._pending = ""
			self._stream.flush()

	def __getattr__(self, name):
		return getattr(self._stream, name)


def enable_rtl_console():
	mode = os.environ.get("SNAPINTEL_RTL", "auto").strip().lower()
	if mode in ("0", "off", "no", "false"):
		return
	if mode not in ("1", "on", "yes", "true") and not sys.stdout.isatty():
		return
	try:
		import arabic_reshaper
		try:
			# python-bidi's pure-Python implementation mirrors brackets: "(@user)" in Arabic text shows as
			# "(user@)", as a browser draws it. Its Rust get_display (python-bidi 0.5+) leaves ")user@(".
			from bidi.algorithm import get_display
		except ImportError:
			from bidi import get_display
	except ImportError:
		return
	sys.stdout = RtlConsole(sys.stdout, arabic_reshaper.reshape, get_display)


class Display:
	def __init__(self, parser, ssd):
		self.parser = parser
		self.ssd = ssd

	def print_basic_information(self, data):
		if(self.parser.list_user or self.parser.list_all or self.parser.stats):
			print(rf"""{Fore.YELLOW}
                                    _   
     /\                            | |  
    /  \   ___ ___ ___  _   _ _ __ | |_ 
   / /\ \ / __/ __/ _ \| | | | '_ \| __|
  / ____ \ (_| (_| (_) | |_| | | | | |_ 
 /_/    \_\___\___\___/ \__,_|_| |_|\__|

				{Style.RESET_ALL}""")

			print(f"{Fore.GREEN}[+] Page title:{Style.RESET_ALL} {data[0]}")
			print(f"{Fore.GREEN}[+] Page description:{Style.RESET_ALL} {data[1]}")

			#If false then account is public
			if(not data[2]):
				print(f"{Fore.GREEN}[+] This is a public profile{Style.RESET_ALL}")
				print(f"{Fore.GREEN}[+] Username:{Style.RESET_ALL} {data[3]}")
				if(data[4] == 1):
					print(f"{Fore.GREEN}[+] Badge:{Style.RESET_ALL} Creator")
				elif(data[4] == 3):
					print(f"{Fore.GREEN}[+] Badge:{Style.RESET_ALL} Public Figure")
				else:
					print(f"{Fore.GREEN}[+] Badge:{Style.RESET_ALL} {Fore.RED}None{Style.RESET_ALL}")
				print(f"{Fore.GREEN}[+] Profile picture:{Style.RESET_ALL} {data[5]}")
				print(f"{Fore.GREEN}[+] Background picture:{Style.RESET_ALL} {data[6]}")
				print(f"{Fore.GREEN}[+] Subscriber count:{Style.RESET_ALL} {data[7]}")
				print(f"{Fore.GREEN}[+] Bio:{Style.RESET_ALL} {data[8]}")
				if(len(data[9]) != 0):
					print(f"{Fore.GREEN}[+] Website:{Style.RESET_ALL} {data[9]}")
				else:
					print(f"{Fore.GREEN}[+] Website:{Style.RESET_ALL} None")
				print(f"{Fore.GREEN}[+] Snap code:{Style.RESET_ALL} {data[10]}")
			else:
				print(f"{Fore.RED}[-] This is a private profile{Style.RESET_ALL}")
				print(f"{Fore.GREEN}[+] Username:{Style.RESET_ALL} {data[3]}")
				print(f"{Fore.GREEN}[+] Display name:{Style.RESET_ALL} {data[4]}")
				print(f"{Fore.GREEN}[+] Avatar image (current bitmoji):{Style.RESET_ALL} {data[5]}")
				print(f"{Fore.GREEN}[+] Background image:{Style.RESET_ALL} {data[7]}")
				print(f"{Fore.GREEN}[+] Snap code:{Style.RESET_ALL} {data[8]}")

	def print_stats(self, data):
		if(self.parser.stats):
			print(rf"""{Fore.YELLOW}
	   _____ _        _       
	  / ____| |      | |      
	 | (___ | |_ __ _| |_ ___ 
	  \___ \| __/ _` | __/ __|
	  ____) | || (_| | |_\__ \
	 |_____/ \__\__,_|\__|___/
				{Style.RESET_ALL}""")

			#Stories
			print(f"\n{Fore.GREEN}[+] Number of stories:{Style.RESET_ALL} {data[0][0]}")
			
			#Curated highlights

			try:
				print(f"\n{Fore.GREEN}[+] Number of curated highlight stories:{Style.RESET_ALL} {data[1][0]}")
				total = 0
				for story in data[1][1:]:
					total += len(story[1:])
			except:
				total = 0
			print(f"{Fore.GREEN}[+] Number of total snaps in curated highlights:{Style.RESET_ALL} {total}")

			#Spotlights
			print(f"\n{Fore.GREEN}[+] Number of spotlights:{Style.RESET_ALL} {data[2][0]}")

			total = 0
			time_strings = list()
			for spotlight in data[2][1:-1]:
				#print(spotlight)
				try:
					total += len(spotlight[7])
				except IndexError:
					total += len(spotlight[6])

				time_strings.append(spotlight[2])

			print(f"{Fore.GREEN}[+] Number of total snaps in spotlights:{Style.RESET_ALL} {total}")
			print(f"{Fore.GREEN}[+] Total snaps' duration in spotlights:{Style.RESET_ALL} {self.ssd.time_str_list_to_seconds(time_strings)}")

			try:
				if(data[2][-1] > 0):
					print(f"{Fore.GREEN}[+] Total spotlights engagement stats:{Style.RESET_ALL} {data[2][-1]}")
			except:
				pass

			#Lenses
			print(f"\n{Fore.GREEN}[+] Number of lenses:{Style.RESET_ALL} {data[3][0]}")


	def print_stories(self, data):	
		if(self.parser.list_stories or self.parser.list_all):
			print(rf"""{Fore.YELLOW}
   _____ _             _           
  / ____| |           (_)          
 | (___ | |_ ___  _ __ _  ___  ___ 
  \___ \| __/ _ \| '__| |/ _ \/ __|
  ____) | || (_) | |  | |  __/\__ \
 |_____/ \__\___/|_|  |_|\___||___/
				{Style.RESET_ALL}""")

			print(f"{Fore.GREEN}[+] Number of stories: {data[0]}")
			EXTS = ["Image (JPG)", "Video (MP4)"]

			for snap in data[1:]:
				print(f"{Fore.GREEN}[+] ID:{Style.RESET_ALL} {Fore.YELLOW}{snap[0]}{Style.RESET_ALL}")
				print(f"\t{Fore.GREEN}[+] URL:{Style.RESET_ALL} {snap[1]}")
				print(f"\t{Fore.GREEN}[+] Upload date:{Style.RESET_ALL} {snap[2]}")
				print(f"\t{Fore.GREEN}[+] File type:{Style.RESET_ALL} {EXTS[snap[3]]}\n")



	def print_curated_highlights(self, data):	
		if(self.parser.list_highlights or self.parser.list_all):

			try:
				#Random test to trigger NoneType error
				if(data[0] == "test"):
					pass
			except:
				return

			print(rf"""{Fore.YELLOW}
   _____                _           _   _    _ _       _     _ _       _     _       
  / ____|              | |         | | | |  | (_)     | |   | (_)     | |   | |      
 | |    _   _ _ __ __ _| |_ ___  __| | | |__| |_  __ _| |__ | |_  __ _| |__ | |_ ___ 
 | |   | | | | '__/ _` | __/ _ \/ _` | |  __  | |/ _` | '_ \| | |/ _` | '_ \| __/ __|
 | |___| |_| | | | (_| | ||  __/ (_| | | |  | | | (_| | | | | | | (_| | | | | |_\__ \
  \_____\__,_|_|  \__,_|\__\___|\__,_| |_|  |_|_|\__, |_| |_|_|_|\__, |_| |_|\__|___/
                                                  __/ |           __/ |              
                                                 |___/           |___/               
				{Style.RESET_ALL}""")

			print(f"{Fore.GREEN}[+] Number of stories:{Style.RESET_ALL} {data[0]}")

			total = 0
			for story in data[1:]:
				total += len(story[1:])
			print(f"{Fore.GREEN}[+] Number of total snaps in curated highlights:{Style.RESET_ALL} {total}")

			for story in data[1:]:
				print(f"\n{Fore.GREEN}[+] Story title:{Style.BRIGHT}{Style.RESET_ALL} {story[0]}")
				print(f"{Fore.GREEN}[+] Number of snaps:{Style.RESET_ALL} {len(story[1:])}")
				
				for snap in story[1:]:
					print(f"\t{Fore.GREEN}[+] ID:{Style.RESET_ALL} {Fore.YELLOW}{snap[0]}{Style.RESET_ALL}")
					print(f"\t\t{Fore.GREEN}[+] URL:{Style.RESET_ALL} {snap[1]}")
					print(f"\t\t{Fore.GREEN}[+] Upload date:{Style.RESET_ALL} {snap[2]}")

			



	def print_spotlights(self, data):
		if(self.parser.list_spotlights or self.parser.list_all):
			print(rf"""{Fore.YELLOW}
   _____             _   _ _       _     _       
  / ____|           | | | (_)     | |   | |      
 | (___  _ __   ___ | |_| |_  __ _| |__ | |_ ___ 
  \___ \| '_ \ / _ \| __| | |/ _` | '_ \| __/ __|
  ____) | |_) | (_) | |_| | | (_| | | | | |_\__ \
 |_____/| .__/ \___/ \__|_|_|\__, |_| |_|\__|___/
        | |                   __/ |              
        |_|                  |___/               
						{Style.RESET_ALL}""")

			print(f"{Fore.GREEN}[+] Number of spotlights:{Style.RESET_ALL} {data[0]}")
			time_strings = list()

			for spotlight in data[1:-1]:
				print(f"\n{Fore.GREEN}[+] Spotlight name:{Style.RESET_ALL} {Style.BRIGHT}{Fore.YELLOW}{spotlight[1]}{Style.RESET_ALL}")
				print(f"{Fore.GREEN}[+] Thumbnail:{Style.RESET_ALL} {spotlight[0]}")
				print(f"{Fore.GREEN}[+] Duration:{Style.RESET_ALL} {spotlight[2]}")
				time_strings.append(spotlight[2])
				print(f"{Fore.GREEN}[+] Upload date:{Style.RESET_ALL} {spotlight[3]}")
				if(len(spotlight[4]) != 0):
					print(f"{Fore.GREEN}[+] Engagement stats:{Style.RESET_ALL} {spotlight[4]}")
				print(f"{Fore.GREEN}[+] Hashtags:{Style.RESET_ALL} {', '.join(spotlight[5])}")
				

				print(f"{Fore.GREEN}[+] Snaps:{Style.RESET_ALL}")
				try:
					for story in spotlight[7]:
						print(f"\t{Fore.GREEN}[+] ID:{Style.RESET_ALL} {Fore.YELLOW}{story[0]}{Style.RESET_ALL}")
						print(f"\t{Fore.GREEN}[+] URL:{Style.RESET_ALL} {story[1]}\n")
				except IndexError:
					for story in spotlight[6]:
						print(f"\t{Fore.GREEN}[+] ID:{Style.RESET_ALL} {Fore.YELLOW}{story[0]}{Style.RESET_ALL}")
						print(f"\t{Fore.GREEN}[+] URL:{Style.RESET_ALL} {story[1]}\n")

			if(data[-1] > 0):
				print(f"\n{Fore.GREEN}[+] Total engagement stats:{Style.RESET_ALL} {data[-1]}")
			print(f"{Fore.GREEN}[+] Total snaps' duration in spotlights:{Style.RESET_ALL} {self.ssd.time_str_list_to_seconds(time_strings)}")

			try:
				if(len(data[-2][5]) > 0):
					print(f"{Fore.GREEN}[+] Top 10 hashtags:{Style.RESET_ALL}")
					for hashtag, count in data[-2][6]:
						print(f"\t{hashtag}: {count} time(s)")
			except TypeError:
				pass

	def print_lenses(self, data):
		if(self.parser.list_lenses or self.parser.list_all):
			print(rf"""{Fore.YELLOW}
.__                                      
|  |   ____   ____   ______ ____   ______
|  | _/ __ \ /    \ /  ___// __ \ /  ___/
|  |_\  ___/|   |  \\___ \\  ___/ \___ \ 
|____/\___  >___|  /____  >\___  >____  >
          \/     \/     \/     \/     \/               
					{Style.RESET_ALL}""")

			print(f"{Fore.GREEN}[+] Number of lenses:{Style.RESET_ALL} {data[0]}")
			for lense in data[1:]:
				print(f"{Fore.GREEN}[+] Lense title:{Style.RESET_ALL} {lense[0]}")
				print(f"\t{Style.BRIGHT}Is lense official?{Style.RESET_ALL} {lense[1]}")
				print(f"\t{Style.BRIGHT}Preview video URL:{Style.RESET_ALL} {lense[2]}")


	def print_bitmojis(self, data, download):
		if(self.parser.list_bitmojis or self.parser.list_all):
			print(rf"""{Fore.YELLOW}
__________.__  __                     __.__        
\______   \__|/  |_  _____   ____    |__|__| ______
 |    |  _/  \   __\/     \ /  _ \   |  |  |/  ___/
 |    |   \  ||  | |  Y Y  (  <_> )  |  |  |\___ \ 
 |______  /__||__| |__|_|  /\____/\__|  |__/____  >
        \/               \/      \______|       \/ 
        {Style.RESET_ALL}""")

			try:
				print(f"{Fore.GREEN}[+] Number of different bitmojis:{Style.RESET_ALL} {len(data)}")
			except TypeError:
				print(f"{Fore.GREEN}[+] Number of different bitmojis:{Style.RESET_ALL} 0")
			if(not download):
				print("If you want to download the bitmojis, take a look at the --download/-d option!")