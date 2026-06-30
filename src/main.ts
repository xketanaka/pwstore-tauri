import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { api } from "./api.ts";
import { showScreen } from "./router.ts";
import { initInitScreen } from "./screens/init.ts";
import { initSearchScreen } from "./screens/search.ts";
import { initAdminScreen } from "./screens/admin.ts";

async function resizeTo(w: number, h: number): Promise<void> {
  try { await getCurrentWindow().setSize(new LogicalSize(w, h)); } catch {}
}

window.addEventListener("DOMContentLoaded", async () => {
  initInitScreen();
  initSearchScreen();
  initAdminScreen();

  showScreen("loading");

  try {
    const initialized = await api.isInitialized();

    if (!initialized) {
      await resizeTo(480, 650);
      showScreen("init");
      return;
    }

    await api.unlock();
    api.driveDownload().catch((e) => {
      const el = document.querySelector<HTMLElement>("#search-status");
      if (el) {
        el.textContent = `Drive同期エラー: ${e}`;
        el.className = "search-status search-status-error";
        el.hidden = false;
      }
    });
    showScreen("search");
  } catch (err) {
    console.error("起動エラー:", err);
    await resizeTo(480, 650);
    showScreen("init");
  }
});
