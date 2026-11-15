// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import type { MarketRow, MarketView as MarketData } from "../types";

const market = vi.hoisted(() => ({ data: null as MarketData | null }));
vi.mock("../api", () => ({
  getMarket: () => Promise.resolve(market.data),
  onSnapshot: () => Promise.resolve(() => {}),
  loadTooltip: () => Promise.resolve(null),
}));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: () => Promise.resolve() }));

import { MarketView } from "./MarketView";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

function row(itemId: number, name: string, quantity: number, minPrice: number | null): MarketRow {
  return {
    itemId,
    variantKey: "",
    name,
    quality: "Common",
    itemClass: "Trade Goods",
    itemSubclass: null,
    iconUrl: null,
    minPrice,
    median: minPrice,
    quantity,
    median7d: null,
    estSoldPerDay: null,
    trendPct: null,
    ahlMin: null,
    ahlDeltaPct: null,
    feed: "items",
  };
}

const names = () => [...document.querySelectorAll("tbody tr td:first-child")].map((td) => td.textContent?.trim());
const header = (label: string) => [...document.querySelectorAll("th")].find((th) => th.textContent?.startsWith(label))!;
const button = (label: string) => [...document.querySelectorAll("button")].find((b) => b.textContent === label)!;
const click = (el: Element) =>
  act(async () => {
    el.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });

describe("MarketView table", () => {
  let host: HTMLDivElement;
  let root: Root;

  const render = async (rows: MarketRow[]) => {
    market.data = { house: "alliance", asOf: 1_700_000_000_000, stale: false, partiallyStale: false, rows };
    await act(async () => {
      root.render(<MarketView house="alliance" onOpenItem={() => {}} groups={[]} selectedGroups={[]} onSelectGroups={() => {}} onGroupsChanged={async () => {}} />);
    });
  };

  beforeEach(() => {
    document.body.innerHTML = "";
    host = document.createElement("div");
    document.body.append(host);
    root = createRoot(host);
  });
  afterEach(() => act(() => root.unmount()));

  test("sorts by quantity, most first, by default", async () => {
    await render([row(1, "Linen Cloth", 5, 10), row(2, "Copper Ore", 50, 20), row(3, "Silk Cloth", 20, 30)]);
    expect(names()).toEqual(["Copper Ore", "Silk Cloth", "Linen Cloth"]);
    expect(header("Qty").getAttribute("aria-sort")).toBe("descending");
  });

  test("sorts item names as text", async () => {
    await render([row(1, "silk Cloth", 1, 1), row(2, "Copper Ore", 2, 2), row(3, "Linen Cloth", 3, 3)]);
    await click(header("Item"));
    expect(header("Item").getAttribute("aria-sort")).toBe("ascending");
    expect(names()).toEqual(["Copper Ore", "Linen Cloth", "silk Cloth"]);
  });

  test("keeps items without a buyout last whichever way min buyout is sorted", async () => {
    await render([row(1, "No Price", 1, null), row(2, "Cheap", 2, 10), row(3, "Pricey", 3, 90)]);
    await click(header("Min buyout"));
    const first = names();
    await click(header("Min buyout"));
    const second = names();
    expect([first, second]).toContainEqual(["Cheap", "Pricey", "No Price"]);
    expect([first, second]).toContainEqual(["Pricey", "Cheap", "No Price"]);
  });

  test("shows 100 rows per page", async () => {
    await render(Array.from({ length: 150 }, (_, i) => row(i + 1, `Item ${i + 1}`, 1000 - i, 1)));
    expect(document.querySelectorAll("tbody tr")).toHaveLength(100);
    expect(document.querySelector(".pager span")?.textContent).toBe("Page 1 of 2");
    expect(button("Previous").hasAttribute("disabled")).toBe(true);

    await click(button("Next"));
    expect(document.querySelectorAll("tbody tr")).toHaveLength(50);
    expect(document.querySelector(".pager span")?.textContent).toBe("Page 2 of 2");
    expect(button("Next").hasAttribute("disabled")).toBe(true);
  });

  test("goes back to the first page when the rows change", async () => {
    await render(Array.from({ length: 150 }, (_, i) => row(i + 1, `Item ${i + 1}`, 1000 - i, 1)));
    await click(button("Next"));
    const search = document.querySelector<HTMLInputElement>('input[type="search"]')!;
    await act(async () => {
      const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
      set.call(search, "Item 1");
      search.dispatchEvent(new Event("input", { bubbles: true }));
    });
    expect(document.querySelector(".pager span")?.textContent).toBe("Page 1 of 1");
  });
});
