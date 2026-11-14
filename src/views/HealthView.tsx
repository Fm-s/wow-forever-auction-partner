import { useEffect, useState } from "react";
import { getHealth, onSnapshot, onStatus } from "../api";
import { AhledgerCredit } from "../components/AhledgerCredit";
import { Money } from "../components/Money";
import { tickPosition } from "../timeline";
import { ago, dayTime } from "../time";
import { HOUSES, HOUSE_LABEL, type Feed, type FeedHealth, type Health, type House, type ItemRef } from "../types";

const FEED_LABEL: Record<string, string> = { items: "Items", commodities: "Commodities", pricetable: "Price table" };

function FeedStatus({ f }: { f: FeedHealth }) {
  const failing = f.lastErrorAt !== null && (f.lastSuccess === null || f.lastErrorAt > f.lastSuccess);
  if (failing) return <span className="badge bad">Failing</span>;
  if (f.stale) return <span className="badge stale">Stale</span>;
  return <span className="badge ok">OK</span>;
}

export function HealthView({ onOpenItem }: { onOpenItem: (house: House, item: ItemRef) => void }) {
  const [health, setHealth] = useState<Health | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    const load = () =>
      getHealth()
        .then((h) => live && (setHealth(h), setError(null)))
        .catch((e) => live && setError(String(e)));
    void load();
    const a = onSnapshot(() => void load());
    const b = onStatus(() => void load());
    return () => {
      live = false;
      void a.then((f) => f());
      void b.then((f) => f());
    };
  }, []);

  if (error) return <div className="view-empty">Couldn't load sources: {error}</div>;
  if (!health) return <div className="view-empty">Loading sources…</div>;

  const now = Date.now() / 1000;
  const from = now - 7 * 86400;
  const lanes: { house: House; feed: Feed }[] = HOUSES.flatMap((house) => (["items", "commodities"] as Feed[]).map((feed) => ({ house, feed })));

  return (
    <div className="health-grid">
      <section className="panel">
        <h2>Feeds</h2>
        <table className="data">
          <thead>
            <tr>
              <th>Source</th>
              <th>House</th>
              <th>Feed</th>
              <th>Status</th>
              <th>Data from</th>
              <th>Last success</th>
              <th>Last error</th>
            </tr>
          </thead>
          <tbody>
            {health.feeds.map((f) => (
              <tr key={`${f.source}-${f.house}-${f.feed}`}>
                <td>{f.source === "blizzard" ? "Blizzard" : "AHledger"}</td>
                <td>
                  <span className={`house-dot ${f.house}`} />
                  {HOUSE_LABEL[f.house]}
                </td>
                <td>{FEED_LABEL[f.feed] ?? f.feed}</td>
                <td>
                  <FeedStatus f={f} />
                </td>
                <td>{dayTime(f.lastModified)}</td>
                <td>{ago(f.lastSuccess)}</td>
                <td>
                  {f.lastError ? (
                    <span className="error-text" title={f.lastError}>
                      {ago(f.lastErrorAt)}: {f.lastError}
                    </span>
                  ) : (
                    <span className="muted">—</span>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <section className="panel">
        <h2>Snapshots, last 7 days</h2>
        <p className="muted" style={{ marginTop: 0 }}>
          Each tick is one snapshot. Amber ticks follow a gap (Mac asleep or offline); sales across a gap are left out of estimates.
        </p>
        {lanes.map(({ house, feed }) => (
          <div className="timeline-row" key={`${house}-${feed}`}>
            <span>
              <span className={`house-dot ${house}`} />
              {HOUSE_LABEL[house]} {FEED_LABEL[feed].toLowerCase()}
            </span>
            <div className="timeline-track">
              {health.timeline
                .filter((p) => p.house === house && p.feed === feed)
                .map((p) => (
                  <span key={p.t} className={`timeline-tick${p.gap ? " gap" : ""}`} style={{ left: `${tickPosition(p.t, from, now)}%` }} title={dayTime(p.t)} />
                ))}
            </div>
          </div>
        ))}
      </section>

      <section className="panel">
        <h2>Blizzard vs AHledger disagreements</h2>
        <p className="muted" style={{ marginTop: 0 }}>
          Items whose lowest buyout differs by more than 10% between the two sources, compared only when both were observed within an hour of each other.
        </p>
        {health.disagreements.length === 0 ? (
          <div className="muted">No disagreements right now.</div>
        ) : (
          <table className="data">
            <thead>
              <tr>
                <th>Item</th>
                <th>House</th>
                <th className="num">Blizzard min</th>
                <th className="num">AHledger min</th>
                <th className="num">Difference</th>
              </tr>
            </thead>
            <tbody>
              {health.disagreements.map((d) => (
                <tr key={`${d.house}-${d.itemId}`} className="clickable" onClick={() => onOpenItem(d.house, { itemId: d.itemId, variantKey: "" })}>
                  <td>{d.name}</td>
                  <td>
                    <span className={`house-dot ${d.house}`} />
                    {HOUSE_LABEL[d.house]}
                  </td>
                  <td className="num">
                    <Money copper={d.blizzardMin} />
                  </td>
                  <td className="num">
                    <Money copper={d.ahledgerMin} />
                  </td>
                  <td className={`num ${d.deltaPct > 0 ? "up" : "down"}`}>{`${d.deltaPct > 0 ? "+" : ""}${d.deltaPct.toFixed(1)}%`}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <section className="panel">
        <h2>Sales estimate check</h2>
        <p className="muted" style={{ marginTop: 0 }}>
          Our est. sold/day against AHledger's for the items AHledger sees selling most. A ratio far from 1 across many items means the estimate is biased.
        </p>
        <table className="data">
          <thead>
            <tr>
              <th>Item</th>
              <th>House</th>
              <th className="num">Est. sold/day (ours)</th>
              <th className="num">AHledger sold/day</th>
              <th className="num">Ratio</th>
            </tr>
          </thead>
          <tbody>
            {health.calibration.map((c) => (
              <tr key={`${c.house}-${c.itemId}`}>
                <td>{c.name}</td>
                <td>
                  <span className={`house-dot ${c.house}`} />
                  {HOUSE_LABEL[c.house]}
                </td>
                <td className="num">{c.estSoldPerDay === null ? <span className="muted">—</span> : c.estSoldPerDay.toFixed(1)}</td>
                <td className="num">{c.ahlSalesPerDay.toFixed(1)}</td>
                <td className="num">{c.estSoldPerDay === null || c.ahlSalesPerDay === 0 ? <span className="muted">—</span> : (c.estSoldPerDay / c.ahlSalesPerDay).toFixed(2)}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <AhledgerCredit />
      </section>
    </div>
  );
}
