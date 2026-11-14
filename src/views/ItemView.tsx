import type { House, ItemRef } from "../types";

export function ItemView(_props: { house: House; item: ItemRef; onBack: () => void }) {
  return <div className="view-empty">Item</div>;
}
