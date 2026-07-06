import { describe, it, expect } from "vitest";
import { filterAndRankEntries } from "./entryFilter.ts";
import type { Entry } from "./api.ts";

function makeEntry(id: number, service: string, keyword: string, account: string): Entry {
  return {
    id, service_name: service, keyword, account,
    password: "", category: "", status: 1, extra_fields: [],
  };
}

const entries = [
  makeEntry(1, "AWS",    "cloud infra", "alice@example.com"),
  makeEntry(2, "GitHub", "git code",    "bob@example.com"),
  makeEntry(3, "Google", "search mail", "carol@example.com"),
];

describe("filterAndRankEntries", () => {
  it("空キーワードは全件返す", () => {
    expect(filterAndRankEntries(entries, "")).toHaveLength(3);
  });

  it("サービス名にマッチするものを返す", () => {
    const result = filterAndRankEntries(entries, "aws");
    expect(result).toHaveLength(1);
    expect(result[0].service_name).toBe("AWS");
  });

  it("キーワードフィールドにマッチするものを返す", () => {
    const result = filterAndRankEntries(entries, "infra");
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe(1);
  });

  it("アカウントにマッチするものを返す", () => {
    const result = filterAndRankEntries(entries, "carol");
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe(3);
  });

  it("マッチなしは空配列", () => {
    expect(filterAndRankEntries(entries, "zzz")).toHaveLength(0);
  });

  it("大文字小文字を区別しない", () => {
    expect(filterAndRankEntries(entries, "GITHUB")).toHaveLength(1);
  });

  it("複数キーワードのAND検索", () => {
    const result = filterAndRankEntries(entries, "aws alice");
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe(1);
  });

  it("複数キーワード: 片方がサービス名、もう片方がアカウント名にマッチ", () => {
    const result = filterAndRankEntries(entries, "github bob");
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe(2);
  });

  it("複数キーワード: どちらかがマッチしないエントリは除外", () => {
    const result = filterAndRankEntries(entries, "aws bob");
    expect(result).toHaveLength(0);
  });

  it("両方のキーワードがサービス名にマッチ", () => {
    const e = [makeEntry(99, "Amazon Web Service", "", "user")];
    expect(filterAndRankEntries(e, "Amazon Service")).toHaveLength(1);
  });

  it("サービス名マッチが先頭に来る", () => {
    const mixed = [
      makeEntry(10, "infra-tool", "cloud", "user"),
      makeEntry(11, "AWS",        "cloud", "user"),
    ];
    const result = filterAndRankEntries(mixed, "infra");
    expect(result[0].service_name).toBe("infra-tool");
  });
});
