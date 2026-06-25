export type Screen = "init" | "search" | "admin" | "loading";

const showCallbacks = new Map<Screen, () => void>();

export function onScreenShow(screen: Screen, cb: () => void): void {
  showCallbacks.set(screen, cb);
}

export function showScreen(screen: Screen): void {
  document.querySelectorAll<HTMLElement>("[data-screen]").forEach((el) => {
    el.hidden = true;
  });
  const target = document.querySelector<HTMLElement>(`[data-screen="${screen}"]`);
  if (target) target.hidden = false;
  showCallbacks.get(screen)?.();
}
