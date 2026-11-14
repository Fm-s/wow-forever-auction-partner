import type { House, ItemRef } from "../types";

export function MarketView(_props: { house: House; onOpenItem: (item: ItemRef) => void }) {
  return <div className="view-empty">Market</div>;
}
