from . import display, ssd


def main():
	display.enable_rtl_console()
	snap_dl = ssd.Snap_Story_Downloader()
	snap_dl.run()
