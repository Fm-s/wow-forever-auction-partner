import { describe, expect, test } from "vitest";
import { filterRows, filterOptions, groupUnion, memberKey } from "./marketFilter";
import type { Group, MarketRow } from "./types";

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
const none = { search: "", itemClass: null, quality: null, groupKeys: null };

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

describe("groups", () => {
  const groups: Group[] = [
    { id: 1, name: "Swords", members: [{ itemId: 727, variantKey: "12920" }] },
    { id: 2, name: "Cloth", members: [{ itemId: 2589, variantKey: "" }, { itemId: 727, variantKey: "12920" }] },
    { id: 3, name: "Empty", members: [] },
  ];

  test("union of the selected groups' variants", () => {
    expect(groupUnion(groups, [1, 2])).toEqual(new Set([memberKey(727, "12920"), memberKey(2589, "")]));
  });

  test("no selection means no group filter", () => {
    expect(groupUnion(groups, [])).toBeNull();
  });

  test("selected groups that were deleted are ignored; none left means no filter", () => {
    expect(groupUnion(groups, [3, 99])).toEqual(new Set());
    expect(groupUnion(groups, [99])).toBeNull();
  });

  test("filterRows keeps only member variants", () => {
    const variants = [row({ itemId: 727, variantKey: "12920", name: "Notched" }), row({ itemId: 727, variantKey: "12718", name: "Notched" }), row({ itemId: 2589, name: "Linen" })];
    const kept = filterRows(variants, { ...none, groupKeys: groupUnion(groups, [1]) });
    expect(kept.map((r) => `${r.itemId}:${r.variantKey}`)).toEqual(["727:12920"]);
  });
});
