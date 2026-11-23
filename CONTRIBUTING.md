# Contributing to Seamless Download Manager

Thanks for helping. Bug reports, ideas and pull requests are all welcome.

## Reporting a bug

[Open an issue](https://github.com/DavidHiFi/seamless-download-manager/issues/new) and include:

- what you did, what you expected and what happened instead
- your Windows version, your browser, and the app version from **About**
- the link that failed, if you can share it
- a screenshot of the download's details or the error

For the Windhawk mod, say so, and add your Windhawk version.

## Suggesting a feature

Open an issue that describes what you want to do and why. A screenshot or a mock-up helps.

## Sending a pull request

1. Fork the repo and create a branch from `main`.
2. Install Rust 1.85 or newer. For the extension icons you also need Python with Pillow and resvg_py.
3. Before you push, run these and make sure they pass:
   ```powershell
   cargo clippy --workspace --all-targets
   cargo test --release --workspace
   node --test extension/tests
   ```
4. Keep each pull request to one change, and say in the description how you tested it.

The window is Slint (`app/ui`) and follows the Catppuccin Mocha colours in `app/ui/theme.slint`. Use those colours, never new ones. The engine library's commands are listed in `docs/engine-library.md`; change that file when you change them, because the Windhawk mod depends on it.

## License

By contributing, you agree that your work is released under this repo's [Apache-2.0 license](LICENSE).
