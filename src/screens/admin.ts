import { showScreen } from "../router.ts";

export function initAdminScreen(): void {
  document.querySelector<HTMLButtonElement>("#admin-back-btn")
    ?.addEventListener("click", () => showScreen("search"));
}
