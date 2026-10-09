#!/usr/bin/env python3
# What Orca said, from its debug log: one line per utterance, with the
# time; the Linux stand-in for NVDA's log viewer. Read from the strings
# Orca hands speech-dispatcher, which are exactly what is spoken; the
# speech generator's lines before them carry voice settings and are not
# quoted. Parts of one utterance (Orca speaks a change of voice or a pause
# as a new string) are joined with " | ".
#
#   orca --debug-file=orca.log          (see CLAUDE.md: Orca is a service)
#   python3 tools/linux/orca-speech.py orca.log

import re
import sys

UTTERANCE = re.compile(r"^([0-9:]+)\.\d+ - SPEECH: Speak ")
STRING = re.compile(r"^[0-9:.]+ - SPEECH DISPATCHER: Speaking '(.*)")
END = "' as string"


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: orca-speech.py <orca debug log>")
    time, parts, pending = None, [], None
    with open(sys.argv[1], encoding="utf-8", errors="replace") as log:
        for line in log:
            line = line.rstrip("\n")
            # A string with a line end in it goes on over indented lines.
            if pending is not None:
                pending += " " + line.strip()
                if line.endswith(END):
                    parts.append(pending[: -len(END)].strip())
                    pending = None
                continue
            start = UTTERANCE.match(line)
            if start:
                if parts:
                    print(time, " | ".join(parts))
                time, parts = start.group(1), []
                continue
            said = STRING.match(line)
            if said and time and not said.group(1).startswith("<speak>"):
                if line.endswith(END):
                    parts.append(said.group(1)[: -len(END)].strip())
                else:
                    pending = said.group(1)
    if parts:
        print(time, " | ".join(parts))


if __name__ == "__main__":
    main()
