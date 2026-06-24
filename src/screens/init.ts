import { listen } from "@tauri-apps/api/event";
import { api } from "../api.ts";
import { showScreen } from "../router.ts";

export function initInitScreen(): void {
  setupStep1();
}

// ---- ステップ1: クライアントID・パスフレーズ入力 ----

function setupStep1(): void {
  const form       = document.querySelector<HTMLFormElement>("#init-form")!;
  const clientIdEl = document.querySelector<HTMLInputElement>("#init-client-id")!;
  const passEl     = document.querySelector<HTMLInputElement>("#init-passphrase")!;
  const confirmEl  = document.querySelector<HTMLInputElement>("#init-passphrase-confirm")!;
  const errorEl    = document.querySelector<HTMLElement>("#init-error")!;
  const submitBtn  = document.querySelector<HTMLButtonElement>("#init-submit")!;

  form.addEventListener("submit", async (e) => {
    e.preventDefault();
    hideError(errorEl);

    const clientId   = clientIdEl.value.trim();
    const passphrase = passEl.value;
    const confirm    = confirmEl.value;

    if (!clientId) {
      showError(errorEl, "クライアントIDを入力してください");
      clientIdEl.focus();
      return;
    }

    if (passphrase.length < 8) {
      showError(errorEl, "パスフレーズは8文字以上にしてください");
      passEl.focus();
      return;
    }

    if (passphrase !== confirm) {
      showError(errorEl, "パスフレーズが一致しません");
      confirmEl.focus();
      return;
    }

    submitBtn.disabled = true;
    submitBtn.textContent = "保存中...";

    try {
      await api.saveClientId(clientId);
      // Googleアカウント名はOAuth完了後に設定するため仮値として空を入れる
      await api.saveCredentials("", passphrase);
      showStep2();
    } catch (err) {
      showError(errorEl, `エラー: ${err}`);
      submitBtn.disabled = false;
      submitBtn.textContent = "次へ（Google認証）";
    }
  });
}

// ---- ステップ2: Google OAuth ----

function showStep2(): void {
  document.querySelector<HTMLElement>("#init-step1")!.hidden = true;
  document.querySelector<HTMLElement>("#init-step2")!.hidden = false;

  const oauthBtn  = document.querySelector<HTMLButtonElement>("#init-oauth-btn")!;
  const errorEl   = document.querySelector<HTMLElement>("#init-oauth-error")!;
  const statusEl  = document.querySelector<HTMLElement>("#init-oauth-status")!;

  // oauth-complete / oauth-error イベントを待ち受ける
  let unlistenComplete: (() => void) | null = null;
  let unlistenError: (() => void) | null = null;

  const cleanup = () => {
    unlistenComplete?.();
    unlistenError?.();
  };

  listen<void>("oauth-complete", async () => {
    cleanup();
    statusEl.textContent = "認証完了。データを読み込み中...";
    hideError(errorEl);
    try {
      await api.unlock();
      showScreen("search");
    } catch (err) {
      showError(errorEl, `データ読み込みエラー: ${err}`);
      oauthBtn.disabled = false;
      statusEl.textContent = "";
    }
  }).then((fn) => { unlistenComplete = fn; });

  listen<string>("oauth-error", (event) => {
    cleanup();
    showError(errorEl, `認証エラー: ${event.payload}`);
    oauthBtn.disabled = false;
    statusEl.textContent = "";
  }).then((fn) => { unlistenError = fn; });

  oauthBtn.addEventListener("click", async () => {
    hideError(errorEl);
    oauthBtn.disabled = true;
    statusEl.textContent = "ブラウザでGoogleアカウントを選択してください...";
    try {
      await api.startOauth();
    } catch (err) {
      showError(errorEl, `ブラウザを開けませんでした: ${err}`);
      oauthBtn.disabled = false;
      statusEl.textContent = "";
    }
  });
}

// ---- ユーティリティ ----

function showError(el: HTMLElement, msg: string): void {
  el.textContent = msg;
  el.hidden = false;
}

function hideError(el: HTMLElement): void {
  el.hidden = true;
}
