import { describe, expect, it } from "vitest";
import { progressPercent } from "./updateState";

describe("progressPercent", () => {
  it("returns the downloaded share as an integer percent", () => {
    expect(progressPercent(50, 200)).toBe(25);
    expect(progressPercent(1, 3)).toBe(33);
  });

  it("is null when the total size is unknown", () => {
    expect(progressPercent(10, null)).toBeNull();
    expect(progressPercent(10, undefined)).toBeNull();
    expect(progressPercent(10, 0)).toBeNull();
  });

  it("clamps to 100 when more bytes arrive than announced", () => {
    expect(progressPercent(300, 200)).toBe(100);
  });
});
