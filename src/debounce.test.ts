import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { debounce } from "./debounce";

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

test("a burst of calls runs once, after the quiet period", () => {
  const f = vi.fn();
  const d = debounce(f, 1000);
  d();
  d();
  vi.advanceTimersByTime(999);
  d();
  vi.advanceTimersByTime(999);
  expect(f).not.toHaveBeenCalled();
  vi.advanceTimersByTime(1);
  expect(f).toHaveBeenCalledTimes(1);
});

test("cancel drops a pending call", () => {
  const f = vi.fn();
  const d = debounce(f, 100);
  d();
  d.cancel();
  vi.advanceTimersByTime(200);
  expect(f).not.toHaveBeenCalled();
});
