import { api } from "./api.ts";
import { showScreen } from "./router.ts";
import { initInitScreen } from "./screens/init.ts";
import { initSearchScreen } from "./screens/search.ts";
import { initAdminScreen } from "./screens/admin.ts";

window.addEventListener("DOMContentLoaded", async () => {
  initInitScreen();
  initSearchScreen();
  initAdminScreen();

  showScreen("loading");

  try {
    const initialized = await api.isInitialized();

    if (!initialized) {
      showScreen("init");
      return;
    }

    await api.unlock();
    showScreen("search");
  } catch (err) {
    console.error("起動エラー:", err);
    showScreen("init");
  }
});
