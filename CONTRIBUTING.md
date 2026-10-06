# Contributing to Weatherspell

Thanks for helping! Weatherspell is small on purpose, so most of this is about
keeping it that way and keeping it accessible.

## Before you start

Open an issue (or comment on an existing one) before building anything
substantial, so the approach can be discussed and productively worked on.
Bug reports and feature ideas are welcome via the [issue templates](https://github.com/PlanetLinux98/weatherspell/issues/new/choose).

## Building and testing

You need [Rust](https://rustup.rs/) and Visual Studio 2022 (or its Build
Tools) with the "Desktop development with C++" workload. From a Developer
PowerShell for Visual Studio:

```
cargo build --release -p weatherspell
cargo test -p weatherspell-core --all-features
cargo test --release -p weatherspell -p wx-accessibility
```

Run the app from `target\release\weatherspell.exe`. A clean build alone
is not proof that it runs: launch it after any change to a window or
dialog. The first build compiles wxWidgets and takes several minutes;
later ones are quick.

The C# app in `src\` is Weatherspell 0.1, which gets fixes only; it
builds with the .NET SDK (`dotnet build Weatherspell.slnx`).

## Ground rules

- **One exe.** The shipped program is a single `Weatherspell.exe` with no
  files beside it; how that is achieved may change over time, the single file
  will not. That rules out DLLs or other files beside the exe and anything
  that needs to be installed: wxWidgets and the C runtime are linked into
  it, and `tools\Check-SingleExe.ps1` checks the result. Build-time-only
  tools and crates are fine.
- **Accessible name = visible text.** A button, menu item or checkbox is
  announced by the text it shows. Inputs with no text of their own (a text
  box, list or combo box) are named by the visible label just before them,
  so each one gets a label placed directly before it. Rarely should you
  give a control a reworded or more verbose name than what is on screen;
  extra detail typically belongs in help text. The accessibility lint
  (`wx_accessibility::lint`, run by the app's tests in CI) enforces the
  mechanical parts of this.
- **Keyboard first.** Every action reachable with the keyboard, an explicit
  tab order, mnemonics on menus and buttons where they do not collide.
- **Logic in `weatherspell-core`, not in windows.** Anything worth
  testing (parsing, writing the forecast, alerts, settings) lives in the
  core crate, which has no UI and is tested on Windows, macOS and Linux.
- **Comments explain why, not what.** Keep them short; please keep non-obvious
  reasons or rejected alternatives.

## Testing UI changes

If you changed anything a user can see or focus, please consider running it through
use with assistive technology: ideally at least one screen reader (NVDA is a free option) or
screen magnifier (one is built into Windows) and say in the PR what you experienced.
`tools\Dump-A11y.ps1 -ProcessId <id>` prints what the running window
exposes to assistive technology, and `uia-dump` (from the
`wx-accessibility` crate) what Narrator sees; both are good first checks
before reaching for a screen reader.

## Branches, commits and pull requests

- Branch from `main` and name the branch by type: `feature/`, `fix/`,
  `chore/` or `docs/` plus a short name.
- One change or small related changes per pull request.
Describe what changed and why; link the issue it addresses (`Fixes #12`).
- Add a line under `[Unreleased]` in `CHANGELOG.md` for anything a user would
  notice. Fixes cite the issue number in parentheses.
- CI must be green.

## Licence

By contributing you agree that your contributions are licensed under the
project's [MIT licence](LICENSE).
