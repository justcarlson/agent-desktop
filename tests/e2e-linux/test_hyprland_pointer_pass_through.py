import contextlib
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from hyprland_leg_fakes import FakeContext, delivered
from runner.legs import leg_by_id
from scenarios import hyprland_pointer_pass_through as through
from scenarios.hyprland_pointer_pass_through import Sample

WINDOW = (10.0, 10.2)
MAIN = {"signature": "sig", "pid": 4242}
HIDE_LUA = ["eval", "hl.config({ cursor = { invisible = true } })"]
SHOW_LUA = ["eval", "hl.config({ cursor = { invisible = false } })"]


def drawn(oracle: str, start: float, end: float, shown: bool = True) -> Sample:
    return Sample(oracle, start, end, shown, "")


def press(widget: str = "button", n: int = 1) -> dict:
    return {"ev": "press", "widget": widget, "n": n, "button": 1, "wx": 160, "wy": 60, "t": 1.0}


def release(widget: str = "button") -> dict:
    return {"ev": "release", "widget": widget, "button": 1, "wx": 160, "wy": 60, "t": 1.1}


class OnePressTests(unittest.TestCase):
    def test_one_press_and_its_release_on_the_button_pass(self):
        self.assertEqual(through.one_press_problems([press(), release()]), [])

    def test_no_press_fails(self):
        problems = through.one_press_problems([])
        self.assertTrue(any("0 presses" in problem for problem in problems), problems)

    def test_two_presses_fail(self):
        problems = through.one_press_problems([press(), release(), press(), release()])
        self.assertTrue(any("2 presses" in problem for problem in problems), problems)

    def test_a_synthetic_double_press_counts_as_a_second_press(self):
        problems = through.one_press_problems([press(), press(n=2)])
        self.assertTrue(problems)

    def test_a_press_on_another_widget_fails(self):
        problems = through.one_press_problems([press(), press("area")])
        self.assertTrue(any("area" in problem for problem in problems), problems)


class DrawnProblemsTests(unittest.TestCase):
    def test_every_sample_drawn_and_each_oracle_inside_the_click_passes(self):
        samples = [drawn("layers", 9.9, 9.9), drawn("layers", 10.1, 10.1), drawn("pixels", 9.95, 10.15),
                   drawn("pixels", 10.15, 10.4)]
        self.assertEqual(through.drawn_problems(samples, WINDOW), [])

    def test_one_sample_without_the_cursor_fails(self):
        samples = [drawn("layers", 10.1, 10.1), drawn("pixels", 9.95, 10.15), drawn("pixels", 10.15, 10.4, False)]
        problems = through.drawn_problems(samples, WINDOW)
        self.assertTrue(any("pixels" in problem and "not drawn" in problem for problem in problems), problems)

    def test_layers_read_only_outside_the_click_fail(self):
        samples = [drawn("layers", 9.0, 9.0), drawn("layers", 11.0, 11.0), drawn("pixels", 9.95, 10.15)]
        problems = through.drawn_problems(samples, WINDOW)
        self.assertTrue(any("layers" in problem for problem in problems), problems)

    def test_pixels_grabbed_only_outside_the_click_fail(self):
        samples = [drawn("layers", 10.1, 10.1), drawn("pixels", 9.0, 9.9), drawn("pixels", 10.3, 10.5)]
        problems = through.drawn_problems(samples, WINDOW)
        self.assertTrue(any("pixels" in problem for problem in problems), problems)

    def test_no_samples_fail(self):
        self.assertEqual(len(through.drawn_problems([], WINDOW)), 2)


class DrawnWatchTests(unittest.TestCase):
    def test_each_read_is_sampled_and_a_failed_read_counts_as_not_drawn(self):
        def broken():
            raise AssertionError("grim could not capture")

        with through.DrawnWatch({"layers": lambda: True, "pixels": broken}, 0.01) as watch:
            pass
        layers = [sample for sample in watch.samples if sample.oracle == "layers"]
        pixels = [sample for sample in watch.samples if sample.oracle == "pixels"]
        self.assertTrue(layers and all(sample.drawn for sample in layers))
        self.assertTrue(pixels and not any(sample.drawn for sample in pixels))
        self.assertIn("grim could not capture", pixels[0].detail)

    def test_samples_carry_their_read_interval(self):
        with through.DrawnWatch({"layers": lambda: True}, 0.01) as watch:
            pass
        self.assertTrue(all(sample.start <= sample.end for sample in watch.samples))


class FakeCompositor:
    """`cursor:invisible` as getoption reports it, and every hyprctl argv sent to change it."""

    def __init__(self, lua: bool, reply_key: str, original):
        self.lua, self.key, self.value, self.sent = lua, reply_key, original, []

    def query(self, _runtime, _signature, name, *, pid=None):
        assert name == "getoption cursor:invisible" and pid == MAIN["pid"], (name, pid)
        return {"option": "cursor:invisible", self.key: self.value, "set": True}

    def control(self, _runtime, _signature, argv, **_kw):
        self.sent.append(argv)
        if argv == ["eval", "return 1"]:
            return "ok" if self.lua else "error"
        word = argv[-1] if argv[0] == "keyword" else argv[-1].split("invisible = ")[1].split(" ")[0]
        hidden = word == "true"
        self.value = {"bool": hidden, "int": int(hidden), "str": word}[self.key]
        return "ok"

    def patched(self):
        return mock.patch.multiple(through.hyprland, query=self.query, control=self.control,
                                   uses_lua=lambda *_: self.lua)


def pointer_leg_context() -> FakeContext:
    ctx = FakeContext(lambda argv: delivered({}))
    ctx.leg = leg_by_id("pointer.coordinate_click_through_cursor")
    return ctx


class SystemPointerHiddenTests(unittest.TestCase):
    def run_hidden(self, compositor: FakeCompositor):
        ctx, inside = pointer_leg_context(), []
        with compositor.patched(), mock.patch.object(through, "shared", return_value=Path("/run")):
            with through.system_pointer_hidden(ctx, MAIN):
                inside.append(compositor.value)
        return ctx, inside

    def test_lua_configs_hide_the_pointer_with_eval_and_restore_it(self):
        compositor = FakeCompositor(True, "bool", False)
        ctx, inside = self.run_hidden(compositor)
        self.assertEqual(inside, [True])
        self.assertEqual([argv for argv in compositor.sent if argv[0] == "eval" and "cursor" in argv[1]],
                         [HIDE_LUA, SHOW_LUA])
        self.assertEqual(ctx._reads["hyprland.cursor_invisible"],
                         {"before": [False], "after": [True], "restored": [False]})

    def test_legacy_configs_hide_the_pointer_with_keyword_and_restore_it(self):
        compositor = FakeCompositor(False, "int", 0)
        ctx, inside = self.run_hidden(compositor)
        self.assertEqual(inside, [1])
        self.assertEqual([argv for argv in compositor.sent if argv[0] == "keyword"],
                         [["keyword", "cursor:invisible", "true"], ["keyword", "cursor:invisible", "false"]])
        self.assertEqual(ctx._reads["hyprland.cursor_invisible"],
                         {"before": [False], "after": [True], "restored": [False]})

    def test_a_pointer_already_hidden_stays_hidden_after_the_block(self):
        compositor = FakeCompositor(True, "str", "true")
        self.run_hidden(compositor)
        self.assertEqual(compositor.value, "true")
        self.assertEqual(compositor.sent[-1], HIDE_LUA)

    def test_the_pointer_is_restored_when_the_block_fails(self):
        compositor = FakeCompositor(True, "bool", False)
        with compositor.patched(), mock.patch.object(through, "shared", return_value=Path("/run")):
            with self.assertRaises(RuntimeError):
                with through.system_pointer_hidden(pointer_leg_context(), MAIN):
                    raise RuntimeError("the click failed")
        self.assertIs(compositor.value, False)

    def test_a_refused_hide_fails_the_leg(self):
        compositor = FakeCompositor(True, "bool", False)
        compositor.control = lambda _r, _s, argv, **_kw: "ok" if argv == ["eval", "return 1"] else "error: no"
        with compositor.patched(), mock.patch.object(through, "shared", return_value=Path("/run")):
            with self.assertRaises(AssertionError):
                with through.system_pointer_hidden(pointer_leg_context(), MAIN):
                    pass


class FakeScreen:
    def __init__(self, state: dict):
        self.state = state

    def grab(self):
        self.state["grabs"].append(self.state["hidden"])
        return object()

    def shows(self, _image) -> bool:
        return True


class ClickThroughPointerTests(unittest.TestCase):
    def test_every_pixel_read_and_the_click_run_while_the_system_pointer_is_hidden(self):
        state = {"hidden": False, "grabs": [], "clicks": []}

        @contextlib.contextmanager
        def hidden(_ctx, _main):
            state["hidden"] = True
            try:
                yield
            finally:
                state["hidden"] = False

        def grab_until(screen, drawn):
            screen.grab()
            return object()

        def answer(argv):
            if argv[0] == "mouse-click":
                state["clicks"].append(state["hidden"])
            return delivered({})

        pressed = [{"ev": "press", "widget": "button", "n": 1}, {"ev": "release", "widget": "button"}]
        with tempfile.TemporaryDirectory() as directory, contextlib.ExitStack() as stack:
            nothing = lambda *_a, **_k: contextlib.nullcontext()
            patches = {"system_pointer_hidden": hidden, "shared": lambda _t: Path(directory),
                       "main_instance": lambda _t: MAIN, "registry_of": lambda *_: Path(directory),
                       "pad_ref": lambda *_: "@s1:e1", "_button_point": lambda *_: (100, 100),
                       "Screen": lambda *_: FakeScreen(state), "covering_layers": lambda *_: [through.NAMESPACE],
                       "_grab_until": grab_until, "harness_holder": nothing, "keep_drawn": nothing,
                       "apart_from_last_click": lambda *_: None, "events": lambda *_: pressed,
                       "settled": lambda *_a, **_k: pressed, "holders_in": lambda *_: [], "QUIET_S": 0.05}
            stack.enter_context(mock.patch.multiple(through, create=True, **patches))
            stack.enter_context(mock.patch.object(through.hyprland, "query", lambda *_a, **_k: {}))
            through._click_through(FakeContext(answer), {}, None, Path(directory) / "pad.log", "s")
        self.assertEqual(state["clicks"], [True])
        self.assertTrue(state["grabs"])
        self.assertTrue(all(state["grabs"]), state["grabs"])


class RegistryRowTests(unittest.TestCase):
    def test_the_pass_through_leg_is_a_headed_hyprland_pass_routed_to_this_scenario(self):
        from runner import legs

        leg = legs.leg_by_id("pointer.coordinate_click_through_cursor")
        self.assertEqual((leg.tiers, leg.modes, leg.disposition), (("hyprland",), ("headed",), "pass"))
        self.assertTrue({"hyprland", "applog", "pixels"} <= set(leg.oracles))
        self.assertIn("mouse-click --xy", leg.requirement)
        self.assertIn("exactly one press", leg.requirement)
        self.assertEqual(legs.LAZY_IMPLEMENTATIONS[leg.id],
                         ("scenarios.hyprland_pointer_pass_through", "coordinate_click_through_cursor"))

    def test_the_ref_click_row_stays_the_d12_refusal(self):
        from runner import legs

        leg = legs.leg_by_id("pointer.cursor_pass_through")
        self.assertEqual(leg.disposition, "asserted-refusal")
        self.assertIn("receiver_unproven", leg.reason)


if __name__ == "__main__":
    unittest.main()
