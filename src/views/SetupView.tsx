import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { completeSetup, getCredentialsStatus } from "../api";
import { REALM_TYPES, REALM_TYPE_LABEL, REGIONS, REGION_LABEL } from "../realm";
import logo from "../assets/logo.png";
import type { AppSettings, CredentialsStatus, RealmType, Region } from "../types";

const REALM_TYPE_HINT: Record<RealmType, string> = {
  pvp: "Open-world PvP",
  normal: "No open-world PvP",
  rp: "Roleplay, no open-world PvP",
  rppvp: "Roleplay with open-world PvP",
};

function ExternalLink({ href, children }: { href: string; children: string }) {
  return (
    <a
      href={href}
      onClick={(e) => {
        e.preventDefault();
        void openUrl(href);
      }}
    >
      {children}
    </a>
  );
}

/** First launch: pick the realm, then connect the Blizzard API. Nothing is collected before this is done. */
export function SetupView({ initial, onDone }: { initial: AppSettings; onDone: () => void }) {
  const [step, setStep] = useState<1 | 2>(1);
  const [region, setRegion] = useState<Region>(initial.region);
  const [realmType, setRealmType] = useState<RealmType>(initial.realmType);
  const [creds, setCreds] = useState<CredentialsStatus | null>(null);
  const [clientId, setClientId] = useState("");
  const [clientSecret, setClientSecret] = useState("");
  const [ahlKey, setAhlKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getCredentialsStatus()
      .then(setCreds)
      .catch(() => setCreds({ blizzardSet: false, ahledgerSet: false }));
  }, []);

  const keepSaved = !!creds?.blizzardSet && !clientId.trim() && !clientSecret.trim();

  async function finish(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    if (!keepSaved && (!clientId.trim() || !clientSecret.trim())) {
      setError("Enter both the client ID and the client secret.");
      return;
    }
    setBusy(true);
    try {
      await completeSetup(region, realmType, keepSaved ? null : clientId, keepSaved ? null : clientSecret, ahlKey || null);
      onDone();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="setup">
      <div className="setup-card">
        <div className="setup-brand">
          <img src={logo} alt="" />
          WoW Forever Auction Partner
        </div>
        <ol className="setup-steps" aria-label="Setup steps">
          <li aria-current={step === 1 ? "step" : undefined}>Realm</li>
          <li aria-current={step === 2 ? "step" : undefined}>Blizzard API</li>
        </ol>

        {step === 1 ? (
          <form
            onSubmit={(e) => {
              e.preventDefault();
              setStep(2);
            }}
          >
            <h1>Where do you play?</h1>
            <p className="hint">
              The app tracks the Alliance, Horde and Blackwater auction houses of one WoW Forever realm type. You can change it later in Settings.
            </p>
            <fieldset className="choice-row">
              <legend>Region</legend>
              {REGIONS.map((r) => (
                <label key={r} className={`choice${region === r ? " selected" : ""}`}>
                  <input type="radio" name="region" value={r} checked={region === r} onChange={() => setRegion(r)} />
                  {REGION_LABEL[r]}
                </label>
              ))}
            </fieldset>
            <fieldset className="choice-grid">
              <legend>Realm type</legend>
              {REALM_TYPES.map((t) => (
                <label key={t} className={`choice${realmType === t ? " selected" : ""}`}>
                  <input type="radio" name="realmType" value={t} checked={realmType === t} onChange={() => setRealmType(t)} />
                  <span className="choice-title">{REALM_TYPE_LABEL[t]}</span>
                  <span className="choice-hint">{REALM_TYPE_HINT[t]}</span>
                </label>
              ))}
            </fieldset>
            <div className="setup-actions">
              <button type="submit" className="primary">
                Continue
              </button>
            </div>
          </form>
        ) : (
          <form onSubmit={finish}>
            <h1>Connect the Blizzard API</h1>
            <p className="hint">
              Auction data comes from Blizzard's official API, which needs a free API client. Create one at{" "}
              <ExternalLink href="https://develop.battle.net/access/clients">develop.battle.net</ExternalLink> (any name and redirect URL), then paste its ID
              and secret. They're stored in your system's credential store (the Keychain on macOS).
            </p>
            {creds?.blizzardSet && <p className="setup-note">Credentials are already saved. Leave both fields empty to keep them.</p>}
            <label>
              Client ID
              <input value={clientId} onChange={(e) => setClientId(e.target.value)} autoComplete="off" spellCheck={false} autoFocus disabled={busy} />
            </label>
            <label>
              Client secret
              <input type="password" value={clientSecret} onChange={(e) => setClientSecret(e.target.value)} autoComplete="off" disabled={busy} />
            </label>
            <label>
              <span>
                AHledger key <span className="muted">(optional)</span>
              </span>
              <input type="password" value={ahlKey} onChange={(e) => setAhlKey(e.target.value)} autoComplete="off" placeholder="ahlk_…" disabled={busy} />
              <span className="hint">
                {creds?.ahledgerSet ? (
                  "An AHledger key is saved. Leave this empty to keep it."
                ) : (
                  <>
                    AHledger prices are compared without a key; a <ExternalLink href="https://ahledger.com/developers">free key</ExternalLink> raises its
                    rate limit.
                  </>
                )}
              </span>
            </label>
            {error && (
              <p className="msg-error" role="alert">
                {error}
              </p>
            )}
            <div className="setup-actions">
              <button type="button" className="btn" onClick={() => setStep(1)} disabled={busy}>
                Back
              </button>
              <button type="submit" className="primary" disabled={busy || !creds}>
                {busy ? (
                  <>
                    <span className="spinner" aria-hidden="true" /> Checking with Blizzard…
                  </>
                ) : keepSaved ? (
                  `Start tracking ${REALM_TYPE_LABEL[realmType]} · ${REGION_LABEL[region]}`
                ) : (
                  "Check and start tracking"
                )}
              </button>
            </div>
          </form>
        )}
      </div>
    </div>
  );
}
