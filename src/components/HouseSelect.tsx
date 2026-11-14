import { HOUSES, HOUSE_LABEL, type House } from "../types";

export function HouseSelect({ value, onChange }: { value: House; onChange: (h: House) => void }) {
  return (
    <div className="house-select" role="tablist" aria-label="Auction house">
      {HOUSES.map((h) => (
        <button key={h} role="tab" aria-selected={value === h} className={`house-tab house-${h}`} onClick={() => onChange(h)}>
          {HOUSE_LABEL[h]}
        </button>
      ))}
    </div>
  );
}
