# Privacy and limits

[← All guides](README.md)

SnapIntel shows what a profile's owner chose to publish on snapchat.com. This page says exactly what it reads, what it deliberately does not, and what it cannot show.

## What SnapIntel reads

- **The profile's public page** on snapchat.com: the same page anyone gets in a browser without signing in.
- **The media that page links to** (photos, videos, previews, the Snapcode, a Bitmoji), from Snapchat's own servers, when you view or download them.
- **Captions** for a video, when you press **Show captions**.
- **Earlier versions of a Bitmoji**, when you press **Find earlier looks**.

It needs no Snapchat account and never asks you to sign in. It connects only to Snapchat's servers: there are no analytics, no crash reports, no update checks and no third parties.

## What stays on your computer

| What | Where | How to remove it |
|---|---|---|
| Settings | `settings.json` (see [Settings](settings.md#where-settings-are-stored)) | Delete the file |
| The profiles you opened recently | In the same file | **Clear** on the start screen, or turn off **Remember recent lookups** |
| Downloads and their record | The folder you chose | Delete the files |

Nothing else is kept. Profiles are not cached: closing one forgets it.

## What SnapIntel never reads

These are on Snapchat's page, and SnapIntel skips them on purpose. They are not shown, not stored and not in any export, and the project's tests fail if that ever changes.

- **Location.** The coordinates attached to snaps, and a profile's address.
- **Who commented.** Comments are read as words, a date and two counts. The commenter's name, ID and avatar are never read.

## What SnapIntel does not do

- **It does not monitor.** There is no watching a profile over time, no schedule, and no alert when an account posts. Every lookup is one you start yourself.
- **It does not get around privacy settings.** A profile that isn't public shows what Snapchat shows of it: a name, a Snapcode and, at most, a Bitmoji.
- **It does not sign in as you** or use anyone's account.

## What Snapchat doesn't publish

Some things can't be shown, because Snapchat does not give them to visitors who aren't signed in.

- **More than the first page of each list.** A profile's page carries its newest highlights, Spotlights and lenses. The rest load only inside Snapchat. SnapIntel marks a list that has more with a `+` (for example **16+**) and adds a note with a link to the profile on Snapchat.
- **The Snap Map.** Snapchat closed the public map on the web. There is no map to explore, and SnapIntel does not offer one.
- **All of a video's comments.** The page includes the top few.
- **Some figures.** Snapchat hides view, share or subscriber counts on some profiles; SnapIntel shows a dash or leaves the figure out.
- **Stories older than 24 hours**, unless the profile saved them to a highlight.

## Using it responsibly

What a profile publishes is public, and the people behind profiles are still people. Use what you find lawfully and fairly: respect copyright in the media you save, follow the rules that apply to your work if you use it for research or journalism, and don't use it to harass anyone.

SnapIntel is an independent project and is not affiliated with Snap Inc.
