import type { ItemRef } from "../types";

export function HealthView(_props: { onOpenItem: (house: import("../types").House, item: ItemRef) => void }) {
  return <div className="view-empty">Health</div>;
}
