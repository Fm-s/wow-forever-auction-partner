import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getCredentialsStatus, getSettings, runCollectionNow, saveCredentials, saveSettings } from "../api";
import type { AppSettings, CredentialsStatus } from "../types";

const pct = (bps: number) => (bps / 100).toString();
const bps = (pctText: string) => Math.round(parseFloat(pctText || "0") * 100);

export function SettingsView() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [creds, setCreds] = useState<CredentialsStatus | null>(null);
  const [clientId, setClientId] = useState("");
  const [clientSecret, setClientSecret] = useState("");
  const [ahlKey, setAhlKey] = useState("");
  const [cuts, setCuts] = useState({ alliance: "", horde: "", blackwater: "" });
  const [cadenceMin, setCadenceMin] = useState("");
  const [message, setMessage] = useState<{ kind: "ok" | "error"; text: string } | null>(null);

  useEffect(() => {
    void getSettings().then((s) => {
      setSettings(s);
      setCuts({ alliance: pct(s.cutBpsAlliance), horde: pct(s.cutBpsHorde), blackwater: pct(s.cutBpsBlackwater) });
      setCadenceMin(String(Math.round(s.cadenceSecs / 60)));
    });
    void getCredentialsStatus().then(setCreds);
  }, []);

  if (!settings) return <div className="view-empty">Loading settings…</div>;

  async function save(e: React.FormEvent) {
    e.preventDefault();
    if (!settings) return;
    setMessage(null);
    try {
      if (clientId || clientSecret || ahlKey) {
        if (!clientId || !clientSecret) throw new Error("Enter both the Blizzard client ID and secret to change credentials.");
        await saveCredentials(clientId, clientSecret, ahlKey || null);
        setClientId("");
        setClientSecret("");
        setAhlKey("");
      }
      const cadence = Math.round(parseFloat(cadenceMin) * 60);
      if (!(cadence >= 300)) throw new Error("Snapshot cadence must be at least 5 minutes.");
      const next: AppSettings = {
        ...settings,
        cadenceSecs: cadence,
        cutBpsAlliance: bps(cuts.alliance),
        cutBpsHorde: bps(cuts.horde),
        cutBpsBlackwater: bps(cuts.blackwater),
      };
      await saveSettings(next);
      setSettings(next);
      setCreds(await getCredentialsStatus());
      await runCollectionNow();
      setMessage({ kind: "ok", text: "Settings saved. Collecting now." });
    } catch (err) {
      setMessage({ kind: "error", text: String(err instanceof Error ? err.message : err) });
    }
  }

  return (
    <form className="settings" onSubmit={save}>
      <section>
        <h2>Blizzard API</h2>
        <p className="hint">
          {creds?.blizzardSet ? "Credentials are saved in your Keychain. Enter new ones to replace them." : "No credentials yet. Create a client at develop.battle.net."}
        </p>
        <label>
          Client ID
          <input value={clientId} onChange={(e) => setClientId(e.target.value)} autoComplete="off" spellCheck={false} placeholder={creds?.blizzardSet ? "saved" : ""} />
        </label>
        <label>
          Client secret
          <input type="password" value={clientSecret} onChange={(e) => setClientSecret(e.target.value)} autoComplete="off" placeholder={creds?.blizzardSet ? "saved" : ""} />
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
          <input type="checkbox" checked={settings.collectionPaused} onChange={(e) => setSettings({ ...settings, collectionPaused: e.target.checked })} />
          Pause collection
        </label>
      </section>

      <div className="actions">
        <button type="submit" className="primary">
          Save settings
        </button>
        {message && <span className={`msg msg-${message.kind}`}>{message.text}</span>}
      </div>
    </form>
  );
}
