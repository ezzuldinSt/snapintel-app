# Troubleshooting

[← All guides](README.md)

## Installing

**Windows says "Windows protected your PC".**
The installer is not code-signed, so SmartScreen doesn't recognise it. Choose **More info**, then **Run anyway**. You can confirm the file first by comparing its SHA-256 with `SHA256SUMS.txt` in the release.

**The AppImage does nothing when I double-click it.**
Make it executable: `chmod +x SnapIntel_1.1.1_amd64.AppImage`. On a system without FUSE, run it with `--appimage-extract-and-run`.

**The window is blank or flickers on Linux.**
This is a known problem between WebKitGTK and some graphics drivers. Start the app with:

```bash
WEBKIT_DISABLE_DMABUF_RENDERER=1 snapintel
```

## Looking up profiles

**"That isn't a username".**
A username is made of letters, digits, `.`, `-` and `_`, with no spaces. Display names such as "Team Snapchat" are not usernames. Paste the profile's link if you are unsure of the spelling.

**The profile can't be found.**
The username doesn't exist, was renamed, or the account was removed. Check it at `https://www.snapchat.com/add/<username>` in a browser.

**The profile shows only a name and a Snapcode.**
It isn't a public profile. That is all Snapchat publishes for it: see [profiles that aren't public](profiles.md#profiles-that-arent-public).

**There are fewer highlights or Spotlights than in the Snapchat app.**
Snapchat's page lists only the newest of each. A `+` after a count means there are more. See [what Snapchat doesn't publish](privacy.md#what-snapchat-doesnt-publish).

**A figure shows a dash.**
Snapchat hides that figure for this profile or video.

**Details that used to be there are missing, or the app says the page has changed.**
Snapchat has probably changed its page. Open Settings and press **Check now** under Compatibility: it lists exactly what can no longer be found. Then look for a newer release, or [report it](https://github.com/ezzuldinSt/snapintel-app/issues).

**Lookups time out.**
Raise **Give up after** in Settings. If you are behind a proxy or a filter, make sure `snapchat.com` and `sc-cdn.net` are reachable.

## Playing media

**"This can't be played here".**
The media may have expired (stories do, after 24 hours), or your system lacks a video codec.

- **Debian, Ubuntu:** `sudo apt install gstreamer1.0-libav gstreamer1.0-plugins-good`
- **Fedora:** install `gstreamer1-libav` and `gstreamer1-plugins-good` (from RPM Fusion).
- **Any system:** **Open in the browser** plays it there, and downloading it always works.

**There are no captions for a video.**
Snapchat hasn't transcribed it. The **Show captions** button only appears when it has.

## Downloading

**Downloaded videos are small (480 × 852).**
That is the largest version Snapchat publishes without its watermark. A 540 × 960 version with the watermark exists for Spotlight and some highlights; choose **Largest** under **Videos** in Settings to save that one. See [which version is saved](downloads.md#which-version-of-a-video-is-saved).

**"Already saved: nothing new to download".**
Every file you asked for is already in the folder and matches its record. That is the expected result of repeating a download.

**"A download is already running".**
Only one runs at a time. Start the next when it finishes, or press **Stop** in the Downloads panel.

**Some files failed.**
Start the same download again: finished files are skipped and only the failed ones are fetched. If it keeps failing, lower **Downloads at once** in Settings.

**Check files says a file has changed.**
Something modified it after it was downloaded: an editor, a sync tool, or a re-encode. The original can be fetched again while Snapchat still has it; SnapIntel saves the fresh copy beside the changed one and overwrites nothing.

## Starting over

Delete the settings file to return to the defaults ([where it is](settings.md#where-settings-are-stored)). Your downloads are not touched.

## Still stuck?

[Open an issue](https://github.com/ezzuldinSt/snapintel-app/issues) with your system, the SnapIntel version (bottom of Settings), and what you did. Please don't include other people's personal details.
