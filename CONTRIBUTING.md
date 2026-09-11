# Contributing to Weatherspell

Thanks for helping. Weatherspell is small on purpose, so most of this is about
keeping it that way and keeping it accessible.

## Before you start

Open an issue (or comment on an existing one) before building anything
substantial, so the approach can be agreed first. Bug reports and feature ideas
are welcome via the [issue templates](https://github.com/PlanetLinux98/weatherspell/issues/new/choose).

## Building and testing

You need the [.NET SDK](https://dotnet.microsoft.com/download) (10.0 or later)
on Windows; nothing else.

```bash
dotnet build Weatherspell.slnx
dotnet test Weatherspell.slnx
```

Run the app from `src\Weatherspell\bin\Debug\net48\Weatherspell.exe`. A clean
build is not proof that it runs: launch it after any change to a form.

## Ground rules

- **One exe.** The shipped program is a single `Weatherspell.exe` with no
  files beside it. That rules out runtime NuGet dependencies, `.exe.config`
  settings, and anything that needs to be installed. Build-time-only packages
  (analyzers, source generators) are fine.
- **Accessible name = visible text.** A button, menu item or checkbox is
  announced by the text it shows. Inputs with no text of their own take their
  visible label's text as `AccessibleName`, verbatim. Never give a control a
  reworded or more verbose name than what is on screen; extra detail belongs
  in a tooltip or help text. The `AccessibilityLint` test enforces the
  mechanical parts of this and runs in CI.
- **Keyboard first.** Every action reachable with the keyboard, an explicit
  tab order, mnemonics on menus and buttons where they do not collide.
- **Logic in plain classes, not in forms.** Anything worth testing (parsing,
  formatting, alerts, settings) lives in a class the test project can reach.
- **Comments explain why, not what.** Keep them short; keep every non-obvious
  reason or rejected alternative.

## Testing UI changes

If you changed anything a user can see or focus, run it through at least one
screen reader (NVDA is free) and say in the PR what you used and what you
heard. `tools\Dump-A11y.ps1` prints what the running window exposes to
assistive technology and is a good first check before reaching for NVDA.

## Branches, commits and pull requests

- Branch from `main` and name the branch by type: `feature/`, `fix/`,
  `chore/` or `docs/` plus a short name.
- One change per pull request. Describe what changed and why; link the issue
  it addresses (`Fixes #12`).
- Add a line under `[Unreleased]` in `CHANGELOG.md` for anything a user would
  notice. Fixes cite the issue number in parentheses.
- CI must be green.

## Licence

By contributing you agree that your contributions are licensed under the
project's [MIT licence](LICENSE).
