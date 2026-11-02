import unittest

from scenarios import hyprland_pointer_pass_through as through
from scenarios.hyprland_pointer_pass_through import Sample

WINDOW = (10.0, 10.2)


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
