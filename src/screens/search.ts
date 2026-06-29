import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, Entry } from "../api.ts";
import { onScreenShow } from "../router.ts";
import { showAdminScreen, showAdminScreenWithEntry } from "./admin.ts";

// ---- State ----

let results: Entry[] = [];
let selectedIdx = -1;

// ---- Init ----

export function initSearchScreen(): void {
  const input = document.querySelector<HTMLInputElement>("#search-input")!;

  document.querySelector<HTMLButtonElement>("#search-admin-btn")
    ?.addEventListener("click", () => showAdminScreen());

  input.addEventListener("input", () => {
    selectedIdx = -1;
    handleInput(input.value.trim());
  });

  input.addEventListener("keydown", (ev) => handleKeydown(ev, input));

  // 画面表示のたびにフォーカスをリセット
  onScreenShow("search", () => {
    input.value = "";
    selectedIdx = -1;
    results = [];
    setResultsVisible(false);
    input.focus();
  });
}

// ---- Search ----

async function handleInput(keyword: string): Promise<void> {
  if (!keyword) {
    results = [];
    setResultsVisible(false);
    return;
  }
  results = await api.searchEntries(keyword);
  renderResults();
}

// ---- Keyboard ----

async function handleKeydown(
  ev: KeyboardEvent,
  input: HTMLInputElement
): Promise<void> {
  const list = document.querySelector<HTMLUListElement>("#search-results")!;
  if (list.hidden) return;

  if (ev.key === "ArrowDown") {
    ev.preventDefault();
    selectedIdx = (selectedIdx + 1) % results.length;
    updateSelection();
  } else if (ev.key === "ArrowUp") {
    ev.preventDefault();
    selectedIdx = (selectedIdx - 1 + results.length) % results.length;
    updateSelection();
  } else if (ev.key === "Enter") {
    ev.preventDefault();
    const entry = results[selectedIdx];
    if (!entry) return;
    if (ev.shiftKey) {
      await copyToClipboard(entry.account);
      showStatus("アカウントをコピーしました");
    } else {
      await copyToClipboard(entry.password);
      showStatus("パスワードをコピーしました");
      setTimeout(() => getCurrentWindow().minimize(), 600);
    }
  } else if (ev.key === "Escape") {
    input.value = "";
    results = [];
    setResultsVisible(false);
  }
}

// ---- Render ----

function renderResults(): void {
  const list = document.querySelector<HTMLUListElement>("#search-results")!;
  list.innerHTML = "";

  if (results.length === 0) {
    setResultsVisible(false);
    return;
  }

  for (let i = 0; i < results.length; i++) {
    const entry = results[i];
    const li = document.createElement("li");
    li.className = "search-result-item";
    if (i === selectedIdx) li.classList.add("selected");

    li.innerHTML = `
      <span class="result-service">${esc(entry.service_name)}</span>
      <div class="result-right">
        <span class="result-account">${esc(entry.account)}</span>
        ${entry.otp_uri ? `<button class="btn-small btn-otp" data-i="${i}">OTP</button>` : ""}
        <button class="btn-small btn-detail" data-i="${i}">詳細</button>
      </div>
    `;

    // 行クリック: パスワードコピー＋最小化
    li.addEventListener("click", async (ev) => {
      if ((ev.target as HTMLElement).closest("button")) return;
      selectedIdx = i;
      updateSelection();
      await copyToClipboard(entry.password);
      getCurrentWindow().minimize();
    });

    // OTPボタン
    li.querySelector<HTMLButtonElement>(".btn-otp")?.addEventListener("click", async (ev) => {
      ev.stopPropagation();
      const btn = ev.currentTarget as HTMLButtonElement;
      try {
        const [code] = await api.generateOtp(entry.otp_uri!);
        await copyToClipboard(code);
        const orig = btn.textContent!;
        btn.textContent = code;
        setTimeout(() => { btn.textContent = orig; }, 2000);
      } catch {
        btn.textContent = "ERR";
        setTimeout(() => { btn.textContent = "OTP"; }, 1500);
      }
    });

    // 詳細ボタン
    li.querySelector<HTMLButtonElement>(".btn-detail")?.addEventListener("click", async (ev) => {
      ev.stopPropagation();
      await showAdminScreenWithEntry(entry);
    });

    list.appendChild(li);
  }

  setResultsVisible(true);
}

function updateSelection(): void {
  document.querySelectorAll<HTMLElement>(".search-result-item").forEach((el, i) => {
    el.classList.toggle("selected", i === selectedIdx);
  });
}

// ---- Utilities ----

function setResultsVisible(visible: boolean): void {
  const list = document.querySelector<HTMLUListElement>("#search-results")!;
  list.hidden = !visible;
}

async function copyToClipboard(text: string): Promise<void> {
  await navigator.clipboard.writeText(text);
}

function showStatus(msg: string): void {
  const el = document.querySelector<HTMLElement>("#search-status")!;
  el.textContent = msg;
  el.className = "search-status";
  el.hidden = false;
  setTimeout(() => { el.hidden = true; }, 1500);
}

function esc(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}
