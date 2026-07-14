import { listen } from "@tauri-apps/api/event";
import { api } from "./api.ts";

/**
 * invalid_grant エラーを検知したら true を返す
 */
export function isInvalidGrant(err: unknown): boolean {
  return String(err).includes("invalid_grant");
}

/**
 * Google OAuth フローを開始し、完了を待つ。
 * oauth-complete で resolve、oauth-error で reject する。
 */
export async function triggerReauth(): Promise<void> {
  return new Promise<void>((resolve, reject) => {
    let unlistenComplete: (() => void) | null = null;
    let unlistenError: (() => void) | null = null;

    const cleanup = () => {
      unlistenComplete?.();
      unlistenError?.();
    };

    listen<void>("oauth-complete", () => {
      cleanup();
      resolve();
    }).then(fn => { unlistenComplete = fn; });

    listen<string>("oauth-error", (event) => {
      cleanup();
      reject(new Error(event.payload));
    }).then(fn => { unlistenError = fn; });

    api.startOauth().catch(err => {
      cleanup();
      reject(err);
    });
  });
}
