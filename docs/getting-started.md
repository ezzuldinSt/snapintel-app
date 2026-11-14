# Getting started

[← All guides](README.md)

## 1. Install

Download the file for your system from the [latest release](https://github.com/ezzuldinSt/snapintel-app/releases/latest).

### Windows 10 and 11

1. Run `SnapIntel_1.0.0_x64-setup.exe`.
2. Windows SmartScreen warns about an unrecognised app, because the installer is not code-signed. Choose **More info**, then **Run anyway**.
3. Follow the installer. It installs for your user account only, so it never asks for administrator rights.

SnapIntel appears in the Start menu and on the desktop. It uses the WebView2 runtime that Windows 10 and 11 already include; if yours is missing, the installer downloads it.

### Debian, Ubuntu, Linux Mint

```bash
sudo apt install ./SnapIntel_1.0.0_amd64.deb
```

This also installs what the app needs: WebKitGTK 4.1, and the GStreamer plugins that play Snapchat's videos.

### Fedora and openSUSE

```bash
sudo dnf install ./SnapIntel-1.0.0-1.x86_64.rpm
```

If videos don't play, install the GStreamer plugins for H.264 and AAC (on Fedora they come from RPM Fusion: `gstreamer1-libav` and `gstreamer1-plugins-good`).

### Any other Linux: the AppImage

```bash
chmod +x SnapIntel_1.0.0_amd64.AppImage
./SnapIntel_1.0.0_amd64.AppImage
```

The AppImage carries its own copy of WebKitGTK, so the app starts on a system that has none. Videos still use the GStreamer plugins installed on your system.

### Check your download (optional)

Every release includes `SHA256SUMS.txt`. Compare it with the hash of the file you downloaded:

```powershell
Get-FileHash .\SnapIntel_1.0.0_x64-setup.exe -Algorithm SHA256
```

```bash
sha256sum SnapIntel_1.0.0_amd64.deb
```

## 2. Look up a profile

<img src="media/walkthrough-lookup.gif" width="820" alt="Typing a username on the start screen, then moving through the profile's tabs.">

1. Open SnapIntel.
2. Type a username, or paste a profile link, into the search box.
3. Press <kbd>Enter</kbd>.

The profile opens on its **Overview** tab. The other tabs hold its story, highlights, Spotlight videos and lenses; the number beside each tab is how many items it has. [Reading a profile](profiles.md) explains each one.

If Snapchat has no such username, or the profile can't be loaded, the app says which it is and what you can do about it.

## 3. Save something

- **Everything:** press **Download all** on the profile.
- **One tab:** each tab has its own **Download all**.
- **A few items:** tick the circle in the corner of each one, then press **Download** in the bar that appears.

Files go to `Downloads/SnapIntel/<username>` unless you choose another folder in [Settings](settings.md). [Downloading and verifying](downloads.md) explains the record kept beside them.

## Opening straight onto a profile

Start the app with a username and it opens that profile:

```bash
snapintel nasa
```

On Windows the program is `%LOCALAPPDATA%\SnapIntel\snapintel.exe`.

## Updating

Install the new version over the old one. Your settings, recent lookups and downloads are kept.

## Uninstalling

- **Windows:** Settings → Apps → Installed apps → SnapIntel → Uninstall.
- **Debian, Ubuntu:** `sudo apt remove snap-intel`
- **Fedora, openSUSE:** `sudo dnf remove snap-intel`
- **AppImage:** delete the file.

Your downloads are never removed. Settings live in `%APPDATA%\app.snapintel.desktop` on Windows and `~/.config/app.snapintel.desktop` on Linux; delete that folder to forget them.
