import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { onOpenUrl } from "@tauri-apps/plugin-deep-link";
import { api } from "./api.ts";
import { showScreen } from "./router.ts";
import { initInitScreen } from "./screens/init.ts";
import { initSearchScreen } from "./screens/search.ts";
import { initAdminScreen } from "./screens/admin.ts";
import { showConflictDialog } from "./conflict.ts";
import { isInvalidGrant, triggerReauth } from "./reauth.ts";

function showDriveStatus(msg: string, isError = false): void {
  const el = document.querySelector<HTMLElement>("#search-status");
  if (!el) return;
  el.textContent = msg;
  el.className = "search-status" + (isError ? " search-status-error" : "");
  el.style.top = "56px";
  el.hidden = false;
}

function hideDriveStatus(): void {
  const el = document.querySelector<HTMLElement>("#search-status");
  if (el) el.hidden = true;
}

async function resizeTo(w: number, h: number): Promise<void> {
  try { await getCurrentWindow().setSize(new LogicalSize(w, h)); } catch {}
}

// Android: Google OAuth コールバックを deep link で受け取る
onOpenUrl((urls) => {
  const url = urls[0];
  if (url) api.handleOauthCallback(url).catch(console.error);
}).catch(console.error);

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
    api.driveDownload().catch(async (e) => {
      if (isInvalidGrant(e)) {
        showDriveStatus("Google認証が切れました。ブラウザで再認証してください...");
        try {
          await triggerReauth();
          await api.driveDownload();
          hideDriveStatus();
        } catch (authErr) {
          showDriveStatus(`再認証エラー: ${authErr}`, true);
        }
      } else if (String(e).includes("競合")) {
        const choice = await showConflictDialog();
        if (choice === "local") await api.driveForceUpload().catch(console.error);
        else if (choice === "drive") await api.driveForceDownload().catch(console.error);
      } else {
        showDriveStatus(`Drive同期エラー: ${e}`, true);
      }
    });
    showScreen("search");
  } catch (err) {
    console.error("起動エラー:", err);
    await resizeTo(480, 650);
    showScreen("init");
  }
});
