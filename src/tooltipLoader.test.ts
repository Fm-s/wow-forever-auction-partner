import { describe, expect, test, vi } from "vitest";
import { createTooltipLoader } from "./tooltipLoader";
import type { ItemTooltip } from "./types";

const tip = (name: string): ItemTooltip => ({ name, quality: "Common", lines: [], sellPrice: null });

describe("createTooltipLoader", () => {
  test("a locally stored tooltip never hits the network or shows the spinner", async () => {
    const source = { cached: vi.fn(async () => ({ cached: true, tooltip: tip("Linen Cloth") })), fetch: vi.fn() };
    const onRemote = vi.fn();
    await expect(createTooltipLoader(source)(2589, onRemote)).resolves.toEqual(tip("Linen Cloth"));
    expect(source.fetch).not.toHaveBeenCalled();
    expect(onRemote).not.toHaveBeenCalled();
  });

  test("an item stored as having no tooltip stays local too", async () => {
    const source = { cached: vi.fn(async () => ({ cached: true, tooltip: null })), fetch: vi.fn() };
    await expect(createTooltipLoader(source)(31337, vi.fn())).resolves.toBeNull();
    expect(source.fetch).not.toHaveBeenCalled();
  });

  test("a miss signals the spinner, then fetches from Blizzard", async () => {
    const order: string[] = [];
    const source = {
      cached: vi.fn(async () => ({ cached: false, tooltip: null })),
      fetch: vi.fn(async () => (order.push("fetch"), tip("Runecloth"))),
    };
    await expect(createTooltipLoader(source)(14047, () => order.push("spinner"))).resolves.toEqual(tip("Runecloth"));
    expect(order).toEqual(["spinner", "fetch"]);
  });

  test("repeat hovers reuse the first result: one lookup, one fetch", async () => {
    const source = { cached: vi.fn(async () => ({ cached: false, tooltip: null })), fetch: vi.fn(async () => tip("Runecloth")) };
    const load = createTooltipLoader(source);
    await load(14047, vi.fn());
    await load(14047, vi.fn());
    expect(source.cached).toHaveBeenCalledTimes(1);
    expect(source.fetch).toHaveBeenCalledTimes(1);
  });

  test("a second hover while the fetch is in flight also shows the spinner", async () => {
    let release!: (t: ItemTooltip) => void;
    const source = { cached: vi.fn(async () => ({ cached: false, tooltip: null })), fetch: vi.fn(() => new Promise<ItemTooltip>((r) => (release = r))) };
    const load = createTooltipLoader(source);
    const first = load(14047, vi.fn());
    const second = vi.fn();
    const p = load(14047, second);
    await vi.waitFor(() => expect(source.fetch).toHaveBeenCalled());
    expect(second).toHaveBeenCalled();
    release(tip("Runecloth"));
    await expect(Promise.all([first, p])).resolves.toEqual([tip("Runecloth"), tip("Runecloth")]);
  });

  test("a failed fetch is forgotten so the next hover retries", async () => {
    const fetch = vi.fn().mockRejectedValueOnce("Blizzard credentials rejected").mockResolvedValueOnce(tip("Runecloth"));
    const load = createTooltipLoader({ cached: vi.fn(async () => ({ cached: false, tooltip: null })), fetch });
    await expect(load(14047, vi.fn())).rejects.toBe("Blizzard credentials rejected");
    await expect(load(14047, vi.fn())).resolves.toEqual(tip("Runecloth"));
  });
});
