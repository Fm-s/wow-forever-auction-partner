/**
 * Closes a floating panel (menu, popover) on a press outside it, Escape, wheel-scrolling outside it,
 * resize or window blur. Returns a function that removes the listeners.
 *
 * The click that completes an outside press is swallowed, so dismissing the panel by clicking a table
 * row doesn't also open that row. `scroll` events are ignored on purpose: they also fire when an input
 * scrolls its text or the page shrinks, neither of which should close the panel.
 */
export function onDismiss(panel: () => HTMLElement | null, close: () => void): () => void {
  const inside = (e: Event) => e.target instanceof Node && !!panel()?.contains(e.target);
  const down = (e: MouseEvent) => {
    if (inside(e)) return;
    swallowNextClick();
    close();
  };
  const key = (e: KeyboardEvent) => {
    if (e.key === "Escape") close();
  };
  const wheel = (e: WheelEvent) => {
    if (!inside(e)) close();
  };

  const opts = { capture: true };
  window.addEventListener("mousedown", down, opts);
  window.addEventListener("wheel", wheel, opts);
  document.addEventListener("keydown", key);
  window.addEventListener("resize", close);
  window.addEventListener("blur", close);
  return () => {
    window.removeEventListener("mousedown", down, opts);
    window.removeEventListener("wheel", wheel, opts);
    document.removeEventListener("keydown", key);
    window.removeEventListener("resize", close);
    window.removeEventListener("blur", close);
  };
}

/**
 * Swallows the click that completes the current press. Independent of the panel's listeners, which
 * are usually gone by then (closing unmounts the panel).
 */
function swallowNextClick() {
  const opts = { capture: true };
  const click = (e: MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    done();
  };
  // A click, if any, is dispatched right after mouseup; after that the press is over (e.g. a drag).
  const up = () => setTimeout(done, 0);
  const done = () => {
    window.removeEventListener("click", click, opts);
    window.removeEventListener("mouseup", up, opts);
  };
  window.addEventListener("click", click, opts);
  window.addEventListener("mouseup", up, opts);
}
