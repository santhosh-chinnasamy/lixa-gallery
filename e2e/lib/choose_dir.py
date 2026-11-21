#!/usr/bin/env python3
"""Activate a GTK file chooser that is already open.

Sets the location entry to the path given on argv and invokes the Open button
through AT-SPI. Used because xdotool Return does not confirm this chooser on Xvfb.
"""

from __future__ import annotations

import sys
import time

import pyatspi


def walk(root):
    yield root
    try:
        count = root.childCount
    except Exception:
        return
    for i in range(count):
        try:
            child = root.getChildAtIndex(i)
        except Exception:
            continue
        yield from walk(child)


def find_dialog():
    desktop = pyatspi.Registry.getDesktop(0)
    for app in desktop:
        for node in walk(app):
            try:
                role = node.getRoleName()
                name = node.name or ""
            except Exception:
                continue
            if role == "dialog" and name in {"Select Folder", "Open"}:
                return node
    return None


def main() -> int:
    target = sys.argv[1]
    dialog = None
    for _ in range(40):
        dialog = find_dialog()
        if dialog is not None:
            break
        time.sleep(0.1)
    if dialog is None:
        print("NO_DIALOG")
        return 1
    entry = None
    button = None
    for node in walk(dialog):
        try:
            role = node.getRoleName()
            name = node.name or ""
        except Exception:
            continue
        if role == "text" and entry is None:
            entry = node
        if role == "push button" and name.lower() in {"open", "select"}:
            button = node
    if entry is not None:
        try:
            entry.setTextContents(target)
        except Exception:
            # Some entries only accept editable-text.
            try:
                entry.queryEditableText().setTextContents(target)
            except Exception as err:
                print("ENTRY", type(err).__name__)
    if button is None:
        print("NO_BUTTON")
        return 2
    try:
        button.doAction(0)
    except Exception as err:
        print("ACTION", type(err).__name__)
        return 3
    print("OPENED")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
