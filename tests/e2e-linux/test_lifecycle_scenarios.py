import unittest
from types import SimpleNamespace
from unittest.mock import patch

from runner.product import Envelope
from scenarios import lifecycle


class LifecycleScenarioTests(unittest.TestCase):
    def test_maximize_requires_both_axes_and_restore_requires_neither(self):
        vert, horz = "_NET_WM_STATE_MAXIMIZED_VERT", "_NET_WM_STATE_MAXIMIZED_HORZ"
        for maximized, restored, accepted in ((vert, "", False), (horz, "", False),
                (vert + ", " + horz, vert, False), (vert + ", " + horz, horz, False),
                (vert + ", " + horz, "", True)):
            with self.subTest(maximized=maximized, restored=restored):
                state = [""]
                def product(argv):
                    state[0] = maximized if argv[0] == "maximize" else restored
                    return Envelope(True, {}, None, "{}", 0, argv)
                ctx = SimpleNamespace(tier=object(), product=product,
                    before=lambda *a: None, after=lambda *a: None, restored=lambda *a: None)
                def changed(read, expected, **kw):
                    assert read() == expected, "window still maximized on one axis"
                    return expected
                with patch.object(lifecycle.apps, "launch", return_value=SimpleNamespace(windows=["w-test"])), \
                    patch.object(lifecycle.apps, "stop"), \
                    patch.object(lifecycle, "_wm_state", side_effect=lambda *a: state[0]), \
                    patch.object(lifecycle.atspi_raw, "poll", side_effect=lambda read,*a: read()), \
                    patch.object(lifecycle, "changed", side_effect=changed):
                    if accepted:
                        lifecycle.maximize_restore(ctx)
                    else:
                        with self.assertRaises(AssertionError):
                            lifecycle.maximize_restore(ctx)

    def test_resize_and_restore_use_visible_bounds_inside_client_shadows(self):
        raw = [39, 45, 822, 642]
        calls = []
        observations = {}
        def product(argv):
            calls.append(argv)
            if argv[0] == "resize-window":
                width, height = int(argv[4]), int(argv[6])
                self.assertIn((width, height), ((740, 550), (700, 520)))
                raw[2:] = [width + 122, height + 122]
            else:
                self.assertEqual(argv[0], "move-window")
                x, y = int(argv[4]), int(argv[6])
                self.assertEqual((x, y), (100, 100))
                raw[:2] = [x - 61, y - 55]
            return Envelope(True, {}, None, "{}", 0, argv)
        def changed(read, expected, **kw):
            self.assertEqual(read(), expected)
            return expected
        ctx = SimpleNamespace(tier=object(), product=product,
            before=lambda k,v: observations.update(before=v),
            after=lambda k,v: observations.update(after=v),
            restored=lambda k,v: observations.update(restored=v))
        with patch.object(lifecycle.apps, "launch", return_value=SimpleNamespace(windows=["w-test"])), \
            patch.object(lifecycle.apps, "stop"), \
            patch.object(lifecycle.x11, "geometry", side_effect=lambda *a: tuple(raw)), \
            patch.object(lifecycle, "frame_extents", return_value=(61, 61, 55, 67), create=True), \
            patch.object(lifecycle.atspi_raw, "poll", side_effect=lambda read,*a: read()), \
            patch.object(lifecycle, "changed", side_effect=changed):
            lifecycle.resize_window(ctx)
        self.assertEqual(observations, {"before": (100,100,700,520),
            "after": (100,100,740,550), "restored": (100,100,700,520)})
        self.assertEqual(len(calls), 3)
        self.assertEqual(raw, [39,45,822,642])
