# SnapIntel desktop

A desktop app for Windows and Linux that shows what a public Snapchat profile publishes on snapchat.com and saves its media. It is SnapIntel rewritten in Rust with a window, so nothing needs to be typed into a terminal.

Type a username (or paste a profile link) and you get:

- **The profile:** name, badge, category, subscriber count, bio, website, when the account was created and last updated, its Snapcode, and the accounts Snapchat lists as related.
- **Story, highlights, Spotlight and lenses** as grids you can open, play and step through with the arrow keys.
- **Spotlight details:** views, comments, shares, boosts and recommends, length, hashtags, the sound used, the video's language and category, the text shown on the video, Snapchat's own AI title and summary, the phrases it expects people to search for, and captions when Snapchat has transcribed the video.
- **Top comments** under each Spotlight: the words, the date, and how many reactions and replies each has. Who wrote a comment is never read.
- **Insights:** totals, upload times by weekday and hour, uploads per month, the hashtags used most, and what the videos are about and which languages they are in.
- **Downloads you can check later:** every file is recorded with its source, date and SHA-256 in `snapintel-manifest.json`; a repeated download skips what is already saved, and "Check files" re-hashes a folder. The format is the Python tool's, so either can continue a folder the other started.
- **Export** of a profile as JSON (everything) or CSV (one row per item).
- **A compatibility check** (Settings) that tells you whether Snapchat has changed the page SnapIntel reads.

It reads only what the profile's page on snapchat.com carries. Location data on that page (snap coordinates, a profile's address) and the names, IDs and avatars of people who commented are deliberately not read, stored or exported, and the app does not watch profiles or send alerts.

What it can't show, because Snapchat doesn't publish it to visitors who aren't signed in:

- **More than the first page of each list.** A profile's page carries its newest highlights, Spotlights and lenses (the app marks a list that has more with a `+`); the rest loads only inside Snapchat.
- **The Snap Map.** Snapchat closed the public map on the web, so there is no map to explore or download from.

## Install

Installers are built by the `desktop` workflow on GitHub: an `.exe` installer for Windows, and `.deb`, `.rpm` and `.AppImage` for Linux.

- **Windows:** needs the WebView2 runtime, which Windows 10 and 11 already have.
- **Linux:** needs WebKitGTK 4.1 (the `.deb` and `.rpm` pull it in). Video uses GStreamer; if videos don't play, install `gstreamer1.0-libav` and `gstreamer1.0-plugins-good` (Debian/Ubuntu names).

`snapintel <username>` opens the app straight onto a profile.

## Build it yourself

You need [Rust](https://rustup.rs) 1.77 or newer. On Linux also install the WebKitGTK development packages:

```bash
sudo apt install libwebkit2gtk-4.1-dev librsvg2-dev libsoup-3.0-dev libxdo-dev libayatana-appindicator3-dev build-essential
```

Then, in this folder:

```bash
cargo run              # build and start the app
cargo test --workspace # the offline tests
```

The interface in `ui/` is built into the program, so there is no dev server and no Node.js step. To produce installers, use the Tauri CLI:

```bash
cargo install tauri-cli --version "^2" --locked
cargo tauri build      # installers land in target/release/bundle/
```

## How it is put together

| Folder | What it is |
|---|---|
| `core/` | `snapintel-core`, a library with no window: fetching and parsing the page, the typed report, downloads with their manifest, exports, captions, and the compatibility check. |
| `src-tauri/` | The app: a [Tauri](https://tauri.app) window, the commands the interface may call, and settings. |
| `ui/` | The interface: plain HTML, CSS and JavaScript, no framework and no build step. |

`core/src/paths.rs` is the only place the layout of Snapchat's page is written down. Parsing reads through it and the compatibility check tests it against the live site, so when Snapchat moves something, the check names the field and that one table is what needs changing.

The tests run offline on the page snapshots in `../tests/fixtures` (shared with the Python tool) and on a small local web server: parsing both profile layouts, exports, comments arriving without their authors, the compatibility check catching a renamed field, and the whole download cycle (manifest, upload-time file dates, resuming, altered and deleted files, cancelling).

Security notes: text from Snapchat is only ever put into the window as text, never as HTML; only `https` addresses are used for media and links; the window can call the app's own commands and nothing else; and the app only opens folders and files it saved itself.

## Licence

AGPL-3.0, like the original [SnapIntel](https://github.com/Kr0wZ/SnapIntel) by Kr0wZ, which this is based on.
