from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))

from runner import legs
from scenarios import hyprland_press_foot as subject


class PressFootTests(unittest.TestCase):
    def test_registered_headed_only(self):
        leg = legs.leg_by_id('keyboard.press_window_id_compositor_only', 'hyprland')
        self.assertEqual(leg.modes, ('headed',))
        self.assertIn(leg.id, legs.implementations())

    def test_oracle_requires_exact_bytes_no_bystander_and_proof(self):
        good = {'ok': True, 'data': {'details': {'focus_proof': 'compositor_only'}}}
        self.assertEqual(subject.problems(good, b'a\r', b''), [])
        for result, reader, other in [
            ({'ok': False}, b'a\r', b''),
            ({'ok': True, 'data': {}}, b'a\r', b''),
            (good, b'a', b''), (good, b'a\ra', b''), (good, b'a\r', b'a'),
        ]:
            self.assertTrue(subject.problems(result, reader, other))

    def test_type_refusal_needs_safe_zero_delivery_and_unchanged_reader(self):
        good = {'ok': False, 'error': {'code': 'APP_NOT_FOUND', 'disposition': {'delivery': 'not_delivered', 'retry': 'safe'}}}
        self.assertEqual(subject.type_problems(good, b'a\r', b'a\r'), [])
        self.assertTrue(subject.type_problems({'ok': True}, b'a\r', b'a\r'))
        self.assertTrue(subject.type_problems(good, b'a\r', b'a\rx'))

    def test_type_uses_window_id_as_ref_without_an_unsupported_scope_flag(self):
        self.assertEqual(subject.type_argv('w-hypr-1-2-a'), ['type', 'w-hypr-1-2-a', 'x'])
