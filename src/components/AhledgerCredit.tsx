import { openUrl } from "@tauri-apps/plugin-opener";

/** Required credit wherever AHledger numbers are shown. */
export function AhledgerCredit() {
  return (
    <p className="credit">
      <a
        href="https://ahledger.com"
        onClick={(e) => {
          e.preventDefault();
          void openUrl("https://ahledger.com");
        }}
      >
        Prices from AHledger
      </a>
    </p>
  );
}
