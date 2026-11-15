import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createGroup, setGroupMember } from "../api";
import { clampMenu } from "../menuPosition";
import { qualityClass } from "../quality";
import type { Group } from "../types";

export interface MenuTarget {
  x: number;
  y: number;
  itemId: number;
  variantKey: string;
  name: string;
  quality: string | null;
}

interface Props {
  target: MenuTarget;
  groups: Group[];
  /** Reloads the group list after a change. */
  onChanged: () => Promise<void>;
  onClose: () => void;
}

/** Right-click menu on a Market row: toggle the row's variant in each group, or add it to a new group. */
export function GroupMenu({ target, groups, onChanged, onClose }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null);
  const [naming, setNaming] = useState(false);
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const { width, height } = el.getBoundingClientRect();
    setPos(clampMenu(target.x, target.y, width, height, window.innerWidth, window.innerHeight));
  }, [target, groups.length, naming, error]);

  useEffect(() => {
    // Naming focuses the name field; otherwise the first entry, for keyboard use.
    ref.current?.querySelector<HTMLElement>(naming ? "input" : "button")?.focus();
  }, [naming]);

  useEffect(() => {
    const outside = (e: MouseEvent) => !ref.current?.contains(e.target as Node) && onClose();
    const key = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    document.addEventListener("mousedown", outside);
    document.addEventListener("keydown", key);
    window.addEventListener("scroll", onClose, true);
    window.addEventListener("resize", onClose);
    window.addEventListener("blur", onClose);
    return () => {
      document.removeEventListener("mousedown", outside);
      document.removeEventListener("keydown", key);
      window.removeEventListener("scroll", onClose, true);
      window.removeEventListener("resize", onClose);
      window.removeEventListener("blur", onClose);
    };
  }, [onClose]);

  const isMember = (g: Group) => g.members.some((m) => m.itemId === target.itemId && m.variantKey === target.variantKey);

  async function run(action: () => Promise<unknown>, closeAfter: boolean) {
    setBusy(true);
    setError(null);
    try {
      await action();
      await onChanged();
      if (closeAfter) onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  const toggle = (g: Group) => run(() => setGroupMember(g.id, target.itemId, target.variantKey, !isMember(g)), false);
  const create = () =>
    run(async () => {
      const g = await createGroup(name);
      await setGroupMember(g.id, target.itemId, target.variantKey, true);
    }, true);

  return (
    <div
      ref={ref}
      className="menu"
      role="menu"
      aria-label={`Groups for ${target.name}`}
      style={pos ?? { visibility: "hidden" }}
      onContextMenu={(e) => e.preventDefault()}
    >
      <div className="menu-head">
        <span className={qualityClass(target.quality)}>{target.name}</span>
        {target.variantKey && <span className="variant">{target.variantKey}</span>}
      </div>
      {groups.map((g) => (
        <button
          key={g.id}
          type="button"
          role="menuitemcheckbox"
          aria-checked={isMember(g)}
          className="menu-item"
          disabled={busy}
          onClick={() => void toggle(g)}
        >
          <span className="menu-check" aria-hidden="true">
            {isMember(g) ? "✓" : ""}
          </span>
          {g.name}
        </button>
      ))}
      {groups.length > 0 && <div className="menu-sep" role="separator" />}
      {naming ? (
        <form
          className="menu-new"
          onSubmit={(e) => {
            e.preventDefault();
            void create();
          }}
        >
          <input value={name} onChange={(e) => setName(e.target.value)} placeholder="Group name" aria-label="New group name" maxLength={60} disabled={busy} />
          <button type="submit" className="btn" disabled={busy || !name.trim()}>
            Create
          </button>
        </form>
      ) : (
        <button type="button" role="menuitem" className="menu-item" disabled={busy} onClick={() => setNaming(true)}>
          <span className="menu-check" aria-hidden="true">
            +
          </span>
          New group…
        </button>
      )}
      {error && <p className="menu-error">{error}</p>}
    </div>
  );
}
