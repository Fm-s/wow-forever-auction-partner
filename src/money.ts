export type CoinUnit = "g" | "s" | "c";
export interface CopperParts {
  negative: boolean;
  parts: { value: number; unit: CoinUnit }[];
}

/** Splits copper into gold/silver/copper, omitting zero units ("0c" for zero). */
export function copperParts(copper: number): CopperParts {
  const rounded = Math.round(copper);
  const negative = rounded < 0;
  const abs = Math.abs(rounded);
  const g = Math.floor(abs / 10000);
  const s = Math.floor((abs % 10000) / 100);
  const c = abs % 100;
  const parts: CopperParts["parts"] = [];
  if (g) parts.push({ value: g, unit: "g" });
  if (s) parts.push({ value: s, unit: "s" });
  if (c || parts.length === 0) parts.push({ value: c, unit: "c" });
  return { negative, parts };
}

/** "12g 34s 56c"; "—" when unknown. */
export function formatCopper(copper: number | null | undefined): string {
  if (copper === null || copper === undefined || Number.isNaN(copper)) return "—";
  const { negative, parts } = copperParts(copper);
  return (negative ? "-" : "") + parts.map((p) => `${p.value}${p.unit}`).join(" ");
}
