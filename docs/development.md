# Development

[← All guides](README.md)

## Build and run

You need [Rust](https://rustup.rs) 1.77 or newer.

On Linux, also install the WebKitGTK development packages:

```bash
sudo apt install libwebkit2gtk-4.1-dev librsvg2-dev libsoup-3.0-dev libxdo-dev libayatana-appindicator3-dev build-essential
```

Then:

```bash
cd desktop
cargo run                                              # build and start the app
cargo test --workspace                                 # the offline tests
cargo fmt --all --check                                # formatting
cargo clippy --workspace --all-targets -- -D warnings  # lints
node --check ui/app.js                                 # the interface script parses
```

The interface in `desktop/ui` is compiled into the program. There is no dev server and no Node.js build step; after editing a file there, `cargo run` again.

## How the code is organised

```
desktop/
  core/        snapintel-core: a library with no window
    paths.rs       the one table of where things are in Snapchat's page
    parse.rs       turns the page into a typed report (model.rs)
    fetch.rs       requests to snapchat.com
    downloads.rs   downloads and the manifest
    export.rs      JSON and CSV
    selfcheck.rs   the compatibility check
    transcript.rs  captions; bitmoji.rs earlier Bitmoji versions
    tests/         offline tests
  src-tauri/   the app: the Tauri window, commands, settings
  ui/          index.html, styles.css, app.js: no framework
cli/           the Python command-line tool (snapintel/) and its tests
tests/fixtures/  page snapshots shared by both test suites
docs/          these guides
```

**One table describes Snapchat's page.** `desktop/core/src/paths.rs` is the only place the layout of the page is written down. Parsing reads through it and the compatibility check tests it against the live site. When Snapchat moves something, the check names the field, and that table is what needs to change. The Python tool has the same arrangement in `cli/snapintel/config.json`.

**The window can do very little.** The interface may call the app's own commands and nothing else (`desktop/src-tauri/capabilities/default.json`). Text from Snapchat is only ever put into the window as text, never as HTML. Only `https` addresses are used for media and links. The app opens only folders and files it saved itself.

**Privacy is enforced by tests.** `location_is_never_read` and `spotlight_extras_and_comments_without_commenters` in `desktop/core/tests/offline.rs` put location data and a marked commenter into a page and fail if any of it reaches a report or an export. Please keep them passing; see [Privacy and limits](privacy.md).

## Tests

Both test suites run offline on the page snapshots in `tests/fixtures`: two of Snap's own profiles, reduced to what SnapIntel reads, with stand-in text in place of real comments.

```bash
cd desktop && cargo test --workspace             # the app
cd cli && python -m unittest discover -s tests   # the command-line tool
```

To refresh the snapshots from snapchat.com, then see what a change on Snapchat's side broke:

```bash
python tests/fixtures/update_fixtures.py
```

## Looking at the running app

On Windows, start it with WebView2's debugging port open and attach any Chrome DevTools client:

```powershell
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9223'
cargo run
```

Then open `http://127.0.0.1:9223` in a Chromium browser. The page's functions (`lookup`, `showTab`, `state`) are globals, which makes it easy to script.

## Installers

```bash
cargo install tauri-cli --version "^2" --locked
cd desktop
cargo tauri build                                     # every installer for this system
cargo tauri build --bundles deb,rpm,appimage          # or pick
```

They land in `desktop/target/release/bundle`. Windows produces an NSIS installer; Linux produces `.deb`, `.rpm` and `.AppImage`. Build Linux packages on Linux (WSL works), in a folder on the Linux file system.

## Continuous integration

| Workflow | What it does |
|---|---|
| `desktop.yml` | Formatting, lints and tests for the app on Linux and Windows. For a version tag, or on demand, it also builds the installers as artifacts. |
| `cli.yml` | The command-line tool's tests on Linux and Windows, Python 3.10 to 3.14. |
| `self-check.yml` | Every Monday, checks snapchat.com against what SnapIntel reads. |

## Making a release

1. Set the version in `desktop/Cargo.toml`, `desktop/src-tauri/tauri.conf.json` and `cli/snapintel/__init__.py`, and add it to `CHANGELOG.md`.
2. Run the tests, then build the installers on Windows and Linux.
3. Write `SHA256SUMS.txt` for the installers.
4. Tag the commit (`v1.0.0`) and publish a GitHub release with the installers and the checksums attached.

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md).
