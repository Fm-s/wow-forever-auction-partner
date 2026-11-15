import { qualityClass } from "../quality";
import { ItemHover } from "./ItemTooltip";

export { qualityClass };

export function ItemName({ itemId, name, quality, iconUrl, variantKey }: { itemId: number; name: string; quality: string | null; iconUrl: string | null; variantKey?: string }) {
  return (
    <span className="item-cell">
      {iconUrl ? (
        <ItemHover itemId={itemId} name={name} quality={quality} variantKey={variantKey}>
          <img src={iconUrl} alt="" loading="lazy" />
        </ItemHover>
      ) : (
        <span style={{ width: 22 }} />
      )}
      <span className={qualityClass(quality)}>{name}</span>
      {variantKey ? <span className="variant" title="Bonus list (random suffix variant)">{variantKey}</span> : null}
    </span>
  );
}
