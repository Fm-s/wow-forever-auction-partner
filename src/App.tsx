import { useCallback, useEffect, useState } from "react";
import { getCollectorStatus, getGroups, getSettings, onStatus } from "./api";
import { realmLabel } from "./realm";
import { SetupView } from "./views/SetupView";
import { HouseSelect } from "./components/HouseSelect";
import { hhmm } from "./time";
import type { AppSettings, CollectorStatus, Group, House, ItemRef } from "./types";
import { HealthView } from "./views/HealthView";
import { ItemView } from "./views/ItemView";
import { MarketView } from "./views/MarketView";
import { SettingsView } from "./views/SettingsView";

type View = "market" | "item" | "health" | "settings";

function statusLabel(s: CollectorStatus | null): string {
  if (!s) return "Connecting…";
  switch (s.state) {
    case "running":
      return s.lastNewSnapshotAt ? `Updated ${hhmm(s.lastNewSnapshotAt)}` : "Waiting for first snapshot";
    case "paused":
      return "Collection paused";
    case "setup":
      return "Finish setup to start collecting";
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
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  const [groups, setGroups] = useState<Group[]>([]);
  const [selectedGroups, setSelectedGroups] = useState<number[]>([]);

  // Selection is pruned to existing groups, so a deleted group's reused id can't come back selected.
  const reloadGroups = useCallback(async () => {
    const gs = await getGroups();
    setGroups(gs);
    setSelectedGroups((sel) => sel.filter((id) => gs.some((g) => g.id === id)));
  }, []);

  useEffect(() => {
    reloadGroups().catch(() => {});
  }, [reloadGroups]);

  const reloadSettings = useCallback(() => {
    getSettings()
      .then((s) => (setSettings(s), setSettingsError(null)))
      .catch((e) => setSettingsError(String(e)));
  }, []);
  useEffect(reloadSettings, [reloadSettings]);

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

  if (!settings) return <div className="view-empty">{settingsError ? `Couldn't load settings: ${settingsError}` : "Loading…"}</div>;
  if (!settings.setupDone) return <SetupView initial={settings} onDone={reloadSettings} />;

  return (
    <div className={`app house-theme-${house}`}>
      <header className="topbar">
        <span className="brand">Auctioneer</span>
        <span className="realm" title="Realm being tracked (change it in Settings)">
          {realmLabel(settings)}
        </span>
        <HouseSelect value={house} onChange={setHouse} />
        <nav>
          {nav.map((n) => (
            <button
              key={n.id}
              className={navActive === n.id ? "active" : ""}
              aria-current={navActive === n.id ? "page" : undefined}
              onClick={() => setView(n.id)}
            >
              {n.label}
            </button>
          ))}
        </nav>
        <span className={`status status-${status?.state ?? "starting"}`} title={status?.message ?? undefined}>
          {statusLabel(status)}
        </span>
      </header>
      <div className="house-band" aria-hidden="true" />
      <main>
        {view === "market" && (
          <MarketView
            house={house}
            onOpenItem={(ref) => openItem(house, ref)}
            groups={groups}
            selectedGroups={selectedGroups}
            onSelectGroups={setSelectedGroups}
            onGroupsChanged={reloadGroups}
          />
        )}
        {view === "item" && item && <ItemView house={house} item={item} onBack={() => setView("market")} />}
        {view === "health" && <HealthView onOpenItem={openItem} />}
        {view === "settings" && <SettingsView groups={groups} onGroupsChanged={reloadGroups} onSaved={setSettings} />}
      </main>
    </div>
  );
}
