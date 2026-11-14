import { expect, test } from "vitest";
import { tickPosition } from "./timeline";

test("position is the percentage of the window, clamped", () => {
  expect(tickPosition(150, 100, 200)).toBe(50);
  expect(tickPosition(100, 100, 200)).toBe(0);
  expect(tickPosition(50, 100, 200)).toBe(0);
  expect(tickPosition(250, 100, 200)).toBe(100);
});

test("empty window does not divide by zero", () => expect(tickPosition(5, 5, 5)).toBe(100));
