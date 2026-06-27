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

export interface FlatEntry {
  id: number;
  service_name: string;
  account: string;
  password: string;
  status: number;
  keyword: string;
  category: string;
  extra1_key_name?: string;
  extra1_value?: string;
  extra1_encrypted?: boolean;
  extra2_key_name?: string;
  extra2_value?: string;
  extra2_encrypted?: boolean;
  extra3_key_name?: string;
  extra3_value?: string;
  extra3_encrypted?: boolean;
}

export const api = {
  // 認証情報
  isInitialized:        ()                                      => invoke<boolean>("is_initialized"),
  saveCredentials:      (googleAccount: string, passphrase: string) =>
                                                                   invoke<void>("save_credentials", { googleAccount, passphrase }),
  getGoogleAccount:     ()                                      => invoke<string>("get_google_account"),
  unlock:               ()                                      => invoke<void>("unlock"),
  // エントリ操作
  searchEntries:        (keyword: string)                      => invoke<Entry[]>("search_entries", { keyword }),
  upsertEntry:          (entry: Entry)                         => invoke<Entry>("upsert_entry", { entry }),
  deleteEntry:          (id: number)                           => invoke<void>("delete_entry", { id }),
  // インポート／エクスポート
  importFlat:           (entries: FlatEntry[])                 => invoke<number>("import_flat", { entries }),
  exportFlat:           ()                                     => invoke<FlatEntry[]>("export_flat"),
  // OTP
  generateOtp:          (otpUri: string)                      => invoke<[string, number]>("generate_otp", { otpUri }),
  // Google OAuth
  saveClientId:         (clientId: string)                    => invoke<void>("save_client_id", { clientId }),
  getClientId:          ()                                     => invoke<string>("get_client_id"),
  saveClientSecret:     (clientSecret: string)                => invoke<void>("save_client_secret", { clientSecret }),
  getClientSecret:      ()                                     => invoke<string>("get_client_secret"),
  startOauth:           ()                                     => invoke<void>("start_oauth"),
  handleOauthCallback:  (url: string)                         => invoke<void>("handle_oauth_callback", { url }),
};
