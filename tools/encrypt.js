#!/usr/bin/env node
/**
 * pwstore-tauri データ暗号化スクリプト
 *
 * 使い方:
 *   node encrypt.js <JSONファイル> <マスターパスフレーズ> [出力ファイル = data.enc]
 *
 *   出力ファイルを省略すると data.enc として保存します。
 *
 * JSON フォーマット（DataStore 形式）:
 *   {
 *     "version": 1,          // 省略可（デフォルト: 1）
 *     "categories": [...],   // 省略可（デフォルト: []）
 *     "entries": [
 *       {
 *         "id": 1,                     // 省略可（自動採番）
 *         "service_name": "GitHub",
 *         "account": "alice@example.com",
 *         "password": "secret",
 *         "url": "https://github.com", // 省略可
 *         "keyword": "git code",       // 省略可
 *         "category": "仕事",           // 省略可
 *         "otp_uri": "otpauth://...",  // 省略可
 *         "notes": "備考",              // 省略可
 *         "status": 1,                 // 省略可（デフォルト: 1）
 *         "extra_fields": []           // 省略可
 *       }
 *     ]
 *   }
 *
 * 依存: Node.js 組み込みモジュールのみ（外部パッケージ不要）
 */

import { readFileSync, writeFileSync } from "fs";
import { scryptSync, createCipheriv, randomBytes } from "crypto";

const SALT_LEN = 16;
const NONCE_LEN = 12;
const SCRYPT_N = 16384; // 2^14（Rust 側と同じ）
const SCRYPT_R = 8;
const SCRYPT_P = 1;
const KEY_LEN = 32;

function normalizeEntry(entry, nextId) {
  return {
    id:           entry.id > 0 ? entry.id : nextId,
    service_name: entry.service_name ?? "",
    account:      entry.account ?? "",
    password:     entry.password ?? "",
    ...(entry.url      ? { url:      entry.url }      : {}),
    keyword:      entry.keyword  ?? "",
    category:     entry.category ?? "",
    ...(entry.otp_uri  ? { otp_uri:  entry.otp_uri }  : {}),
    ...(entry.notes    ? { notes:    entry.notes }     : {}),
    status:       entry.status ?? 1,
    extra_fields: (entry.extra_fields ?? []).map((f) => ({
      key_name:  f.key_name  ?? "",
      value:     f.value     ?? "",
      encrypted: f.encrypted ?? false,
    })),
  };
}

function main() {
  const [, , jsonPath, passphrase, outPath = "data.enc"] = process.argv;

  if (!jsonPath || !passphrase) {
    console.error("使い方: node encrypt.js <JSONファイル> <マスターパスフレーズ> [出力ファイル]");
    process.exit(1);
  }

  // JSON 読み込み
  let raw;
  try {
    raw = JSON.parse(readFileSync(jsonPath, "utf8"));
  } catch (e) {
    console.error(`エラー: JSON の読み込みに失敗しました: ${e.message}`);
    process.exit(1);
  }

  if (!Array.isArray(raw.entries)) {
    console.error('エラー: JSON に "entries" 配列がありません');
    process.exit(1);
  }

  // ID の自動採番（既存 ID の最大値 + 1 から順に振る）
  const existingIds = raw.entries.map((e) => e.id ?? 0).filter((id) => id > 0);
  let nextId = existingIds.length > 0 ? Math.max(...existingIds) + 1 : 1;

  const entries = raw.entries.map((e) => {
    const entry = normalizeEntry(e, nextId);
    if (entry.id === nextId) nextId++;
    return entry;
  });

  const store = {
    version:    raw.version    ?? 1,
    entries,
    categories: raw.categories ?? [],
  };

  // 暗号化（AES-256-GCM + scrypt）
  const plaintext = Buffer.from(JSON.stringify(store), "utf8");
  const salt      = randomBytes(SALT_LEN);
  const nonce     = randomBytes(NONCE_LEN);
  const key       = scryptSync(passphrase, salt, KEY_LEN, { N: SCRYPT_N, r: SCRYPT_R, p: SCRYPT_P });

  const cipher     = createCipheriv("aes-256-gcm", key, nonce);
  const ciphertext = Buffer.concat([cipher.update(plaintext), cipher.final()]);
  const authTag    = cipher.getAuthTag();

  // フォーマット: salt(16) | nonce(12) | ciphertext | authTag(16)
  writeFileSync(outPath, Buffer.concat([salt, nonce, ciphertext, authTag]));

  console.log(`暗号化完了: ${entries.length} 件のエントリを ${outPath} に保存しました`);
}

main();
