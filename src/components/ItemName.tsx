export function qualityClass(q: string | null | undefined): string {
  return q ? `q-${q.toLowerCase()}` : "q-common";
}

export function ItemName({ name, quality, iconUrl, variantKey }: { name: string; quality: string | null; iconUrl: string | null; variantKey?: string }) {
  return (
    <span className="item-cell">
      {iconUrl ? <img src={iconUrl} alt="" loading="lazy" /> : <span style={{ width: 22 }} />}
      <span className={qualityClass(quality)}>{name}</span>
      {variantKey ? <span className="variant" title="Bonus list (random suffix variant)">{variantKey}</span> : null}
    </span>
  );
}
