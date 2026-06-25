import { showAdminScreen } from "./admin.ts";

export function initSearchScreen(): void {
  document.querySelector<HTMLButtonElement>("#search-admin-btn")
    ?.addEventListener("click", () => showAdminScreen());
}
