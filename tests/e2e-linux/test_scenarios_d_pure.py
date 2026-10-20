from __future__ import annotations

import sys
import unittest
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parent))

from scenarios import events, identity, launch, menus, surfaces
from scenarios import shell_surfaces, wayland_actions, wayland_focus, wayland_input


class FakeNode:
    def __init__(self, name="", text=None, states=(), children=()):
        self.name = name
        self._text = text
        self.states = frozenset(states)
        self._children = list(children)

    def children(self):
        return list(self._children)

    def find(self, *, role=None, name=None):
        found = []
        stack = [self]
        while stack:
            node = stack.pop()
            if (role is None or getattr(node, "role", None) == role) and (
                name is None or node.name == name
            ):
                found.append(node)
            stack.extend(node.children())
        return found

    def text(self):
        return self._text


class EventTests(unittest.TestCase):
    def test_kind_is_normalized(self) -> None:
        self.assertEqual(events.event_kind({"event": {"kind": "surface_appeared"}}), "surface-appeared")

    def test_missing_kind_is_empty(self) -> None:
        self.assertEqual(events.event_kind(None), "")

    def test_timeout_detection(self) -> None:
        self.assertTrue(events.timed_out(SimpleNamespace(ok=False, error={"code": "TIMEOUT"})))
        self.assertFalse(events.timed_out(SimpleNamespace(ok=False, error={"code": "STALE_REF"})))
        self.assertFalse(events.timed_out(SimpleNamespace(ok=True, error=None)))


class IdentityTests(unittest.TestCase):
    def test_ids_skip_entries_without_one(self) -> None:
        self.assertEqual(identity.window_ids([{"id": "w-1"}, {}, {"id": "w-2"}]), ["w-1", "w-2"])

    def test_empty_data_has_no_ids(self) -> None:
        self.assertEqual(identity.window_ids(None), [])


class SurfaceTests(unittest.TestCase):
    TREE = {
        "role": "window",
        "children": [
            {"role": "button", "name": "7", "ref": "@s:e1"},
            {"role": "group", "children": [{"role": "button", "name": "Main Menu", "ref": "@s:e2"}]},
        ],
    }

    def test_find_ref_walks_the_tree(self) -> None:
        self.assertEqual(surfaces.find_ref(self.TREE, name="Main Menu"), "@s:e2")

    def test_find_ref_requires_a_name_match(self) -> None:
        self.assertIsNone(surfaces.find_ref(self.TREE, name="About"))

    def test_surface_ids_group_by_id(self) -> None:
        grouped = surfaces.surface_ids(
            [{"id": "w-1", "type": "window"}, {"id": "w-1", "type": "sheet"}]
        )
        self.assertEqual(grouped, {"w-1": ["window", "sheet"]})

    def test_surface_ids_keep_distinct_ids_apart(self) -> None:
        grouped = surfaces.surface_ids([{"id": "a", "type": "window"}, {"id": "b", "type": "window"}])
        self.assertEqual(len(grouped), 2)


class MenuTests(unittest.TestCase):
    def test_names_containing_is_case_insensitive(self) -> None:
        root = FakeNode(children=[FakeNode("Power"), FakeNode("Mouse")])
        self.assertEqual(menus.names_containing(root, "power"), ["Power"])

    def test_timeout_message_reads_the_error(self) -> None:
        envelope = SimpleNamespace(error={"message": "the menu never opened"})
        self.assertIn("menu", menus.timeout_message(envelope))

    def test_is_timeout_requires_the_code(self) -> None:
        self.assertTrue(menus.is_timeout(SimpleNamespace(ok=False, error={"code": "TIMEOUT"})))
        self.assertFalse(menus.is_timeout(SimpleNamespace(ok=True, error=None)))


class LaunchTests(unittest.TestCase):
    def test_app_names_are_sorted(self) -> None:
        self.assertEqual(launch.app_names({"apps": [{"name": "b"}, {"name": "a"}]}), ["a", "b"])

    def test_missing_apps_is_empty(self) -> None:
        self.assertEqual(launch.app_names(None), [])

    def test_first_ref_returns_the_root_ref(self) -> None:
        self.assertEqual(launch._first_ref({"ref": "@s:e1"}), "@s:e1")

    def test_first_ref_descends(self) -> None:
        tree = {"children": [{"children": [{"ref": "@s:e9"}]}]}
        self.assertEqual(launch._first_ref(tree), "@s:e9")


class ShellTests(unittest.TestCase):
    def test_count_role_counts_descendants(self) -> None:
        tree = {"role": "menu", "children": [{"role": "menuitem"}, {"role": "menuitem"}]}
        self.assertEqual(shell_surfaces.count_role(tree, "menuitem"), 2)

    def test_walk_includes_the_root(self) -> None:
        self.assertEqual(len(shell_surfaces.walk({"role": "window"})), 1)

    def test_raw_showing_items_counts_showing_only(self) -> None:
        root = FakeNode(
            children=[
                FakeNode("a", states=("SHOWING",)),
                FakeNode("b", states=()),
            ]
        )
        for node in (root, *root.children()):
            node.role = shell_surfaces.MENU_ITEM_ROLE
        self.assertEqual(shell_surfaces.raw_showing_items(root), 1)

    def test_supported_reads_the_status_body(self) -> None:
        self.assertEqual(shell_surfaces.supported({"supported_surfaces": ["window"]}), ["window"])


class WaylandTests(unittest.TestCase):
    def test_document_text_joins_text_nodes(self) -> None:
        root = FakeNode(children=[FakeNode(text="alpha"), FakeNode(text="beta")])
        self.assertEqual(wayland_input.document_text(root), "alpha\nbeta")

    def test_text_ref_prefers_a_text_role(self) -> None:
        tree = {"role": "window", "children": [{"role": "button", "ref": "a"}, {"role": "textbox", "ref": "b"}]}
        self.assertEqual(wayland_input.text_ref(tree), "b")

    def test_search_finds_a_nested_detail(self) -> None:
        self.assertTrue(wayland_actions.search({"details": {"kind": "wayland_session"}}, "wayland_session"))

    def test_search_rejects_absent_text(self) -> None:
        self.assertFalse(wayland_focus.search({"message": "refused"}, "wayland_session"))

    def test_active_names_require_the_state(self) -> None:
        root = FakeNode(children=[FakeNode("Editor", states=("ACTIVE",)), FakeNode("Other")])
        self.assertEqual(wayland_focus.active_names(root), ["Editor"])

    def test_shows_matches_a_text_node(self) -> None:
        self.assertTrue(wayland_actions.shows(FakeNode(children=[FakeNode(text="7")]), "7"))

    def test_shows_rejects_a_different_value(self) -> None:
        self.assertFalse(wayland_actions.shows(FakeNode(children=[FakeNode(text="8")]), "7"))


if __name__ == "__main__":
    unittest.main()
