# Downloading and verifying

[← All guides](README.md)

SnapIntel saves media exactly as Snapchat serves it and keeps a record beside the files, so that later you can show where each file came from and that it hasn't changed.

<img src="media/walkthrough-download.gif" width="820" alt="Ticking three lenses, pressing Download, watching the Downloads panel, then pressing Check files.">

## Choosing what to save

| To save | Do this |
|---|---|
| Everything the profile's page lists | **Download all** in the profile's header |
| One tab's items | **Download all** at the top of that tab |
| A few items | Tick the circle in the corner of each, then **Download** in the bar at the bottom |
| All the snaps of one highlight | Tick the highlight's card |
| The item you are viewing | **Download** in the viewer |

**Clear** in the selection bar, or <kbd>Esc</kbd>, drops the selection. Ticks stay as you move between tabs, so you can gather items from several.

## Watching it happen

The **Downloads** panel opens when a download of several files starts, and the download button at the top right reopens it at any time. It shows the progress, each file as it lands with its size, and a **Stop** button. A single file saved from the viewer finishes with a short message and a **Show** button that opens its folder.

Only one download runs at a time; start another when it finishes.

## Where files go

```
Downloads/
  SnapIntel/
    <username>/
      story_<id>.mp4
      highlight_<id>.jpg
      spotlight_<id>.mp4
      lens_<id>.mp4
      snapintel-manifest.json
```

- Each profile gets its own folder. Turn that off, or choose another place, in [Settings](settings.md).
- A file's name is its kind and Snapchat's media ID, so the same item always gets the same name.
- A file's "modified" date is set to when it was **uploaded to Snapchat**, where Snapchat gives that date. Sort the folder by date and it reads as a timeline.
- **Open folder** in the Downloads panel shows the folder in your file manager.

## Which version of a video is saved

Snapchat keeps two versions of most videos:

| Version | Size | Picture |
|---|---|---|
| **No watermark** | 480 × 852 | As it was posted. |
| **Largest** | 540 × 960, at a higher bitrate | With Snapchat's logo and the profile's username burned into the top left corner. |

SnapIntel plays and saves the version **without the watermark**, for every kind of video. Snapchat's own page plays the watermarked one for Spotlight, so a Spotlight in SnapIntel is slightly smaller than on snapchat.com, and clean.

To play and save the larger version instead, choose **Largest** under **Videos** in [Settings](settings.md). Either way:

- A few videos exist in one version only. They are saved as they are, and if that means a watermark you asked not to have, the Downloads panel names them.
- When you download again after changing the setting, files saved as the other version are **replaced**, provided they are still exactly what was downloaded. A file you changed is left alone, as always.
- Photos have a single full-size version (up to 1080 × 1920 for story photos, 720 × 1280 for highlight photos), and lens previews are saved as Snapchat provides them.

These are the largest versions Snapchat publishes to visitors who aren't signed in. SnapIntel doesn't enlarge anything.

## The record: `snapintel-manifest.json`

For every file the manifest stores:

- the address it was downloaded from,
- when it was downloaded,
- its size and its **SHA-256** hash,
- what it is (story, highlight, Spotlight, lens, Bitmoji),
- and its upload date, when Snapchat gives one.

It is a plain JSON file you can read, archive with the media, or hand to someone else along with them.

## Downloading again

Run the same download later and SnapIntel only fetches what is new. Files it already saved are skipped and counted as **already saved**.

A saved file that no longer matches its recorded hash is **never overwritten**. SnapIntel leaves it alone, saves the fresh copy beside it, and tells you which files had changed.

A stopped or failed download can simply be started again: finished files are kept and unfinished ones are fetched from the start.

## Check files

**Check files** in the Downloads panel re-hashes every file in the folder and compares it with the manifest:

<img src="media/downloads.webp" width="820" alt="The Downloads panel after Check files: all 3 files match what was downloaded.">

- **All files match** means every file is byte-for-byte what was downloaded.
- **Changed** lists files whose contents differ from the record.
- **Missing** lists files in the record that are no longer in the folder.
- Media files that are in the folder but not in the record are counted separately.

## Working with the command-line tool

The [command-line tool](../cli/README.md) writes the same manifest. A folder started by one can be continued by the other, and `snapintel --verify <folder>` runs the same check from a terminal.
