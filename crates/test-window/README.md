# Test window

A throwaway window that checks what wxWidgets, through the wxDragon
bindings, gives screen readers before any of Weatherspell is ported to it
(step 1 of #24). It looks like Weatherspell's main window, with sample
forecasts and nothing fetched, and its checks are the problems the C# app's
testing found (NOTES.md, "The C# app (0.1)").

Build and run it from the repository root (the first build compiles
wxWidgets and takes several minutes; CMake and Ninja must be on PATH):

    cargo build --release -p test-window

The window is then `target/release/test-window.exe`. Started with
`--edit`, it uses a plain edit control for the forecast instead of the
rich edit control; with `--advertise`, the window advertises its UI
Automation provider (see check 5).

Two small tools come with it: `target/release/uia-listen.exe` prints every
UI Automation notification raised on the desktop for a minute, with the
window it came from, and `target/release/uia-dump.exe` prints what a
native UI Automation client such as Narrator sees in the window, including
which provider answered for each control.

## What to try

1. **Menu bar.** Alt opens it; each menu's letter works; items read with
   their shortcuts, such as "Refresh F5" and "Next Section Ctrl+Page
   Down" (wxWidgets writes the key name with a space).
2. **Location box.** Arrowing through it while it is closed reads each
   place, and the forecast for it follows with "forecast ready".
3. **Forecast text.** Reading by line, word and character with NVDA and
   Narrator; moving the mouse over it reads a line or paragraph, not the
   whole text; Shift+F10 or the Applications key opens a context menu,
   and after closing it Ctrl+A and Ctrl+C still work.
4. **Section keys.** Ctrl+PageDown and Ctrl+PageUp move between headings
   and say each one; Ctrl+Shift+A goes to Alerts. F5 rewrites the text
   (the first time in Peterborough, a frost advisory appears above
   everything) and the caret stays on the same words. Test > Refresh in 5
   Seconds does the same while you read.
5. **Announcements.** Test > Announce (Ctrl+Shift+N) and Announce in 5
   Seconds speak a line without moving focus. Test > Advertise UI
   Automation Provider changes how the window presents itself to UI
   Automation; the question is whether that changes anything else NVDA
   does with the window, such as the Location box.
6. **Labels.** The Location box and the forecast are named by the labels
   before them; in Locations > Rename Location, so is the nickname field.
7. **Fonts.** Test > Font and Scale says which font the window uses and
   what Windows' own message font is, which follows the Text size setting.
8. **Dialogs.** Help > About is read on opening; Test > About as a
   Message Box is Windows' own, to compare. In Rename Location, Enter is
   OK and Escape is Cancel.
9. **One exe.** The release build is a single exe that needs nothing
   beside it.

## What the tools found (2026-10-04)

Before any screen reader: Dump-A11y for MSAA, `uia-dump` for UI
Automation, `uia-listen` for the announcements, and keys sent to the
window, with the caret's line read back through UI Automation.

- The menu bar is the native Windows one (not owner drawn), with each
  menu's Alt letter; item text keeps the shortcut after a tab, which is
  what screen readers read as the shortcut.
- Every control is the Windows control itself, answered by Windows' own
  accessibility objects (with the identity NVDA's mouse reading needs):
  the Location box is a ComboBox, the forecast a RICHEDIT50W (wx's choice
  for a rich text control) whose own UI Automation provider has the text
  pattern Narrator reads by. The plain edit control gets a text pattern
  from Windows too. wxWidgets has no UI Automation code of its own, so
  nothing stands between the controls and the screen reader.
- The labels name the Location box (with Alt+O), the forecast and the
  nickname field.
- The section keys work from the text and from the Location box, and F5
  left the caret on the same line while an alert was added above it.
  Shift+F10, Escape, Ctrl+A and Ctrl+C selected and copied the whole text,
  but that was with sent keys, not a keyboard.
- Every announcement arrived: forecast ready, the section headings, "No
  previous section", and the new alert as an important one. They come
  from the main window, which does not answer UI Automation itself unless
  told to advertise.
- A wx dialog is a real Windows dialog (class #32770), so MSAA and UI
  Automation already call it a dialog and its text is readable. The C#
  About had to set that role; here, wxDragon's role setter replaces the
  dialog's accessible object and its children lose their names, so it
  must not be used on a window with children.
- The window uses Segoe UI 9 point, the Windows message font, at this
  computer's 150 percent.
- The release exe is 7.3 MB, imports only Windows' own DLLs, and runs
  from a folder with nothing else in it. Its window appeared in about 70
  milliseconds (the first run after the build took nearly a second,
  probably the virus scanner's first look at a new file).
