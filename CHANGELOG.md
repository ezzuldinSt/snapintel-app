# Changelog

All notable changes to SnapIntel. Versions follow [semantic versioning](https://semver.org).

## 1.0.0 (2026-11-14)

The first release of the desktop app, and of this repository as a fork of [Kr0wZ/SnapIntel](https://github.com/Kr0wZ/SnapIntel).

### The desktop app

A new app for Windows and Linux, written in Rust, that does everything the command-line tool does in a window.

- **Look up** a profile by username or link: name, badge, category, subscriber count, bio, website, creation and update dates, Snapcode (save as SVG or PNG), and related accounts.
- **Browse** the story, highlights, Spotlight videos and lenses as grids, and play them in a viewer with keyboard navigation.
- **Spotlight details:** views, comments, shares, boosts and recommends; date, length and sound; the video's language and category; the text shown on the video; Snapchat's AI title and summary; the phrases people search for; the top comments; and captions you can read, click through and copy.
- **Insights:** totals, upload times by weekday and hour, uploads per month, hashtags, categories and languages.
- **Downloads you can verify:** every file is recorded with its source, upload date and SHA-256. Repeating a download fetches only what is new, a changed file is never overwritten, and **Check files** re-hashes a folder against its record.
- **Export** a profile as JSON or CSV.
- **Compatibility check** that reports when Snapchat has changed the page SnapIntel reads.
- **Profiles that aren't public:** name, Snapcode, Bitmoji, and the Bitmoji's earlier looks.
- Light and dark themes, 10 page languages, right-to-left text, and a list of recent lookups you can turn off.
- Installers: `.exe` for Windows; `.deb`, `.rpm` and `.AppImage` for Linux.

### Privacy

- Location data on Snapchat's page (snap coordinates, a profile's address) is never read.
- Comments are read without their authors: commenter names, IDs and avatars are never read.
- No monitoring, no alerts, no account, no telemetry.

### The command-line tool

The original Python tool, now in `cli/`:

- Installs as the `snapintel` command (`pip install ./cli`).
- `--json` writes typed results for scripts.
- Downloads keep the same verifiable manifest as the app, resume by hash, and can be checked with `--verify`.
- `--self-check` reports when Snapchat's page has changed.
- Arabic and other right-to-left text displays correctly in terminals.
- Fixes: Spotlight downloads, accounts without a Bitmoji, and a crash when run from another folder.
- Offline tests on page snapshots, run on Linux and Windows for Python 3.10 to 3.14.
