"""Select a GNOME Tweaks sidebar page through its accessibility interface.

Tweaks 46 has no CLI or D-Bus action for selecting pages. This helper only
selects a sidebar row in the process owning org.gnome.tweaks; it never changes
preferences, sends keyboard input, or modifies the installed application.
"""

import gettext
import signal
import sys
import time
from collections import deque

PAGES = {"appearance": "Appearance", "fonts": "Fonts", "windows": "Windows"}


def translated_pages(locale_name):
    catalog = gettext.translation(
        "gnome-tweaks", languages=[locale_name] if locale_name else None, fallback=True
    )
    return {key: catalog.gettext(value) for key, value in PAGES.items()}


def descendants(root, limit=600):
    pending = deque([root])
    for _ in range(limit):
        if not pending:
            break
        node = pending.popleft()
        yield node
        pending.extend(node.get_child_at_index(i) for i in range(node.get_child_count()))


def select_sidebar_page(application, page, titles, roles):
    """Use row selection, without depending on sidebar order or coordinates."""
    for node in descendants(application):
        if node.get_role() != roles.LIST:
            continue
        rows = [node.get_child_at_index(i) for i in range(node.get_child_count())]
        labels = [
            {
                child.get_name()
                for child in descendants(row, limit=30)
                if child.get_role() == roles.LABEL
            }
            if row.get_role() == roles.LIST_ITEM
            else set()
            for row in rows
        ]
        # Preferences lists can contain similar labels. Only select in the
        # navigation list, identified by multiple known section names.
        if len(set(titles.values()).intersection(set().union(*labels))) < 2:
            continue
        for index, names in enumerate(labels):
            if titles[page] in names:
                selection = node.get_selection_iface()
                return bool(
                    selection
                    and selection.select_child(index)
                    and selection.is_child_selected(index)
                )
    return False


def main(page):
    if page not in PAGES:
        return 2
    import gi

    gi.require_version("Atspi", "2.0")
    from gi.repository import Atspi, Gio, GLib

    # Bound the entire helper, including unresponsive accessibility providers.
    signal.alarm(8)
    Atspi.set_timeout(300, 500)
    bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    deadline = time.monotonic() + 6
    while time.monotonic() < deadline:
        try:
            pid = bus.call_sync(
                "org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus",
                "GetConnectionUnixProcessID",
                GLib.Variant("(s)", ("org.gnome.tweaks",)),
                None,
                Gio.DBusCallFlags.NONE,
                500,
                None,
            ).unpack()[0]
            desktop = Atspi.get_desktop(0)
            for index in range(desktop.get_child_count()):
                application = desktop.get_child_at_index(index)
                if application.get_process_id() != pid:
                    continue
                titles = translated_pages(application.get_object_locale())
                if select_sidebar_page(application, page, titles, Atspi.Role):
                    return 0
        except GLib.Error:
            # The app can still be starting or publishing its accessible tree.
            pass
        time.sleep(0.1)
    return 1


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv[1] if len(sys.argv) == 2 else ""))
    except (ImportError, ValueError):
        sys.exit(2)
