export type Screen = "init" | "search" | "admin" | "loading";

export function showScreen(screen: Screen): void {
  document.querySelectorAll<HTMLElement>("[data-screen]").forEach((el) => {
    el.hidden = true;
  });
  const target = document.querySelector<HTMLElement>(`[data-screen="${screen}"]`);
  if (target) target.hidden = false;
}
