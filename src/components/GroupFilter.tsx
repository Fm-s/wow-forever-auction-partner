import { useCallback, useEffect, useRef, useState } from "react";
import { onDismiss } from "../dismiss";
import type { Group } from "../types";

interface Props {
  groups: Group[];
  selected: number[];
  onChange: (ids: number[]) => void;
}

/** Toolbar control: pick several groups; the Market shows items in any of them. */
export function GroupFilter({ groups, selected, onChange }: Props) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const chosen = groups.filter((g) => selected.includes(g.id));
  const label = chosen.length === 0 ? "All groups" : chosen.length === 1 ? chosen[0].name : `${chosen.length} groups`;

  const close = useCallback(() => setOpen(false), []);
  useEffect(() => (open ? onDismiss(() => ref.current, close) : undefined), [open, close]);

  const toggle = (id: number) => onChange(selected.includes(id) ? selected.filter((x) => x !== id) : [...selected, id]);

  return (
    <div className="group-filter" ref={ref}>
      <button type="button" className={`picker${chosen.length ? " active" : ""}`} aria-haspopup="true" aria-expanded={open} onClick={() => setOpen(!open)}>
        {label}
        <span className="caret" aria-hidden="true">
          ▾
        </span>
      </button>
      {open && (
        <div className="popover" role="group" aria-label="Filter by groups">
          {groups.length === 0 ? (
            <p className="popover-empty">No groups yet. Right-click an item to add it to a new group.</p>
          ) : (
            <>
              {groups.map((g) => (
                <label key={g.id} className="popover-option">
                  <input type="checkbox" checked={selected.includes(g.id)} onChange={() => toggle(g.id)} />
                  <span className="popover-name">{g.name}</span>
                  <span className="muted">{g.members.length}</span>
                </label>
              ))}
              {chosen.length > 0 && (
                <button type="button" className="popover-clear" onClick={() => onChange([])}>
                  Show all items
                </button>
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
}
