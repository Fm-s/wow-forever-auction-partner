import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { loadTooltip } from "../api";
import type { ItemTooltip as Tooltip } from "../types";
import { qualityClass } from "../quality";
import { Money } from "./Money";

const OPEN_DELAY_MS = 200;
const GAP = 12;
const EDGE = 8;

type State =
  | { kind: "local" } // waiting on the local lookup: nothing shown yet (it takes milliseconds)
  | { kind: "remote" } // not stored locally, fetching from Blizzard
  | { kind: "ready"; tooltip: Tooltip | null }
  | { kind: "error"; message: string };

interface Props {
  itemId: number;
  /** Name and quality as listed (a random-suffix variant shares its base item's tooltip). */
  name: string;
  quality: string | null;
  variantKey?: string;
  children: ReactNode;
}

/** Wraps an item icon: hovering it shows the item's in-game tooltip. */
export function ItemHover({ itemId, name, quality, variantKey, children }: Props) {
  const anchor = useRef<HTMLSpanElement>(null);
  const [rect, setRect] = useState<DOMRect | null>(null);
  const timer = useRef<number | undefined>(undefined);

  const open = () => {
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => anchor.current && setRect(anchor.current.getBoundingClientRect()), OPEN_DELAY_MS);
  };
  const close = () => {
    window.clearTimeout(timer.current);
    setRect(null);
  };

  useEffect(() => () => window.clearTimeout(timer.current), []);
  useEffect(() => {
    if (!rect) return;
    // A fixed-position tooltip would drift from its icon on scroll.
    window.addEventListener("scroll", close, true);
    return () => window.removeEventListener("scroll", close, true);
  }, [rect]);

  return (
    <span ref={anchor} className="item-hover" onMouseEnter={open} onMouseLeave={close}>
      {children}
      {rect && createPortal(<TooltipPanel itemId={itemId} name={name} quality={quality} variantKey={variantKey} anchor={rect} />, document.body)}
    </span>
  );
}

function TooltipPanel({ itemId, name, quality, variantKey, anchor }: Omit<Props, "children"> & { anchor: DOMRect }) {
  const [state, setState] = useState<State>({ kind: "local" });
  const panel = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null);

  useEffect(() => {
    let live = true;
    loadTooltip(itemId, () => live && setState({ kind: "remote" })).then(
      (tooltip) => live && setState({ kind: "ready", tooltip }),
      (e) => live && setState({ kind: "error", message: String(e) }),
    );
    return () => {
      live = false;
    };
  }, [itemId]);

  // Right of the icon, flipped left when it would overflow; kept inside the window vertically.
  useLayoutEffect(() => {
    const el = panel.current;
    if (!el) return;
    const { width, height } = el.getBoundingClientRect();
    let left = anchor.right + GAP;
    if (left + width > window.innerWidth - EDGE) left = Math.max(EDGE, anchor.left - width - GAP);
    const top = Math.min(Math.max(EDGE, anchor.top - 4), window.innerHeight - height - EDGE);
    setPos({ left, top });
  }, [anchor, state]);

  if (state.kind === "local") return null;

  return (
    <div ref={panel} className="tooltip" role="tooltip" style={pos ? { left: pos.left, top: pos.top } : { visibility: "hidden" }}>
      <div className={`tt-name ${qualityClass(quality)}`}>{name}</div>
      {state.kind === "remote" && (
        <div className="tt-loading" role="status">
          <span className="spinner" aria-hidden="true" />
          Loading from Blizzard…
        </div>
      )}
      {state.kind === "error" && <p className="tt-error">Couldn't load item info: {state.message}</p>}
      {state.kind === "ready" && !state.tooltip && <p className="tt-line tt-muted">Blizzard has no item details for this item.</p>}
      {state.kind === "ready" && state.tooltip && (
        <>
          {state.tooltip.lines.map((l, i) =>
            l.left === "" ? (
              <div key={i} className="tt-spacer" />
            ) : (
              <div key={i} className={`tt-line ${toneClass(l.tone)}`}>
                <span>{l.left}</span>
                {l.right && <span className="tt-right">{l.right}</span>}
              </div>
            ),
          )}
          {variantKey && <div className="tt-line tt-muted tt-note">Random suffix: this listing's bonus stats can differ.</div>}
          {state.tooltip.sellPrice ? (
            <div className="tt-line tt-sell">
              Sell price: <Money copper={state.tooltip.sellPrice} />
            </div>
          ) : null}
        </>
      )}
    </div>
  );
}

function toneClass(tone: string): string {
  return tone.startsWith("q-") ? tone : `tt-${tone}`;
}
