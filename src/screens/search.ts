import { showScreen } from "../router.ts";

export function initSearchScreen(): void {
  document.querySelector<HTMLButtonElement>("#search-admin-btn")
    ?.addEventListener("click", () => showScreen("admin"));
}
