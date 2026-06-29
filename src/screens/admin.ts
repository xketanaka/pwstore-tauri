import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { api, Entry, ExtraField } from "../api.ts";
import { showScreen } from "../router.ts";

const ADMIN_W = 960;
const ADMIN_H = 975;

// ---- State ----

let allEntries: Entry[] = [];
let allCategories: string[] = [];
let selectedCategory: string | null = null;  // null = すべて
let selectedEntryId: number | null = null;
let categoryEditMode = false;

// ---- Public API ----

export function initAdminScreen(): void {
  document.querySelector<HTMLButtonElement>("#admin-back-btn")
    ?.addEventListener("click", () => showScreen("search"));

  document.querySelector<HTMLButtonElement>("#admin-new-btn")
    ?.addEventListener("click", () => {
      selectedEntryId = null;
      renderServices();
      showEntryForm(null);
    });

  document.querySelector<HTMLButtonElement>("#admin-drive-upload-btn")
    ?.addEventListener("click", () => handleDriveSync("upload"));

  document.querySelector<HTMLButtonElement>("#admin-drive-download-btn")
    ?.addEventListener("click", () => handleDriveSync("download"));
}

export async function showAdminScreen(): Promise<void> {
  await getCurrentWindow().setSize(new LogicalSize(ADMIN_W, ADMIN_H));
  showScreen("admin");
  await refresh();
}

export async function showAdminScreenWithEntry(entry: Entry): Promise<void> {
  await getCurrentWindow().setSize(new LogicalSize(ADMIN_W, ADMIN_H));
  showScreen("admin");
  await refresh();
  selectedEntryId = entry.id;
  // カテゴリペインも選択状態に合わせる
  const cat = entry.category || "(なし)";
  selectedCategory = cat;
  renderCategories();
  renderServices();
  const found = allEntries.find((e) => e.id === entry.id) ?? entry;
  showEntryForm(found);
}

// ---- Data ----

async function refresh(): Promise<void> {
  [allEntries, allCategories] = await Promise.all([
    api.searchEntries(""),
    api.getCategories(),
  ]);
  renderCategories();
  renderServices();
}

// ---- Category Pane ----

function renderCategories(): void {
  const header = document.querySelector<HTMLElement>(".pane-category .pane-header")!;
  const list = document.querySelector<HTMLUListElement>("#category-list")!;

  if (categoryEditMode) {
    header.innerHTML = `<span>カテゴリ編集</span><button class="btn-icon pane-header-btn" id="category-done-btn">完了</button>`;
    document.querySelector<HTMLButtonElement>("#category-done-btn")!.addEventListener("click", () => {
      categoryEditMode = false;
      renderCategories();
    });
    renderCategoryEdit(list);
  } else {
    header.innerHTML = `<span>カテゴリ</span><button class="btn-icon pane-header-btn" id="category-edit-btn">✏</button>`;
    document.querySelector<HTMLButtonElement>("#category-edit-btn")!.addEventListener("click", () => {
      categoryEditMode = true;
      renderCategories();
    });
    renderCategoryList(list);
  }
}

function renderCategoryList(list: HTMLUListElement): void {
  list.innerHTML = "";
  const cats = [...new Set(allEntries.map((e) => e.category || "(なし)"))].sort();

  for (const cat of ["(すべて)", ...cats]) {
    const li = document.createElement("li");
    li.textContent = cat;
    const isAll = cat === "(すべて)";
    if ((isAll && selectedCategory === null) || cat === selectedCategory) {
      li.classList.add("active");
    }
    li.addEventListener("click", () => {
      selectedCategory = isAll ? null : cat;
      selectedEntryId = null;
      renderCategories();
      renderServices();
      showPlaceholder();
    });
    list.appendChild(li);
  }
}

function renderCategoryEdit(list: HTMLUListElement): void {
  list.innerHTML = "";

  for (const cat of allCategories) {
    const li = document.createElement("li");
    li.className = "category-edit-item";
    li.innerHTML = `
      <span class="category-edit-name">${esc(cat)}</span>
      <button class="btn-icon pane-header-btn">✕</button>
    `;
    li.querySelector("button")!.addEventListener("click", async () => {
      allCategories = allCategories.filter((c) => c !== cat);
      await api.setCategories(allCategories);
      renderCategoryEdit(list);
    });
    list.appendChild(li);
  }

  const addLi = document.createElement("li");
  addLi.className = "category-add-item";
  addLi.innerHTML = `
    <input type="text" class="category-new-input" placeholder="新しいカテゴリ" />
    <button class="btn-icon pane-header-btn">＋</button>
  `;
  list.appendChild(addLi);

  const input = addLi.querySelector<HTMLInputElement>(".category-new-input")!;
  const addBtn = addLi.querySelector<HTMLButtonElement>("button")!;

  const addCategory = async () => {
    const name = input.value.trim();
    if (!name || allCategories.includes(name)) { input.focus(); return; }
    allCategories = [...allCategories, name];
    await api.setCategories(allCategories);
    renderCategoryEdit(list);
  };
  addBtn.addEventListener("click", addCategory);
  input.addEventListener("keydown", (e) => { if (e.key === "Enter") { e.preventDefault(); addCategory(); } });
}

// ---- Service Pane ----

function filteredEntries(): Entry[] {
  if (selectedCategory === null) return allEntries;
  return allEntries.filter(
    (e) => (e.category || "(なし)") === selectedCategory
  );
}

function renderServices(): void {
  const list = document.querySelector<HTMLUListElement>("#service-list")!;
  list.innerHTML = "";

  for (const entry of filteredEntries()) {
    const li = document.createElement("li");
    li.textContent = entry.account
      ? `${entry.service_name} (${entry.account})`
      : entry.service_name;
    li.title = entry.account;
    if (entry.id === selectedEntryId) li.classList.add("active");
    li.addEventListener("click", () => {
      selectedEntryId = entry.id;
      renderServices();
      showEntryForm(entry);
    });
    list.appendChild(li);
  }
}

// ---- Detail Pane ----

function showEntryForm(entry: Entry | null): void {
  const detail = document.querySelector<HTMLElement>("#entry-detail")!;
  detail.innerHTML = buildFormHTML(entry);
  setupFormHandlers(entry);
}

function showPlaceholder(): void {
  const detail = document.querySelector<HTMLElement>("#entry-detail")!;
  detail.innerHTML = `<p class="placeholder-text">左のリストからサービスを選択してください</p>`;
}

function buildFormHTML(entry: Entry | null): string {
  const e = entry ?? emptyEntry();
  const extras = padExtras(e.extra_fields);

  return `
    <form id="entry-form" novalidate>
      <div class="form-row">
        <label>サービス名 <span class="required">*</span></label>
        <input type="text" name="service_name" value="${esc(e.service_name)}" required autocomplete="off" />
      </div>
      <div class="form-row">
        <label>アカウント <span class="required">*</span></label>
        <input type="text" name="account" value="${esc(e.account)}" autocomplete="off" />
      </div>
      <div class="form-row">
        <label>パスワード <span class="required">*</span></label>
        <div class="password-row">
          <input type="password" name="password" value="${esc(e.password)}" class="password-input" autocomplete="off" />
          <button type="button" class="btn-icon btn-show-pass" title="表示/非表示">👁</button>
        </div>
      </div>
      <div class="form-row">
        <label>URL</label>
        <input type="text" name="url" value="${esc(e.url ?? "")}" autocomplete="off" />
      </div>
      <div class="form-row">
        <label>カテゴリ</label>
        <select name="category">
          <option value="">（未分類）</option>
          ${allCategories.map((c) => `<option value="${esc(c)}"${c === e.category ? " selected" : ""}>${esc(c)}</option>`).join("")}
          ${e.category && !allCategories.includes(e.category) ? `<option value="${esc(e.category)}" selected>${esc(e.category)}</option>` : ""}
        </select>
      </div>
      <div class="form-row">
        <label>キーワード</label>
        <input type="text" name="keyword" value="${esc(e.keyword)}" autocomplete="off" />
      </div>
      <div class="form-row">
        <label>OTP URI</label>
        <input type="text" name="otp_uri" value="${esc(e.otp_uri ?? "")}" placeholder="otpauth://totp/..." autocomplete="off" />
      </div>
      <div class="form-row">
        <label>メモ</label>
        <textarea name="notes" rows="3">${esc(e.notes ?? "")}</textarea>
      </div>

      <div class="form-section-title">拡張フィールド</div>
      ${extras.map((f, i) => extraFieldHTML(f, i)).join("")}

      <div id="form-error" class="error-msg" hidden></div>

      <div class="form-actions">
        <button type="submit" class="btn-submit">
          ${entry ? "保存" : "追加"}
        </button>
        ${entry ? `<button type="button" id="delete-btn" class="btn-danger">削除</button>` : ""}
      </div>
    </form>
  `;
}

function extraFieldHTML(f: ExtraField, i: number): string {
  return `
    <div class="extra-field" data-index="${i}">
      <input type="text" class="extra-key" placeholder="項目名" value="${esc(f.key_name)}" autocomplete="off" />
      <input type="${f.encrypted ? "password" : "text"}" class="extra-val" placeholder="値" value="${esc(f.value)}" autocomplete="off" />
      <label class="extra-enc-label" title="暗号化して保存">
        <input type="checkbox" class="extra-enc" ${f.encrypted ? "checked" : ""} />
        🔒
      </label>
    </div>
  `;
}

function setupFormHandlers(entry: Entry | null): void {
  const form = document.querySelector<HTMLFormElement>("#entry-form")!;
  const errorEl = document.querySelector<HTMLElement>("#form-error")!;

  // パスワード表示/非表示トグル
  form.querySelector<HTMLButtonElement>(".btn-show-pass")?.addEventListener("click", () => {
    const input = form.querySelector<HTMLInputElement>(".password-input")!;
    input.type = input.type === "password" ? "text" : "password";
  });

  // 拡張フィールドの暗号化チェックで input type を切り替え
  form.querySelectorAll<HTMLInputElement>(".extra-enc").forEach((cb) => {
    cb.addEventListener("change", () => {
      const row = cb.closest<HTMLElement>(".extra-field")!;
      const val = row.querySelector<HTMLInputElement>(".extra-val")!;
      val.type = cb.checked ? "password" : "text";
    });
  });

  // 保存
  form.addEventListener("submit", async (ev) => {
    ev.preventDefault();
    errorEl.hidden = true;

    const data = new FormData(form);
    const serviceName = (data.get("service_name") as string).trim();
    const account = (data.get("account") as string).trim();
    const password = data.get("password") as string;

    if (!serviceName) { showError(errorEl, "サービス名は必須です"); return; }
    if (!account) { showError(errorEl, "アカウントは必須です"); return; }
    if (!password) { showError(errorEl, "パスワードは必須です"); return; }

    const updated: Entry = {
      id: entry?.id ?? 0,
      service_name: serviceName,
      account,
      password,
      url: (data.get("url") as string).trim() || undefined,
      category: (data.get("category") as string).trim(),
      keyword: (data.get("keyword") as string).trim(),
      otp_uri: (data.get("otp_uri") as string).trim() || undefined,
      notes: (data.get("notes") as string).trim() || undefined,
      status: entry?.status ?? 1,
      extra_fields: collectExtraFields(form),
    };

    try {
      const saved = await api.upsertEntry(updated);
      selectedEntryId = saved.id;
      await refresh();
      const found = allEntries.find((e) => e.id === saved.id) ?? saved;
      showEntryForm(found);
      autoUpload();
    } catch (err) {
      showError(errorEl, `保存エラー: ${err}`);
    }
  });

  // 削除
  form.querySelector<HTMLButtonElement>("#delete-btn")?.addEventListener("click", async () => {
    if (!entry) return;
    if (!window.confirm(`「${entry.service_name}」を削除しますか？`)) return;
    try {
      await api.deleteEntry(entry.id);
      selectedEntryId = null;
      await refresh();
      showPlaceholder();
      autoUpload();
    } catch (err) {
      showError(errorEl, `削除エラー: ${err}`);
    }
  });
}

function collectExtraFields(form: HTMLFormElement): ExtraField[] {
  const result: ExtraField[] = [];
  form.querySelectorAll<HTMLElement>(".extra-field").forEach((row) => {
    const key = (row.querySelector<HTMLInputElement>(".extra-key")?.value ?? "").trim();
    const val = row.querySelector<HTMLInputElement>(".extra-val")?.value ?? "";
    const enc = row.querySelector<HTMLInputElement>(".extra-enc")?.checked ?? false;
    if (key) result.push({ key_name: key, value: val, encrypted: enc });
  });
  return result;
}

// ---- Drive Auto-Upload ----

async function autoUpload(): Promise<void> {
  showAdminStatus("Driveに同期中...");
  try {
    await api.driveUpload();
    showAdminStatus("Drive同期完了");
    setTimeout(() => {
      const el = document.querySelector<HTMLElement>("#admin-status")!;
      el.hidden = true;
    }, 2000);
  } catch (err) {
    showAdminStatusError(`Drive同期エラー: ${err}`);
  }
}

// ---- Drive Sync ----

async function handleDriveSync(direction: "upload" | "download"): Promise<void> {
  const uploadBtn = document.querySelector<HTMLButtonElement>("#admin-drive-upload-btn")!;
  const downloadBtn = document.querySelector<HTMLButtonElement>("#admin-drive-download-btn")!;
  uploadBtn.disabled = true;
  downloadBtn.disabled = true;

  const label = direction === "upload" ? "アップロード" : "ダウンロード";
  showAdminStatus(`Drive ${label}中...`);

  try {
    if (direction === "upload") {
      await api.driveUpload();
    } else {
      await api.driveDownload();
      await refresh();
    }
    showAdminStatus(`Drive ${label}完了`);
    setTimeout(() => {
      const el = document.querySelector<HTMLElement>("#admin-status")!;
      el.hidden = true;
    }, 2000);
  } catch (err) {
    showAdminStatusError(`Drive ${label}エラー: ${err}`);
  } finally {
    uploadBtn.disabled = false;
    downloadBtn.disabled = false;
  }
}

function showAdminStatus(msg: string): void {
  const el = document.querySelector<HTMLElement>("#admin-status")!;
  el.textContent = msg;
  el.className = "admin-status";
  el.hidden = false;
}

function showAdminStatusError(msg: string): void {
  const el = document.querySelector<HTMLElement>("#admin-status")!;
  el.textContent = msg;
  el.className = "admin-status admin-status-error";
  el.hidden = false;
  el.onclick = () => { el.hidden = true; el.onclick = null; };
}

// ---- Utilities ----

function emptyEntry(): Entry {
  return {
    id: 0,
    service_name: "",
    account: "",
    password: "",
    url: undefined,
    category:
      selectedCategory !== null && selectedCategory !== "(なし)"
        ? selectedCategory
        : "",
    keyword: "",
    otp_uri: undefined,
    notes: undefined,
    status: 1,
    extra_fields: [],
  };
}

function padExtras(fields: ExtraField[]): ExtraField[] {
  const result = [...fields];
  while (result.length < 3) result.push({ key_name: "", value: "", encrypted: false });
  return result.slice(0, 3);
}

function esc(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function showError(el: HTMLElement, msg: string): void {
  el.textContent = msg;
  el.hidden = false;
}
