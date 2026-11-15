import { useEffect, useState } from "react";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { progressPercent, type UpdatePhase } from "../updateState";

/** Checks GitHub Releases once at startup and offers a one-click install. Silent when offline or up to date. */
export function UpdateBanner() {
  const [update, setUpdate] = useState<Update | null>(null);
  const [phase, setPhase] = useState<UpdatePhase>("idle");
  const [percent, setPercent] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showNotes, setShowNotes] = useState(false);
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    if (import.meta.env.DEV) return;
    let cancelled = false;
    check()
      .then((u) => {
        if (!cancelled && u) {
          setUpdate(u);
          setPhase("available");
        }
      })
      .catch((e) => console.warn("update check failed", e));
    return () => {
      cancelled = true;
    };
  }, []);

  if (!update || dismissed || phase === "idle") return null;

  const install = async () => {
    setPhase("downloading");
    setError(null);
    let total: number | undefined;
    let downloaded = 0;
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") total = event.data.contentLength;
        if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          setPercent(progressPercent(downloaded, total));
        }
        if (event.event === "Finished") setPhase("ready");
      });
      setPhase("ready");
      await relaunch();
    } catch (e) {
      setError(String(e));
      setPhase("error");
    }
  };

  return (
    <div className="update-banner" role="status">
      <div className="update-row">
        <span>
          <strong>Auctioneer v{update.version}</strong> is available
          <span className="muted"> · you have v{update.currentVersion}</span>
        </span>
        {update.body && (
          <button className="link-btn" onClick={() => setShowNotes((s) => !s)} aria-expanded={showNotes}>
            {showNotes ? "Hide notes" : "What's new"}
          </button>
        )}
        <span className="update-actions">
          {phase === "downloading" && <span className="muted">{percent === null ? "Downloading…" : `Downloading ${percent}%`}</span>}
          {phase === "ready" && <span className="muted">Restarting…</span>}
          {(phase === "available" || phase === "error") && (
            <>
              <button className="btn" onClick={() => setDismissed(true)}>
                Later
              </button>
              <button className="primary" onClick={() => void install()}>
                {phase === "error" ? "Retry" : "Install & restart"}
              </button>
            </>
          )}
        </span>
      </div>
      {phase === "error" && error && <div className="msg-error">Update failed: {error}</div>}
      {showNotes && update.body && <pre className="update-notes">{update.body}</pre>}
    </div>
  );
}
