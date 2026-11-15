import { expect, test } from "vitest";
import { clampMenu } from "./menuPosition";

test("opens at the cursor when it fits", () => {
  expect(clampMenu(100, 120, 200, 150, 1400, 900)).toEqual({ left: 100, top: 120 });
});

test("flips left of the cursor near the right edge", () => {
  expect(clampMenu(1390, 120, 200, 150, 1400, 900)).toEqual({ left: 1190, top: 120 });
});

test("shifts up near the bottom edge", () => {
  expect(clampMenu(100, 850, 200, 150, 1400, 900)).toEqual({ left: 100, top: 742 });
});

test("never goes past the top-left margin in a tiny window", () => {
  expect(clampMenu(50, 50, 400, 400, 300, 300)).toEqual({ left: 8, top: 8 });
});
