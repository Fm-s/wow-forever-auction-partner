import { useEffect, useMemo, useState } from "react";
import { getCrossHouse, getItemHistory, getMarket, onSnapshot } from "../api";
import { AhledgerCredit } from "../components/AhledgerCredit";
import { qualityClass } from "../components/ItemName";
import { Money } from "../components/Money";
import { PriceChart, SoldChart } from "../components/PriceChart";
import { HOUSE_LABEL, type CrossHouseRow, type HistoryRange, type House, type ItemHistory, type ItemRef, type MarketRow } from "../types";

const RANGES: HistoryRange[] = ["24h", "7d", "30d", "all"];
const RANGE_LABEL: Record<HistoryRange, string> = { "24h": "24 hours", "7d": "7 days", "30d": "30 days", all: "All" };

export function ItemView({ house, item, onBack }: { house: House; item: ItemRef; onBack: () => void }) {
  const [range, setRange] = useState<HistoryRange>("7d");
  const [history, setHistory] = useState<ItemHistory | null>(null);
  const [cross, setCross] = useState<CrossHouseRow[]>([]);
  const [row, setRow] = useState<MarketRow | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    const load = () => {
      getItemHistory(house, item.itemId, item.variantKey, range)
        .then((h) => live && setHistory(h))
        .catch((e) => live && setError(String(e)));
      getCrossHouse(item.itemId, item.variantKey).then((c) => live && setCross(c));
      getMarket(house).then((m) => live && setRow(m.rows.find((r) => r.itemId === item.itemId && r.variantKey === item.variantKey) ?? null));
    };
    load();
    const un = onSnapshot((e) => e.house === house && load());
    return () => {
      live = false;
      void un.then((f) => f());
    };
  }, [house, item.itemId, item.variantKey, range]);

  const houseColor = useMemo(() => getComputedStyle(document.documentElement).getPropertyValue(`--${house}`).trim(), [house]);
  const name = row?.name ?? `Item #${item.itemId}`;

  return (
    <>
      <div className="item-head">
        {row?.iconUrl && <img src={row.iconUrl} alt="" />}
        <div>
          <h1 className={qualityClass(row?.quality)}>{name}</h1>
          <div className="meta">
            {[row?.itemClass, row?.itemSubclass].filter(Boolean).join(" / ")}
            {item.variantKey ? ` · variant ${item.variantKey}` : ""} · {HOUSE_LABEL[house]}
          </div>
        </div>
        <button className="btn" onClick={onBack}>
          Back to market
        </button>
      </div>
      {error && <div className="banner">Couldn't load history: {error}</div>}
      <div className="item-grid">
        <div className="panel">
          <div className="range-tabs" role="group" aria-label="Range">
            {RANGES.map((r) => (
              <button key={r} aria-pressed={range === r} onClick={() => setRange(r)}>
                {RANGE_LABEL[r]}
              </button>
            ))}
          </div>
          {history && history.points.length > 0 ? (
            <PriceChart history={history} houseColor={houseColor} />
          ) : (
            <div className="view-empty">No history in this range yet.</div>
          )}
          <h2>Est. sold per day</h2>
          {history && history.estSoldDaily.length > 0 ? <SoldChart daily={history.estSoldDaily} /> : <div className="muted">Not enough snapshots yet.</div>}
        </div>
        <div style={{ display: "grid", gap: 18, alignContent: "start" }}>
          <div className="panel">
            <h2>Now</h2>
            <dl className="stats">
              <dt>Min buyout</dt>
              <dd>
                <Money copper={row?.minPrice} />
              </dd>
              <dt>Median</dt>
              <dd>
                <Money copper={row?.median} />
              </dd>
              <dt>7d median</dt>
              <dd>
                <Money copper={row?.median7d} />
              </dd>
              <dt>Quantity</dt>
              <dd>{row ? row.quantity.toLocaleString() : "—"}</dd>
              <dt>Est. sold/day</dt>
              <dd>{row?.estSoldPerDay != null ? row.estSoldPerDay.toFixed(1) : "—"}</dd>
              <dt>Volatility (7d)</dt>
              <dd>{history?.volatility7d != null ? `${(history.volatility7d * 100).toFixed(1)}%` : "—"}</dd>
              <dt>AHledger min</dt>
              <dd>
                <Money copper={row?.ahlMin} />
              </dd>
            </dl>
          </div>
          <div className="panel">
            <h2>Across houses</h2>
            {cross.length === 0 ? (
              <div className="muted">Not listed in any house right now.</div>
            ) : (
              <table className="data">
                <thead>
                  <tr>
                    <th>House</th>
                    <th className="num">Min</th>
                    <th className="num">Qty</th>
                    <th className="num">Cut</th>
                  </tr>
                </thead>
                <tbody>
                  {cross.map((c) => (
                    <tr key={c.house}>
                      <td>
                        <span className={`house-dot ${c.house}`} />
                        {HOUSE_LABEL[c.house]}
                      </td>
                      <td className="num">
                        <Money copper={c.minPrice} />
                      </td>
                      <td className="num">{c.quantity.toLocaleString()}</td>
                      <td className="num">{(c.cutBps / 100).toFixed(0)}%</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
          {history && history.ahledger.length > 0 && <AhledgerCredit />}
        </div>
      </div>
    </>
  );
}
