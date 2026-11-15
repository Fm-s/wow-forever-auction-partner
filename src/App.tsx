import { useEffect, useState } from "react";
import { getCollectorStatus, onStatus } from "./api";
import { HouseSelect } from "./components/HouseSelect";
import { hhmm } from "./time";
import type { CollectorStatus, House, ItemRef } from "./types";
import { HealthView } from "./views/HealthView";
import { ItemView } from "./views/ItemView";
import { MarketView } from "./views/MarketView";
import { SettingsView } from "./views/SettingsView";
import { UpdateBanner } from "./components/UpdateBanner";

type View = "market" | "item" | "health" | "settings";

function statusLabel(s: CollectorStatus | null): string {
  if (!s) return "Connecting…";
  switch (s.state) {
    case "running":
      return s.lastNewSnapshotAt ? `Updated ${hhmm(s.lastNewSnapshotAt)}` : "Waiting for first snapshot";
    case "paused":
      return "Collection paused";
    case "starting":
      return "Starting…";
    default:
      return s.message ?? s.state;
  }
}

export default function App() {
  const [view, setView] = useState<View>("market");
  const [house, setHouse] = useState<House>("alliance");
  const [item, setItem] = useState<ItemRef | null>(null);
  const [status, setStatus] = useState<CollectorStatus | null>(null);

  useEffect(() => {
    void getCollectorStatus().then(setStatus);
    const un = onStatus(setStatus);
    return () => {
      void un.then((f) => f());
    };
  }, []);

  const openItem = (h: House, ref: ItemRef) => {
    setHouse(h);
    setItem(ref);
    setView("item");
  };

  const nav: { id: View; label: string }[] = [
    { id: "market", label: "Market" },
    { id: "health", label: "Sources" },
    { id: "settings", label: "Settings" },
  ];
  const navActive = view === "item" ? "market" : view;

  return (
    <div className={`app house-theme-${house}`}>
      <header className="topbar">
        <span className="brand">Auctioneer</span>
        <HouseSelect value={house} onChange={setHouse} />
        <nav>
          {nav.map((n) => (
            <button key={n.id} className={navActive === n.id ? "active" : ""} aria-current={navActive === n.id ? "page" : undefined} onClick={() => setView(n.id)}>
              {n.label}
            </button>
          ))}
        </nav>
        <span className={`status status-${status?.state ?? "starting"}`} title={status?.message ?? undefined}>
          {statusLabel(status)}
        </span>
      </header>
      <div className="house-band" aria-hidden="true" />
      <UpdateBanner />
      <main>
        {view === "market" && <MarketView house={house} onOpenItem={(ref) => openItem(house, ref)} />}
        {view === "item" && item && <ItemView house={house} item={item} onBack={() => setView("market")} />}
        {view === "health" && <HealthView onOpenItem={openItem} />}
        {view === "settings" && <SettingsView />}
      </main>
    </div>
  );
}
