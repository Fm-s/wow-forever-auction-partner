import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getCredentialsStatus, getSettings, onStatus, runCollectionNow, saveCredentials, saveSettings } from "../api";
import { applySettingsForm } from "../settingsForm";
import { REALM_TYPES, REALM_TYPE_LABEL, REGIONS, REGION_LABEL, realmSwitchNote } from "../realm";
import type { AppSettings, CredentialsStatus, Group, RealmType, Region } from "../types";
import { GroupSettings } from "../components/GroupSettings";

const pct = (bps: number) => (bps / 100).toString();

export function SettingsView({
  groups,
  onGroupsChanged,
  onSaved,
}: {
  groups: Group[];
  onGroupsChanged: () => Promise<void>;
  onSaved: (s: AppSettings) => void;
}) {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [creds, setCreds] = useState<CredentialsStatus | null>(null);
  const [clientId, setClientId] = useState("");
  const [clientSecret, setClientSecret] = useState("");
  const [ahlKey, setAhlKey] = useState("");
  const [cuts, setCuts] = useState({ alliance: "", horde: "", blackwater: "" });
  const [cadenceMin, setCadenceMin] = useState("");
  const [realm, setRealm] = useState<{ region: Region; realmType: RealmType }>({ region: "us", realmType: "pvp" });
  const [message, setMessage] = useState<{
    kind: "ok" | "error";
    text: string;
  } | null>(null);

  useEffect(() => {
    void getSettings().then((s) => {
      setSettings(s);
      setCuts({
        alliance: pct(s.cutBpsAlliance),
        horde: pct(s.cutBpsHorde),
        blackwater: pct(s.cutBpsBlackwater),
      });
      setCadenceMin(String(Math.round(s.cadenceSecs / 60)));
      setRealm({ region: s.region, realmType: s.realmType });
    });
    void getCredentialsStatus().then(setCreds);
    // pause can also be toggled from the tray; keep the checkbox in step with it
    const un = onStatus((st) => setSettings((cur) => (cur ? { ...cur, collectionPaused: st.state === "paused" } : cur)));
    return () => {
      void un.then((f) => f());
    };
  }, []);

  if (!settings) return <div className="view-empty">Loading settings…</div>;

  async function save(e: React.FormEvent) {
    e.preventDefault();
    if (!settings) return;
    setMessage(null);
    try {
      // validate everything before saving anything
      const form = { cadenceMin, cuts, launchAtLogin: settings.launchAtLogin, ...realm };
      applySettingsForm(settings, form);
      if ((clientId || clientSecret || ahlKey) && (!clientId || !clientSecret)) {
        throw new Error("Enter both the Blizzard client ID and secret to change credentials.");
      }
      if (clientId) {
        await saveCredentials(clientId, clientSecret, ahlKey || null);
        setClientId("");
        setClientSecret("");
        setAhlKey("");
      }
      const next = applySettingsForm(await getSettings(), form);
      await saveSettings(next);
      setSettings(next);
      onSaved(next);
      setCreds(await getCredentialsStatus());
      await runCollectionNow();
      setMessage({ kind: "ok", text: "Settings saved. Collecting now." });
    } catch (err) {
      setMessage({
        kind: "error",
        text: String(err instanceof Error ? err.message : err),
      });
    }
  }

  async function togglePause(paused: boolean) {
    try {
      const next = { ...(await getSettings()), collectionPaused: paused };
      await saveSettings(next);
      setSettings((cur) => (cur ? { ...cur, collectionPaused: paused } : cur));
    } catch (err) {
      setMessage({ kind: "error", text: String(err) });
    }
  }

  return (
    <div className="settings-page">
      <form className="settings" onSubmit={save}>
        <section>
          <h2>Realm</h2>
          <div className="row2">
            <label>
              Region
              <select value={realm.region} onChange={(e) => setRealm({ ...realm, region: e.target.value as Region })}>
                {REGIONS.map((r) => (
                  <option key={r} value={r}>
                    {REGION_LABEL[r]}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Realm type
              <select value={realm.realmType} onChange={(e) => setRealm({ ...realm, realmType: e.target.value as RealmType })}>
                {REALM_TYPES.map((t) => (
                  <option key={t} value={t}>
                    {REALM_TYPE_LABEL[t]}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <p className="hint">{realmSwitchNote(settings, realm) ?? "Tracking this realm type's Alliance, Horde and Blackwater auction houses."}</p>
        </section>
        <section>
          <h2>Blizzard API</h2>
          <p className="hint">
            {creds?.blizzardSet
              ? "Blizzard credentials are set. Enter new ones to replace them."
              : "No credentials yet. Create a client at develop.battle.net."}
          </p>
          <label>
            Client ID
            <input
              value={clientId}
              onChange={(e) => setClientId(e.target.value)}
              autoComplete="off"
              spellCheck={false}
              placeholder={creds?.blizzardSet ? "saved" : ""}
            />
          </label>
          <label>
            Client secret
            <input
              type="password"
              value={clientSecret}
              onChange={(e) => setClientSecret(e.target.value)}
              autoComplete="off"
              placeholder={creds?.blizzardSet ? "saved" : ""}
            />
          </label>
        </section>

        <section>
          <h2>AHledger</h2>
          <p className="hint">
            Optional key for higher rate limits.{" "}
            <a
              href="https://ahledger.com/developers"
              onClick={(e) => {
                e.preventDefault();
                void openUrl("https://ahledger.com/developers");
              }}
            >
              Get a free key
            </a>
            {creds?.ahledgerSet ? " · A key is saved." : ""}
          </p>
          <label>
            API key
            <input type="password" value={ahlKey} onChange={(e) => setAhlKey(e.target.value)} autoComplete="off" placeholder="ahlk_…" />
          </label>
        </section>

        <section>
          <h2>Market</h2>
          <div className="row3">
            <label>
              Alliance cut %
              <input inputMode="decimal" value={cuts.alliance} onChange={(e) => setCuts({ ...cuts, alliance: e.target.value })} />
            </label>
            <label>
              Horde cut %
              <input inputMode="decimal" value={cuts.horde} onChange={(e) => setCuts({ ...cuts, horde: e.target.value })} />
            </label>
            <label>
              Blackwater cut %
              <input inputMode="decimal" value={cuts.blackwater} onChange={(e) => setCuts({ ...cuts, blackwater: e.target.value })} />
            </label>
          </div>
          <label className="narrow">
            Snapshot cadence (minutes)
            <input inputMode="numeric" value={cadenceMin} onChange={(e) => setCadenceMin(e.target.value)} />
          </label>
          <p className="hint">Snapshots further apart than 1.5× the cadence are treated as gaps and left out of sales estimates.</p>
        </section>

        <section>
          <h2>App</h2>
          <label className="check">
            <input type="checkbox" checked={settings.launchAtLogin} onChange={(e) => setSettings({ ...settings, launchAtLogin: e.target.checked })} />
            Open at login, in the menu bar
          </label>
          <label className="check">
            <input type="checkbox" checked={settings.collectionPaused} onChange={(e) => void togglePause(e.target.checked)} />
            Pause collection (applies immediately)
          </label>
        </section>

        <div className="actions">
          <button type="submit" className="primary">
            Save settings
          </button>
          {message && <span className={`msg msg-${message.kind}`}>{message.text}</span>}
        </div>
      </form>
      <GroupSettings groups={groups} onChanged={onGroupsChanged} />
    </div>
  );
}
