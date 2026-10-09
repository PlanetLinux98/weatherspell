#!/usr/bin/env python3
# What Orca is told about Weatherspell's windows: each accessible's role,
# name, description, states, the label that names it and, for text, what
# it holds, indented as they nest; the stand-in on Linux for
# tools\Dump-A11y.ps1 and tools/mac/dump-a11y.js. Run while the app is
# open (any session user can, nothing to allow):
#
#   python3 tools/linux/dump-a11y.py [--app NAME | --pid PID] [--all]
#
# The app is found by its AT-SPI name, "weatherspell" unless --app names
# another (a GNOME app, to compare). Hidden accessibles (no "showing"
# state: wx keeps the other pages of a window around) are left out unless
# --all is given. A long text (the forecast) is cut to TEXT_LIMIT.

import argparse
import sys

import pyatspi

TEXT_LIMIT = 300

# States worth reading; the rest (enabled, sensitive, visible...) are noise
# on nearly every accessible, so only their absence is told.
STATES = [
    "focused", "focusable", "selected", "checked", "pressed", "expanded",
    "collapsed", "editable", "multi_line", "single_line", "read_only",
    "modal", "active", "is_default", "required", "indeterminate",
]
EXPECTED = ["enabled", "sensitive", "visible", "showing"]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--app", default="weatherspell")
    parser.add_argument("--pid", type=int)
    parser.add_argument("--all", action="store_true")
    args = parser.parse_args()

    app = find_app(args.app, args.pid)
    if app is None:
        sys.exit(f"{args.app if args.pid is None else args.pid} is not "
                 "running, or not on the accessibility bus.")
    lines = [f"App: {app.name} (pid {app.get_process_id()})"]
    focused = find_focused(app)
    lines.append(f"Focused: {describe(focused) if focused else 'nothing'}")
    lines.append("")
    for window in children(app):
        dump(window, 0, lines, args.all)
        lines.append("")
    print("\n".join(lines))


def find_app(name, pid):
    for app in children(pyatspi.Registry.getDesktop(0)):
        try:
            if pid is not None and app.get_process_id() == pid:
                return app
            if pid is None and (app.name or "").lower() == name.lower():
                return app
        except Exception:
            continue
    return None


def children(accessible):
    try:
        return [c for c in accessible if c is not None]
    except Exception:
        return []


def find_focused(app):
    def walk(accessible):
        try:
            if accessible.getState().contains(pyatspi.STATE_FOCUSED):
                return accessible
        except Exception:
            return None
        for child in children(accessible):
            found = walk(child)
            if found:
                return found
        return None
    return walk(app)


def dump(accessible, depth, lines, everything):
    try:
        showing = accessible.getState().contains(pyatspi.STATE_SHOWING)
    except Exception:
        showing = False
    if not showing and not everything and depth > 0:
        return
    lines.append("  " * depth + describe(accessible))
    for child in children(accessible):
        dump(child, depth + 1, lines, everything)


# 'push button "Refresh" [focusable, default] labelled by: ...': the role
# as Orca names it, the name Orca reads, then description, states,
# relations, text and actions.
def describe(accessible):
    parts = []
    try:
        parts.append(accessible.getRoleName())
    except Exception:
        return "(gone)"
    name = accessible.name
    parts.append(f'"{name}"' if name else "(no name)")
    description = accessible.description
    if description:
        parts.append(f'description "{description}"')

    states = accessible.getState()
    shown = [s for s in STATES if states.contains(state(s))]
    missing = [s for s in EXPECTED if not states.contains(state(s))]
    marks = shown + [f"not {s}" for s in missing]
    if marks:
        parts.append("[" + ", ".join(marks) + "]")

    for relation in safe(accessible.getRelationSet, []):
        kind = pyatspi.relationToString(relation.getRelationType())
        targets = [relation.getTarget(i) for i in range(relation.getNTargets())]
        named = ", ".join(f"{t.getRoleName()} \"{t.name}\"" for t in targets if t)
        parts.append(f"{kind.lower().replace('_', ' ')}: {named}")

    value = safe(lambda: accessible.queryValue().currentValue, None)
    if value is not None:
        parts.append(f"value {value:g}")

    text = safe(accessible.queryText, None)
    if text is not None and text.characterCount and not name_is_text(accessible, text):
        content = text.getText(0, -1)
        cut = content[:TEXT_LIMIT].replace("\n", "\\n")
        more = f"... ({len(content)} characters)" if len(content) > TEXT_LIMIT else ""
        parts.append(f'text "{cut}{more}" caret {text.caretOffset}')
        selections = [text.getSelection(i) for i in range(text.getNSelections())]
        if selections:
            parts.append("selected " + ", ".join(f"{s}-{e}" for s, e in selections))

    action = safe(accessible.queryAction, None)
    if action is not None and action.nActions:
        names = [action.getName(i) for i in range(action.nActions)]
        keys = [action.getKeyBinding(i) for i in range(action.nActions)]
        acts = [f"{n} ({k})" if k else n for n, k in zip(names, keys)]
        parts.append("actions: " + ", ".join(acts))
    return " ".join(parts)


# A label's or button's text is its name; repeating it as text is noise.
def name_is_text(accessible, text):
    return text.getText(0, -1) == accessible.name


def state(name):
    return getattr(pyatspi, "STATE_" + name.upper())


def safe(call, fallback):
    try:
        return call()
    except Exception:
        return fallback


if __name__ == "__main__":
    main()
