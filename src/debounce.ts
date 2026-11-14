export interface Debounced {
  (): void;
  cancel: () => void;
}

/** Runs `f` once, `ms` after the last call in a burst (snapshot events arrive in bursts of up to six). */
export function debounce(f: () => void, ms: number): Debounced {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const d = (() => {
    clearTimeout(timer);
    timer = setTimeout(f, ms);
  }) as Debounced;
  d.cancel = () => clearTimeout(timer);
  return d;
}
