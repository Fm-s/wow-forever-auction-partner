import { useCallback, useEffect, useMemo, useState } from "react";
import {
  createColumnHelper,
  createPaginatedRowModel,
  createSortedRowModel,
  metaHelper,
  rowPaginationFeature,
  rowSortingFeature,
  sortFn_alphanumeric,
  sortFn_text,
  tableFeatures,
  useTable,
  type SortingState,
} from "@tanstack/react-table";
import { getMarket, onSnapshot } from "../api";
import { debounce } from "../debounce";
import { AhledgerCredit } from "../components/AhledgerCredit";
import { ItemName } from "../components/ItemName";
import { Money } from "../components/Money";
import { filterOptions, filterRows, groupUnion } from "../marketFilter";
import { GroupFilter } from "../components/GroupFilter";
import { GroupMenu, type MenuTarget } from "../components/GroupMenu";
import { hhmm } from "../time";
import type { Group, House, ItemRef, MarketRow, MarketView as MarketData } from "../types";

const features = tableFeatures({
  rowSortingFeature,
  rowPaginationFeature,
  sortedRowModel: createSortedRowModel(),
  paginatedRowModel: createPaginatedRowModel(),
  // what sortFn "auto" picks for string columns, as v8 did
  sortFns: { alphanumeric: sortFn_alphanumeric, text: sortFn_text },
  columnMeta: metaHelper<{ num?: boolean }>(),
});

const col = createColumnHelper<typeof features, MarketRow>();

function Pct({ v }: { v: number | null }) {
  if (v === null) return <span className="muted">—</span>;
  const cls = v > 0.5 ? "up" : v < -0.5 ? "down" : "muted";
  return <span className={cls}>{`${v > 0 ? "+" : ""}${v.toFixed(1)}%`}</span>;
}

const columns = col.columns([
  col.accessor("name", {
    header: "Item",
    cell: (c) => (
      <ItemName
        itemId={c.row.original.itemId}
        name={c.row.original.name}
        quality={c.row.original.quality}
        iconUrl={c.row.original.iconUrl}
        variantKey={c.row.original.variantKey}
      />
    ),
  }),
  col.accessor("minPrice", {
    header: "Min buyout",
    cell: (c) => <Money copper={c.getValue()} />,
    sortUndefined: "last",
    meta: { num: true },
  }),
  col.accessor("median", {
    header: "Median",
    cell: (c) => <Money copper={c.getValue()} />,
    meta: { num: true },
  }),
  col.accessor("quantity", {
    header: "Qty",
    cell: (c) => c.getValue().toLocaleString(),
    meta: { num: true },
  }),
  col.accessor("median7d", {
    header: "7d median",
    cell: (c) => <Money copper={c.getValue()} />,
    meta: { num: true },
  }),
  col.accessor("estSoldPerDay", {
    header: "Est. sold/day",
    cell: (c) => {
      const v = c.getValue();
      return v === null ? <span className="muted">—</span> : v < 10 ? v.toFixed(1) : Math.round(v).toLocaleString();
    },
    meta: { num: true },
  }),
  col.accessor("trendPct", {
    header: "Trend",
    cell: (c) => <Pct v={c.getValue()} />,
    meta: { num: true },
  }),
  col.accessor("ahlMin", {
    header: "AHledger min",
    cell: (c) => <Money copper={c.getValue()} />,
    meta: { num: true },
  }),
  col.accessor("ahlDeltaPct", {
    header: "vs AHledger",
    cell: (c) => <Pct v={c.getValue()} />,
    meta: { num: true },
  }),
]);

interface Props {
  house: House;
  onOpenItem: (item: ItemRef) => void;
  groups: Group[];
  selectedGroups: number[];
  onSelectGroups: (ids: number[]) => void;
  onGroupsChanged: () => Promise<void>;
}

export function MarketView({ house, onOpenItem, groups, selectedGroups, onSelectGroups, onGroupsChanged }: Props) {
  const [menu, setMenu] = useState<MenuTarget | null>(null);
  const closeMenu = useCallback(() => setMenu(null), []);
  const [data, setData] = useState<MarketData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [itemClass, setItemClass] = useState<string | null>(null);
  const [quality, setQuality] = useState<string | null>(null);
  const [sorting, setSorting] = useState<SortingState>([{ id: "quantity", desc: true }]);

  useEffect(() => {
    let live = true;
    const load = () =>
      getMarket(house)
        .then((d) => live && (setData(d), setError(null)))
        .catch((e) => live && setError(String(e)));
    void load();
    const reload = debounce(() => void load(), 1500);
    const un = onSnapshot((e) => e.house === house && reload());
    return () => {
      live = false;
      reload.cancel();
      void un.then((f) => f());
    };
  }, [house]);

  const groupKeys = useMemo(() => groupUnion(groups, selectedGroups), [groups, selectedGroups]);
  const rows = useMemo(() => (data ? filterRows(data.rows, { search, itemClass, quality, groupKeys }) : []), [data, search, itemClass, quality, groupKeys]);
  const options = useMemo(() => filterOptions(data?.rows ?? []), [data]);

  const table = useTable({
    features,
    data: rows,
    columns,
    state: { sorting },
    onSortingChange: setSorting,
    initialState: { pagination: { pageIndex: 0, pageSize: 100 } },
    autoResetPageIndex: true,
  });

  if (error) return <div className="view-empty">Couldn't load the market: {error}</div>;
  if (!data) return <div className="view-empty">Loading market…</div>;
  if (data.rows.length === 0)
    return <div className="view-empty">No snapshots for this house yet. The collector fetches one every few minutes; check Sources if this persists.</div>;

  const showCredit = data.rows.some((r) => r.ahlMin !== null);
  const page = table.state.pagination;

  return (
    <>
      {data.stale && <div className="banner">Data is stale. The last snapshot is from {hhmm(data.asOf)}.</div>}
      {!data.stale && data.partiallyStale && (
        <div className="banner">One of this house's feeds is stale. Some prices are older than the rest; see Sources.</div>
      )}
      <div className="toolbar">
        <input type="search" placeholder="Search items or item ID" value={search} onChange={(e) => setSearch(e.target.value)} aria-label="Search items" />
        <select value={itemClass ?? ""} onChange={(e) => setItemClass(e.target.value || null)} aria-label="Item class">
          <option value="">All classes</option>
          {options.classes.map((c) => (
            <option key={c}>{c}</option>
          ))}
        </select>
        <select value={quality ?? ""} onChange={(e) => setQuality(e.target.value || null)} aria-label="Quality">
          <option value="">All qualities</option>
          {options.qualities.map((q) => (
            <option key={q}>{q}</option>
          ))}
        </select>
        <GroupFilter groups={groups} selected={selectedGroups} onChange={onSelectGroups} />
        <span className="count">
          {rows.length.toLocaleString()} items · snapshot {hhmm(data.asOf)}
        </span>
      </div>
      <table className="data">
        <thead>
          {table.getHeaderGroups().map((hg) => (
            <tr key={hg.id}>
              {hg.headers.map((h) => {
                const num = h.column.columnDef.meta?.num;
                const dir = h.column.getIsSorted();
                return (
                  <th
                    key={h.id}
                    className={`sortable${num ? " num" : ""}`}
                    onClick={h.column.getToggleSortingHandler()}
                    aria-sort={dir === "asc" ? "ascending" : dir === "desc" ? "descending" : "none"}
                  >
                    <table.FlexRender header={h} />
                    {dir === "asc" ? " ▲" : dir === "desc" ? " ▼" : ""}
                  </th>
                );
              })}
            </tr>
          ))}
        </thead>
        <tbody>
          {table.getRowModel().rows.map((r) => (
            <tr
              key={r.id}
              className="clickable"
              onClick={() =>
                onOpenItem({
                  itemId: r.original.itemId,
                  variantKey: r.original.variantKey,
                })
              }
              onContextMenu={(e) => {
                e.preventDefault();
                const o = r.original;
                setMenu({
                  x: e.clientX,
                  y: e.clientY,
                  itemId: o.itemId,
                  variantKey: o.variantKey,
                  name: o.name,
                  quality: o.quality,
                });
              }}
            >
              {r.getAllCells().map((c) => (
                <td key={c.id} className={c.column.columnDef.meta?.num ? "num" : ""}>
                  <table.FlexRender cell={c} />
                </td>
              ))}
            </tr>
          ))}
          {rows.length === 0 && (
            <tr>
              <td colSpan={columns.length} className="empty-row">
                {groupKeys ? "None of the selected groups' items are listed in this house right now." : "No items match these filters."}
              </td>
            </tr>
          )}
        </tbody>
      </table>
      {menu && <GroupMenu target={menu} groups={groups} onChanged={onGroupsChanged} onClose={closeMenu} />}
      <div className="pager">
        <button onClick={() => table.previousPage()} disabled={!table.getCanPreviousPage()}>
          Previous
        </button>
        <span>
          Page {page.pageIndex + 1} of {Math.max(1, table.getPageCount())}
        </span>
        <button onClick={() => table.nextPage()} disabled={!table.getCanNextPage()}>
          Next
        </button>
      </div>
      {showCredit && <AhledgerCredit />}
    </>
  );
}
