import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { getCurrent, onOpenUrl } from "@tauri-apps/plugin-deep-link";
import { listen } from "@tauri-apps/api/event";
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

/// 表示中の画面に応じてOAuthエラーを見える場所に出す
function showOauthError(msg: string): void {
  const initScreen = document.querySelector<HTMLElement>('[data-screen="init"]');
  const initError = document.querySelector<HTMLElement>("#init-oauth-error");
  if (initScreen && !initScreen.hidden && initError) {
    initError.textContent = msg;
    initError.hidden = false;
    return;
  }
  showDriveStatus(msg, true);
}

async function resizeTo(w: number, h: number): Promise<void> {
  try { await getCurrentWindow().setSize(new LogicalSize(w, h)); } catch {}
}

function startDriveSync(): void {
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
}

// Android: Google OAuth コールバックを deep link で受け取る。
// ブラウザから戻る際にアプリが作り直されている場合があるため、
// 認証成功後の画面遷移はここでも行う（init画面のリスナー任せにしない）
async function handleOauthDeepLink(url: string): Promise<void> {
  try {
    await api.handleOauthCallback(url);
    showScreen("search");
    startDriveSync();
  } catch (e) {
    console.error("OAuthコールバックエラー:", e);
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  initInitScreen();
  initSearchScreen();
  initAdminScreen();

  // Rust 側のどの失敗パスでもエラーが見えるようにする
  listen<string>("oauth-error", (e) => showOauthError(`認証エラー: ${e.payload}`))
    .catch(console.error);

  // 起動中に届いた deep link
  onOpenUrl((urls) => {
    if (urls[0]) handleOauthDeepLink(urls[0]);
  }).catch(console.error);

  // アプリ起動のきっかけになった deep link（onOpenUrl では拾えない）
  const launchUrls = await getCurrent().catch(() => null);
  if (launchUrls?.[0]) {
    await handleOauthDeepLink(launchUrls[0]);
    return;
  }

  showScreen("loading");

  try {
    const initialized = await api.isInitialized();

    if (!initialized) {
      await resizeTo(480, 650);
      showScreen("init");
      return;
    }

    await api.unlock();
    startDriveSync();
    showScreen("search");
  } catch (err) {
    console.error("起動エラー:", err);
    await resizeTo(480, 650);
    showScreen("init");
  }
});
