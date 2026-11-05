import type { GenerationRequest, Quote } from "@/lib/api/types";

/** Credit pricing per step; the API is the source of truth, the console mirrors it for the mock and for previews. */
export const CREDIT_RATES = {
  script: 2, // per reel
  question: 1, // per reel
  render: { ai_avatar: 12, motion: 10, broll: 6 } as const, // per reel
  dub: 4, // per reel per extra language
  captions: 1, // per reel per language
} as const;

export type QuoteInput = Pick<GenerationRequest, "reel_count" | "languages" | "style">;

export interface QuoteLine {
  step: string;
  credits: number;
}

export function quoteLines(input: QuoteInput): QuoteLine[] {
  const reels = Math.max(0, Math.floor(input.reel_count));
  const languages = Math.max(1, new Set(input.languages).size);
  const extraLanguages = languages - 1;
  return [
    { step: "script", credits: reels * CREDIT_RATES.script },
    { step: "question", credits: reels * CREDIT_RATES.question },
    { step: "render", credits: reels * CREDIT_RATES.render[input.style] },
    { step: "dub", credits: reels * extraLanguages * CREDIT_RATES.dub },
    { step: "captions", credits: reels * languages * CREDIT_RATES.captions },
  ].filter((line) => line.credits > 0);
}

export function totalCredits(lines: QuoteLine[]): number {
  return lines.reduce((sum, line) => sum + line.credits, 0);
}

export function estimatedMinutes(input: QuoteInput): number {
  const perReel = input.style === "broll" ? 2 : 3;
  return Math.max(1, Math.ceil(input.reel_count * perReel * Math.max(1, input.languages.length) * 0.8));
}

export function buildQuote(input: QuoteInput, creditsAvailable: number): Quote {
  const breakdown = quoteLines(input);
  return {
    cost_credits: totalCredits(breakdown),
    credits_available: creditsAvailable,
    estimated_minutes: estimatedMinutes(input),
    breakdown,
  };
}

export function creditsAfter(quote: Pick<Quote, "cost_credits" | "credits_available">): number {
  return (quote.credits_available ?? 0) - (quote.cost_credits ?? 0);
}

export function isAffordable(quote: Pick<Quote, "cost_credits" | "credits_available">): boolean {
  return creditsAfter(quote) >= 0;
}

export function shortfall(quote: Pick<Quote, "cost_credits" | "credits_available">): number {
  return Math.max(0, -creditsAfter(quote));
}
