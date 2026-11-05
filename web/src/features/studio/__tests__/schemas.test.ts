import { describe, expect, it } from "vitest";
import { signInSchema, signUpSchema, suggestSlug } from "@/features/auth/auth.schema";
import { invitePersonSchema } from "@/features/people/people.schema";
import { cutSchema, generationSchema, kindForFile, publishSchema, reelEditSchema, sourceSchema } from "../studio.schema";

describe("zod schemas", () => {
  it("sign-in requires a slug and an email", () => {
    expect(signInSchema.safeParse({ tenant_slug: "alwadi", email: "admin@alwadi.example" }).success).toBe(true);
    expect(signInSchema.safeParse({ tenant_slug: "Al Wadi", email: "nope" }).success).toBe(false);
  });

  it("sign-up validates slug, email and Saudi phone", () => {
    const ok = { company_name: "Al Wadi Retail", slug: "alwadi", admin_name: "Noura", admin_email: "admin@alwadi.example", admin_phone: "0551234567", default_language: "ar", region: "ksa" };
    expect(signUpSchema.safeParse(ok).success).toBe(true);
    expect(signUpSchema.safeParse({ ...ok, admin_phone: "123" }).success).toBe(false);
    expect(signUpSchema.safeParse({ ...ok, region: "us" }).success).toBe(false);
    expect(suggestSlug("Al Wadi Retail")).toBe("al-wadi-retail");
  });

  it("invite needs a phone or an email", () => {
    const base = { full_name: "Sara", role: "learner", email: "", phone: "", preferred_language: "ar", delivery_channel: "app", group_ids: [] };
    expect(invitePersonSchema.safeParse(base).success).toBe(false);
    expect(invitePersonSchema.safeParse({ ...base, phone: "0551234567" }).success).toBe(true);
    expect(invitePersonSchema.safeParse({ ...base, email: "sara@alwadi.example" }).success).toBe(true);
  });

  it("source: youtube needs a youtube link; files need an asset id", () => {
    expect(sourceSchema.safeParse({ kind: "youtube", title: "Talk", url: "https://example.com/v" }).success).toBe(false);
    expect(sourceSchema.safeParse({ kind: "youtube", title: "Talk", url: "https://youtu.be/abc" }).success).toBe(true);
    expect(sourceSchema.safeParse({ kind: "pdf", title: "Handbook", media_asset_id: "" }).success).toBe(false);
    expect(sourceSchema.safeParse({ kind: "topic_prompt", title: "Ramadan hours", prompt: "short" }).success).toBe(false);
    expect(kindForFile({ type: "application/pdf", name: "x.pdf" })).toBe("pdf");
    expect(kindForFile({ type: "", name: "ep12.m4a" })).toBe("podcast_audio");
    expect(kindForFile({ type: "text/plain", name: "x.txt" })).toBeNull();
  });

  it("generation: 1–12 reels and at least one language", () => {
    const base = { source_id: "s", topic_id: "t", languages: ["ar"], reel_count: 3, style: "ai_avatar" };
    expect(generationSchema.safeParse(base).success).toBe(true);
    expect(generationSchema.safeParse({ ...base, reel_count: 13 }).success).toBe(false);
    expect(generationSchema.safeParse({ ...base, languages: [] }).success).toBe(false);
  });

  it("reel edit: script bounds and three options", () => {
    const q = { prompt: "When do you use her name?", options: [{ key: "A", text: "a" }, { key: "B", text: "b" }, { key: "C", text: "c" }], correct_key: "B" };
    expect(reelEditSchema.safeParse({ title: "Names", script: "x".repeat(30), question: q }).success).toBe(true);
    expect(reelEditSchema.safeParse({ title: "Names", script: "x".repeat(901), question: q }).success).toBe(false);
    expect(reelEditSchema.safeParse({ title: "Names", script: "x".repeat(30), question: { ...q, options: q.options.slice(0, 2) } }).success).toBe(false);
  });

  it("publish needs a group and a date; cuts need consent and 10–90 s", () => {
    expect(publishSchema.safeParse({ group_ids: [], starts_on: "2026-11-01" }).success).toBe(false);
    expect(publishSchema.safeParse({ group_ids: ["g"], starts_on: "2026-11-01" }).success).toBe(true);
    expect(cutSchema.safeParse({ topic_id: "t", title: "Cone first", start_ms: 0, end_ms: 30000, speaker_consent: false }).success).toBe(false);
    expect(cutSchema.safeParse({ topic_id: "t", title: "Cone first", start_ms: 0, end_ms: 5000, speaker_consent: true }).success).toBe(false);
    expect(cutSchema.safeParse({ topic_id: "t", title: "Cone first", start_ms: 0, end_ms: 30000, speaker_consent: true }).success).toBe(true);
  });
});
