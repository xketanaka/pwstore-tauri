import { invoke } from "@tauri-apps/api/core";

export interface ExtraField {
  key_name: string;
  value: string;
  encrypted: boolean;
}

export interface Entry {
  id: number;
  service_name: string;
  account: string;
  password: string;
  url?: string;
  keyword: string;
  category: string;
  otp_uri?: string;
  notes?: string;
  status: number;
  extra_fields: ExtraField[];
}


export const api = {
  // 認証情報
  isInitialized:        ()                                      => invoke<boolean>("is_initialized"),
  saveCredentials:      (passphrase: string)                    => invoke<void>("save_credentials", { passphrase }),
  unlock:               ()                                      => invoke<void>("unlock"),
  // エントリ操作
  searchEntries:        (keyword: string)                      => invoke<Entry[]>("search_entries", { keyword }),
  upsertEntry:          (entry: Entry)                         => invoke<Entry>("upsert_entry", { entry }),
  deleteEntry:          (id: number)                           => invoke<void>("delete_entry", { id }),
  // OTP
  generateOtp:          (otpUri: string)                      => invoke<[string, number]>("generate_otp", { otpUri }),
  // Google OAuth
  startOauth:           ()                                     => invoke<void>("start_oauth"),
  handleOauthCallback:  (url: string)                         => invoke<void>("handle_oauth_callback", { url }),
  // カテゴリ
  getCategories:        ()                                     => invoke<string[]>("get_categories"),
  setCategories:        (categories: string[])                 => invoke<void>("set_categories", { categories }),
  // ウィンドウ操作
  resizeAndCenter:      (width: number, height: number)         => invoke<void>("resize_and_center", { width, height }),
  // Google Drive 同期
  driveDownload:        ()                                     => invoke<void>("drive_download"),
  driveSync:            ()                                     => invoke<void>("drive_sync"),
  driveForceDownload:   ()                                     => invoke<void>("drive_force_download"),
  driveForceUpload:     ()                                     => invoke<void>("drive_force_upload"),
};
