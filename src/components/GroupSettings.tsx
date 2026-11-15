import { useState } from "react";
import { deleteGroup, renameGroup } from "../api";
import type { Group } from "../types";

interface Props {
  groups: Group[];
  onChanged: () => Promise<void>;
}

/** Settings section: rename and delete groups. */
export function GroupSettings({ groups, onChanged }: Props) {
  const [editing, setEditing] = useState<{ id: number; name: string } | null>(null);
  const [confirming, setConfirming] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function run(action: () => Promise<unknown>) {
    setError(null);
    try {
      await action();
      await onChanged();
      setEditing(null);
      setConfirming(null);
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="settings">
      <section>
        <h2>Groups</h2>
        <p className="hint">Right-click an item in the Market to add it to a group. Groups are shared by all houses.</p>
        {groups.length === 0 ? (
          <p className="hint">No groups yet.</p>
        ) : (
          <ul className="group-list">
            {groups.map((g) => (
              <li key={g.id}>
                {editing?.id === g.id ? (
                  <form
                    className="group-row"
                    onSubmit={(e) => {
                      e.preventDefault();
                      void run(() => renameGroup(g.id, editing.name));
                    }}
                  >
                    <input
                      autoFocus
                      value={editing.name}
                      onChange={(e) => setEditing({ id: g.id, name: e.target.value })}
                      aria-label={`New name for ${g.name}`}
                      maxLength={60}
                    />
                    <button type="submit" className="btn">
                      Save
                    </button>
                    <button type="button" className="btn" onClick={() => setEditing(null)}>
                      Cancel
                    </button>
                  </form>
                ) : confirming === g.id ? (
                  <div className="group-row">
                    <span className="group-name">Delete {g.name}? Its items stay in the market.</span>
                    <button type="button" className="btn danger" onClick={() => void run(() => deleteGroup(g.id))}>
                      Delete
                    </button>
                    <button type="button" className="btn" onClick={() => setConfirming(null)}>
                      Cancel
                    </button>
                  </div>
                ) : (
                  <div className="group-row">
                    <span className="group-name">{g.name}</span>
                    <span className="muted">
                      {g.members.length} {g.members.length === 1 ? "item" : "items"}
                    </span>
                    <button type="button" className="btn" onClick={() => (setEditing({ id: g.id, name: g.name }), setConfirming(null), setError(null))}>
                      Rename
                    </button>
                    <button type="button" className="btn" onClick={() => (setConfirming(g.id), setEditing(null), setError(null))}>
                      Delete
                    </button>
                  </div>
                )}
              </li>
            ))}
          </ul>
        )}
        {error && <p className="msg-error">{error}</p>}
      </section>
    </div>
  );
}
