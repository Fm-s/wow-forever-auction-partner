import { describe, expect, test } from "vitest";
import { filterRows, filterOptions } from "./marketFilter";
import type { MarketRow } from "./types";

const row = (over: Partial<MarketRow>): MarketRow => ({
  itemId: 1,
  variantKey: "",
  name: "x",
  quality: "Common",
  itemClass: "Trade Goods",
  itemSubclass: "Cloth",
  iconUrl: null,
  minPrice: 1,
  median: 1,
  quantity: 1,
  median7d: null,
  estSoldPerDay: null,
  trendPct: null,
  ahlMin: null,
  ahlDeltaPct: null,
  feed: "items",
  ...over,
});

const rows = [
  row({ itemId: 2589, name: "Linen Cloth" }),
  row({ itemId: 25890, name: "Something Else" }),
  row({ itemId: 727, name: "Notched Shortsword", itemClass: "Weapon", quality: "Uncommon" }),
];
const none = { search: "", itemClass: null, quality: null };

describe("filterRows", () => {
  test("case-insensitive name search", () => expect(filterRows(rows, { ...none, search: "linen" }).map((r) => r.itemId)).toEqual([2589]));
  test("numeric search matches item id exactly", () => expect(filterRows(rows, { ...none, search: "2589" }).map((r) => r.itemId)).toEqual([2589]));
  test("class filter excludes other classes", () => expect(filterRows(rows, { ...none, itemClass: "Weapon" }).map((r) => r.itemId)).toEqual([727]));
  test("quality filter", () => expect(filterRows(rows, { ...none, quality: "Uncommon" }).map((r) => r.itemId)).toEqual([727]));
  test("empty filter keeps everything", () => expect(filterRows(rows, none)).toHaveLength(3));
});

describe("filterOptions", () => {
  test("distinct sorted classes and qualities, unknowns dropped", () =>
    expect(filterOptions([...rows, row({ itemClass: null, quality: null })])).toEqual({
      classes: ["Trade Goods", "Weapon"],
      qualities: ["Common", "Uncommon"],
    }));
});
