import { api } from "../api.ts";
import { showScreen } from "../router.ts";

export function initInitScreen(): void {
  const form      = document.querySelector<HTMLFormElement>("#init-form")!;
  const accountEl = document.querySelector<HTMLInputElement>("#init-google-account")!;
  const passEl    = document.querySelector<HTMLInputElement>("#init-passphrase")!;
  const confirmEl = document.querySelector<HTMLInputElement>("#init-passphrase-confirm")!;
  const errorEl   = document.querySelector<HTMLElement>("#init-error")!;
  const submitBtn = document.querySelector<HTMLButtonElement>("#init-submit")!;

  form.addEventListener("submit", async (e) => {
    e.preventDefault();
    hideError();

    const account    = accountEl.value.trim();
    const passphrase = passEl.value;
    const confirm    = confirmEl.value;

    if (!account) {
      showError("Googleアカウントを入力してください");
      accountEl.focus();
      return;
    }

    if (passphrase.length < 8) {
      showError("パスフレーズは8文字以上にしてください");
      passEl.focus();
      return;
    }

    if (passphrase !== confirm) {
      showError("パスフレーズが一致しません");
      confirmEl.focus();
      return;
    }

    submitBtn.disabled = true;
    submitBtn.textContent = "保存中...";

    try {
      await api.saveCredentials(account, passphrase);
      await api.unlock();
      showScreen("search");
    } catch (err) {
      showError(`エラー: ${err}`);
      submitBtn.disabled = false;
      submitBtn.textContent = "設定を保存する";
    }
  });

  function showError(msg: string): void {
    errorEl.textContent = msg;
    errorEl.hidden = false;
  }

  function hideError(): void {
    errorEl.hidden = true;
  }
}
