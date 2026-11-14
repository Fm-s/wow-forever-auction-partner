import { describe, expect, test } from "vitest";
import { copperParts, formatCopper } from "./money";

describe("formatCopper", () => {
  test.each([
    [0, "0c"],
    [50, "50c"],
    [10000, "1g"],
    [123456, "12g 34s 56c"],
    [10050, "1g 50c"],
    [-1250, "-12s 50c"],
    [null, "—"],
    [undefined, "—"],
  ])("formatCopper(%p) = %p", (c, s) => expect(formatCopper(c as number | null | undefined)).toBe(s));

  test("non-integer copper is rounded", () => expect(formatCopper(99.6)).toBe("1s"));
});

describe("copperParts", () => {
  test("splits into denominations, skipping zeros", () =>
    expect(copperParts(1230056)).toEqual({ negative: false, parts: [{ value: 123, unit: "g" }, { value: 56, unit: "c" }] }));
  test("zero is a single copper part", () => expect(copperParts(0)).toEqual({ negative: false, parts: [{ value: 0, unit: "c" }] }));
});
