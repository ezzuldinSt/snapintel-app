# SnapIntel desktop

The source of the SnapIntel app for Windows and Linux. What the app does, how to install it and how to use it are on the [front page](../README.md) and in the [user guide](../docs/README.md).

| Folder | What it is |
|---|---|
| `core/` | `snapintel-core`, a library with no window: fetching and parsing Snapchat's page, the typed report, downloads with their manifest, exports, captions, and the compatibility check. |
| `src-tauri/` | The app: a [Tauri 2](https://tauri.app) window, the commands the interface may call, and settings. |
| `ui/` | The interface: plain HTML, CSS and JavaScript, built into the program. No framework and no build step. |
| `assets/` | The icon's source. |

```bash
cargo run                # build and start the app
cargo test --workspace   # the offline tests
```

[Development](../docs/development.md) covers the prerequisites, how the code is organised, the tests and building installers.
