import { copperParts } from "../money";

/** Copper rendered as coloured g/s/c units, the way the game shows money. */
export function Money({ copper }: { copper: number | null | undefined }) {
  if (copper === null || copper === undefined || Number.isNaN(copper)) return <span className="money unknown">—</span>;
  const { negative, parts } = copperParts(copper);
  return (
    <span className="money">
      {negative && "-"}
      {parts.map((p) => (
        <span key={p.unit} className={`coin coin-${p.unit}`}>
          {p.unit === "g" ? p.value.toLocaleString() : p.value}
          <i>{p.unit}</i>
        </span>
      ))}
    </span>
  );
}
