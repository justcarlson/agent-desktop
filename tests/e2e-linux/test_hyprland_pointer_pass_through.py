import unittest

from scenarios import hyprland_pointer_pass_through as through
from scenarios.hyprland_pointer_pass_through import Sample

WINDOW = (10.0, 10.2)


def drawn(oracle: str, start: float, end: float, shown: bool = True) -> Sample:
    return Sample(oracle, start, end, shown, "")


class PressesEnteredTests(unittest.TestCase):
    def test_one_digit_after_the_placement_digit_is_one_press(self):
        self.assertEqual(through.presses_entered("7", "77", "7"), 1)

    def test_a_zero_display_counts_as_empty(self):
        self.assertEqual(through.presses_entered("0", "7", "7"), 1)

    def test_two_digits_are_two_presses(self):
        self.assertEqual(through.presses_entered("7", "777", "7"), 2)

    def test_group_separators_do_not_count(self):
        self.assertEqual(through.presses_entered("777", "7,777", "7"), 1)

    def test_an_unchanged_display_is_no_press(self):
        self.assertEqual(through.presses_entered("7", "7", "7"), 0)

    def test_another_digit_is_not_a_press_of_this_one(self):
        self.assertIsNone(through.presses_entered("7", "78", "7"))

    def test_a_display_that_lost_its_start_is_not_a_press(self):
        self.assertIsNone(through.presses_entered("17", "77", "7"))


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


if __name__ == "__main__":
    unittest.main()
