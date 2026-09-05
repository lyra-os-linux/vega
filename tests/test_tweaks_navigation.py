import importlib.util
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "tweaks_navigation", ROOT / "vega-gtk/resources/open-tweaks-page.py"
)
navigation = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(navigation)
ROLES = SimpleNamespace(LIST="list", LIST_ITEM="row", LABEL="label")


class Node:
    def __init__(self, role, name="", children=(), selectable=True):
        self.role = role
        self.name = name
        self.children = list(children)
        self.selectable = selectable
        self.selected = None

    def get_role(self):
        return self.role

    def get_name(self):
        return self.name

    def get_child_count(self):
        return len(self.children)

    def get_child_at_index(self, index):
        return self.children[index]

    def get_selection_iface(self):
        return self

    def select_child(self, index):
        if self.selectable:
            self.selected = index
        return self.selectable

    def is_child_selected(self, index):
        return self.selected == index


def sidebar(titles):
    return Node(
        ROLES.LIST,
        children=[
            Node(ROLES.LIST_ITEM, children=[Node("box", children=[Node(ROLES.LABEL, title)])])
            for title in titles
        ],
    )


class TweaksNavigationTests(unittest.TestCase):
    def test_navigation_uses_labels_instead_of_row_order(self):
        view = sidebar(["Windows", "Appearance", "Fonts"])
        for page, index in [("appearance", 1), ("fonts", 2), ("windows", 0)]:
            self.assertTrue(navigation.select_sidebar_page(view, page, navigation.PAGES, ROLES))
            self.assertEqual(view.selected, index)

    def test_localized_nested_sidebar_labels(self):
        titles = {"appearance": "Aparência", "fonts": "Fontes", "windows": "Janelas"}
        view = sidebar(["Fontes", "Aparência", "Janelas"])
        self.assertTrue(navigation.select_sidebar_page(view, "appearance", titles, ROLES))
        self.assertEqual(view.selected, 1)

    def test_does_not_select_a_setting_with_the_same_name(self):
        settings = sidebar(["Fonts", "Scaling"])
        view = sidebar(["Appearance", "Windows", "Fonts"])
        root = Node("application", children=[settings, view])
        self.assertTrue(navigation.select_sidebar_page(root, "fonts", navigation.PAGES, ROLES))
        self.assertIsNone(settings.selected)
        self.assertEqual(view.selected, 2)

    def test_missing_page_does_not_select_another_row(self):
        view = sidebar(["Appearance", "Windows"])
        self.assertFalse(navigation.select_sidebar_page(view, "fonts", navigation.PAGES, ROLES))
        self.assertIsNone(view.selected)

    def test_failed_selection_is_reported(self):
        view = sidebar(["Appearance", "Fonts", "Windows"])
        view.selectable = False
        self.assertFalse(navigation.select_sidebar_page(view, "fonts", navigation.PAGES, ROLES))

    def test_uses_tweaks_catalog_and_target_application_locale(self):
        with patch.object(navigation.gettext, "translation") as translation:
            translation.return_value.gettext.side_effect = lambda text: "translated:" + text
            self.assertEqual(navigation.translated_pages("pt_BR.UTF-8")["fonts"], "translated:Fonts")
            translation.assert_called_once_with(
                "gnome-tweaks", languages=["pt_BR.UTF-8"], fallback=True
            )


if __name__ == "__main__":
    unittest.main()
