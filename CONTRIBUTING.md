# Contributing

Thanks for wanting to help. Bug reports, fixes and improvements are welcome.

## Reporting a problem

[Open an issue](https://github.com/ezzuldinSt/snapintel-app/issues) with:

- your system and the SnapIntel version (bottom of Settings, or `snapintel --version` for the command-line tool),
- what you did and what happened,
- for missing details, the result of the compatibility check (Settings → **Check now**, or `snapintel --self-check`).

Please leave other people's personal details out of issues. If a problem only shows on one profile, describe the kind of profile rather than naming a private person.

## Changing the code

[Development](docs/development.md) explains how to build, test and find your way around. Before you send a pull request:

```bash
cd desktop
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
node --check ui/app.js

cd ../cli
python -m unittest discover -s tests
```

A few things keep the project easy to maintain:

- **Snapchat's page layout lives in one place:** `desktop/core/src/paths.rs` for the app and `cli/snapintel/config.json` for the command-line tool. A field that isn't in those tables isn't read.
- **New behaviour comes with a test** that runs offline on the snapshots in `tests/fixtures`.
- **The interface has no framework and no build step.** Keep it that way.

## Privacy boundaries

SnapIntel shows what a profile's owner published, and stops there. It deliberately does not:

- read location data (snap coordinates, profile addresses),
- read who wrote a comment,
- monitor profiles over time or alert when an account posts,
- sign in to Snapchat or get around a profile's privacy settings.

The first two are enforced by tests, which need to keep passing. If you have an idea that touches any of these, please open an issue to discuss it before writing code. [Privacy and limits](docs/privacy.md) has the details.

## Licence

By contributing you agree that your work is licensed under the [AGPL-3.0](LICENSE), like the rest of the project.
