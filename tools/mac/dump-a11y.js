// What VoiceOver is told about Weatherspell's windows and menus: each
// element's role, name and value, indented as they nest, the stand-in on
// the Mac for tools\Dump-A11y.ps1. Run in Terminal while the app is open:
//
//   osascript -l JavaScript dump-a11y.js | pbcopy
//
// and the dump is on the clipboard. "menus" after the file name adds the
// menu bar; another name after it dumps that app instead (an Apple app, to
// compare). Terminal needs Accessibility permission (System Settings >
// Privacy & Security > Accessibility), and asks once to control System
// Events.

const BUNDLE_ID = "io.github.planetlinux98.weatherspell";
// A long text (the forecast) is cut to this many characters.
const VALUE_LIMIT = 300;

function run(argv) {
    const events = Application("System Events");
    const menus = argv.includes("menus");
    const named = argv.filter((a) => a !== "menus")[0];
    const matches = named
        ? events.processes.whose({ name: named })
        : events.processes.whose({ bundleIdentifier: BUNDLE_ID });
    if (matches.length === 0) {
        return `${named || "Weatherspell"} is not running.`;
    }
    const app = matches[0];
    const lines = [];

    const focused = attribute(app, "AXFocusedUIElement");
    lines.push(`Focused: ${focused ? describe(focused) : "nothing"}`);
    lines.push("");

    for (const window of app.windows()) {
        dump(window, 0, lines);
        lines.push("");
    }
    if (menus) {
        for (const bar of app.menuBars()) {
            dump(bar, 0, lines);
        }
    }
    return lines.join("\n");
}

function dump(element, depth, lines) {
    lines.push("  ".repeat(depth) + describe(element));
    for (const child of safe(() => element.uiElements(), [])) {
        dump(child, depth + 1, lines);
    }
}

// "AXButton (button) title "Refresh" [focused]": the role, VoiceOver's
// word for it, then whatever names it, its value, and its state. Read as
// the raw AX attributes VoiceOver reads; System Events' own property
// names differ from them (its "description" is the role's).
function describe(element) {
    const a = (name) => attribute(element, name);
    const role = a("AXRole") || "?";
    const subrole = a("AXSubrole");
    const parts = [subrole ? `${role}/${subrole}` : role];
    const roleDescription = a("AXRoleDescription");
    if (roleDescription) parts.push(`(${roleDescription})`);
    for (const [name, label] of [
        ["AXTitle", "title"],
        ["AXDescription", "description"],
        ["AXPlaceholderValue", "placeholder"],
        ["AXHelp", "help"],
    ]) {
        const v = a(name);
        if (v) parts.push(`${label} "${text(v)}"`);
    }
    // A label control that names this one, as VoiceOver reads it.
    const label = a("AXTitleUIElement");
    if (label) {
        const named = attribute(label, "AXValue") || attribute(label, "AXTitle");
        parts.push(`labelled by "${text(named)}"`);
    }
    const value = text(a("AXValue"));
    if (value !== "") parts.push(`value "${value}"`);
    const shortcut = menuShortcut(element, role);
    if (shortcut) parts.push(`shortcut ${shortcut}`);
    if (a("AXFocused")) parts.push("[focused]");
    if (a("AXSelected")) parts.push("[selected]");
    if (a("AXEnabled") === false) parts.push("[disabled]");
    return parts.join(" ");
}

function text(value) {
    if (value === undefined || value === null) return "";
    let s = String(value).replace(/\r?\n/g, "\\n");
    if (s.length > VALUE_LIMIT) {
        s = `${s.slice(0, VALUE_LIMIT)}... (${s.length} characters)`;
    }
    return s;
}

// A menu item's key, as the menu shows it: "Cmd+Shift+L".
function menuShortcut(element, role) {
    if (role !== "AXMenuItem") return "";
    const key = attribute(element, "AXMenuItemCmdChar");
    const glyph = attribute(element, "AXMenuItemCmdGlyph");
    if (!key && !glyph) return "";
    // Bits: 1 Shift, 2 Option, 4 Control, 8 no Command.
    const mods = attribute(element, "AXMenuItemCmdModifiers") || 0;
    const names = [];
    if (mods & 4) names.push("Ctrl");
    if (mods & 2) names.push("Option");
    if (mods & 1) names.push("Shift");
    if (!(mods & 8)) names.unshift("Cmd");
    names.push(key || `glyph ${glyph}`);
    return names.join("+");
}

function attribute(element, name) {
    return safe(() => element.attributes.byName(name).value(), null);
}

// System Events throws for an attribute an element lacks.
function safe(get, fallback) {
    try {
        const v = get();
        return v === undefined ? fallback : v;
    } catch (e) {
        return fallback;
    }
}
