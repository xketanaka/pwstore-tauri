import { Entry } from "./api.ts";

/**
 * スペース区切りの複数キーワードでエントリを絞り込み、優先度順にソートして返す。
 * 優先度: サービス名マッチ(0) > keywordフィールドマッチ(1) > アカウントマッチ(2)
 * 全キーワードがいずれかのフィールドにマッチしたエントリのみ返す。
 */
export function filterAndRankEntries(entries: Entry[], keyword: string): Entry[] {
  const keywords = keyword.toLowerCase().split(/\s+/).filter(Boolean);
  if (keywords.length === 0) return entries;

  const rankEntry = (e: Entry): number => {
    const sn = e.service_name.toLowerCase();
    const kw = e.keyword.toLowerCase();
    const ac = e.account.toLowerCase();
    const kwRank = (k: string) =>
      sn.includes(k) ? 0 : kw.includes(k) ? 1 : ac.includes(k) ? 2 : 3;
    return Math.max(...keywords.map(kwRank));
  };

  return entries.filter((e) => rankEntry(e) < 3).sort((a, b) => rankEntry(a) - rankEntry(b));
}
