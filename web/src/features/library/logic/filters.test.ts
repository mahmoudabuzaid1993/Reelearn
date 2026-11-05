import { describe, expect, it } from "vitest";
import type { ReelListItem } from "@/lib/api/types";
import { applyLibraryFilters, parseFilters, toSearchParams } from "./filters";

const item = (over: Partial<ReelListItem>): ReelListItem => ({ id: "r", topic: { id: "t1", name: "Store safety" }, kind: "ai_avatar", status: "published", title: "Spill", language: "ar", languages: ["ar", "ur"], version: 1, updated_at: "2026-10-01T00:00:00Z", ...over });

describe("library filters", () => {
  it("filters by status, topic, language, kind and query", () => {
    const items = [item({ id: "a" }), item({ id: "b", status: "draft", kind: "motion", language: "en", languages: ["en"], title: "Cash", topic: { id: "t2", name: "AML" } })];
    expect(applyLibraryFilters(items, { status: "draft" }).map((i) => i.id)).toEqual(["b"]);
    expect(applyLibraryFilters(items, { language: "ur" }).map((i) => i.id)).toEqual(["a"]);
    expect(applyLibraryFilters(items, { q: "aml" }).map((i) => i.id)).toEqual(["b"]);
    expect(applyLibraryFilters(items, { kind: "ai_avatar", topic_id: "t1" }).map((i) => i.id)).toEqual(["a"]);
  });

  it("round-trips URL params", () => {
    const f = parseFilters(new URLSearchParams("status=in_review&q=cone&kind="));
    expect(f).toEqual({ status: "in_review", topic_id: undefined, language: undefined, kind: undefined, q: "cone" });
    expect(toSearchParams(f).toString()).toBe("status=in_review&q=cone");
  });
});
