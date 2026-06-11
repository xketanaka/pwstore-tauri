#!/usr/bin/env node
/**
 * pwstore-tauri 緊急データ復号スクリプト
 *
 * 使い方:
 *   node decrypt.js <暗号化ファイルのパス> <マスターパスフレーズ>
 *
 * 出力:
 *   復号された JSON を標準出力に表示します。
 *   ファイルに保存したい場合は:
 *   node decrypt.js data.enc "my passphrase" > data.json
 *
 * 依存: Node.js 組み込みモジュールのみ（外部パッケージ不要）
 */

import { readFileSync } from "fs";
import { scryptSync, createDecipheriv } from "crypto";

const SALT_LEN  = 16;
const NONCE_LEN = 12;
const AUTH_TAG_LEN = 16;

// Rust 側と同じ scrypt パラメータ
const SCRYPT_N      = 16384;  // 2^14
const SCRYPT_R      = 8;
const SCRYPT_P      = 1;
const KEY_LEN       = 32;

function main() {
  const [, , filePath, passphrase] = process.argv;

  if (!filePath || !passphrase) {
    console.error("使い方: node decrypt.js <ファイルパス> <マスターパスフレーズ>");
    process.exit(1);
  }

  const data = readFileSync(filePath);

  if (data.length < SALT_LEN + NONCE_LEN + AUTH_TAG_LEN) {
    console.error("エラー: ファイルが短すぎます");
    process.exit(1);
  }

  const salt       = data.subarray(0, SALT_LEN);
  const nonce      = data.subarray(SALT_LEN, SALT_LEN + NONCE_LEN);
  const ciphertext = data.subarray(SALT_LEN + NONCE_LEN, data.length - AUTH_TAG_LEN);
  const authTag    = data.subarray(data.length - AUTH_TAG_LEN);

  const key = scryptSync(passphrase, salt, KEY_LEN, { N: SCRYPT_N, r: SCRYPT_R, p: SCRYPT_P });

  const decipher = createDecipheriv("aes-256-gcm", key, nonce);
  decipher.setAuthTag(authTag);

  let plaintext;
  try {
    plaintext = Buffer.concat([decipher.update(ciphertext), decipher.final()]);
  } catch {
    console.error("エラー: 復号に失敗しました。パスフレーズが違うか、ファイルが壊れています。");
    process.exit(1);
  }

  process.stdout.write(plaintext);
}

main();
