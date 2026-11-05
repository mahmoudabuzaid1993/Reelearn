import { z } from "zod";

export const SOURCE_KINDS = ["pdf", "pptx", "image", "url", "youtube", "topic_prompt", "podcast_audio", "video"] as const;
export const MAX_UPLOAD_BYTES = 209_715_200;
export const ACCEPTED_MIME: Record<string, (typeof SOURCE_KINDS)[number]> = {
  "application/pdf": "pdf",
  "application/vnd.openxmlformats-officedocument.presentationml.presentation": "pptx",
  "image/png": "image",
  "image/jpeg": "image",
  "audio/mpeg": "podcast_audio",
  "audio/mp4": "podcast_audio",
  "audio/x-m4a": "podcast_audio",
  "audio/wav": "podcast_audio",
  "video/mp4": "video",
  "video/quicktime": "video",
};

export const sourceSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("url"), title: z.string().trim().min(2, "required"), url: z.string().trim().url("url"), language: z.string().optional() }),
  z.object({
    kind: z.literal("youtube"),
    title: z.string().trim().min(2, "required"),
    url: z
      .string()
      .trim()
      .url("url")
      .refine((u) => /youtube\.com|youtu\.be/.test(u), "youtube"),
    language: z.string().optional(),
  }),
  z.object({ kind: z.literal("topic_prompt"), title: z.string().trim().min(2, "required"), prompt: z.string().trim().min(20, "prompt"), language: z.string().optional() }),
  z.object({
    kind: z.enum(["pdf", "pptx", "image", "podcast_audio", "video"]),
    title: z.string().trim().min(2, "required"),
    media_asset_id: z.string().min(1, "file"),
    language: z.string().optional(),
  }),
]);
export type SourceInput = z.infer<typeof sourceSchema>;

export function kindForFile(file: { type: string; name: string }): (typeof SOURCE_KINDS)[number] | null {
  const byMime = ACCEPTED_MIME[file.type];
  if (byMime) return byMime;
  const ext = file.name.toLowerCase().split(".").pop();
  if (ext === "pdf") return "pdf";
  if (ext === "pptx") return "pptx";
  if (ext === "png" || ext === "jpg" || ext === "jpeg") return "image";
  if (ext === "mp3" || ext === "m4a" || ext === "wav") return "podcast_audio";
  if (ext === "mp4" || ext === "mov") return "video";
  return null;
}

export const generationSchema = z.object({
  source_id: z.string().min(1, "required"),
  topic_id: z.string().min(1, "required"),
  languages: z.array(z.string()).min(1, "languages"),
  reel_count: z.number().int().min(1).max(12),
  style: z.enum(["ai_avatar", "motion", "broll"]),
  presenter: z.object({ name: z.string().optional(), avatar_id: z.string().optional(), voice_id: z.string().optional() }).optional(),
  reviewer_id: z.string().optional(),
  brief: z.string().max(600).optional(),
});
export type GenerationInput = z.infer<typeof generationSchema>;

export const reelEditSchema = z.object({
  title: z.string().trim().min(3, "required"),
  script: z.string().trim().min(20, "script").max(900, "scriptMax"),
  source_citation: z.string().trim().optional(),
  compliance_tag: z.string().trim().optional(),
  presenter_voice: z.string().optional(),
  question: z.object({
    prompt: z.string().trim().min(5, "required"),
    options: z.array(z.object({ key: z.string(), text: z.string().trim().min(1, "required") })).length(3),
    correct_key: z.string().min(1, "required"),
    explanation: z.string().trim().optional(),
  }),
});
export type ReelEditInput = z.infer<typeof reelEditSchema>;

export const publishSchema = z.object({
  group_ids: z.array(z.string()).min(1, "audience"),
  starts_on: z.string().regex(/^\d{4}-\d{2}-\d{2}$/, "date"),
  compliance_tag: z.string().trim().optional(),
});
export type PublishInput = z.infer<typeof publishSchema>;

export const cutSchema = z
  .object({
    topic_id: z.string().min(1, "required"),
    title: z.string().trim().min(3, "required"),
    start_ms: z.number().int().min(0),
    end_ms: z.number().int().min(0),
    speaker_consent: z.literal(true, { message: "consent" }),
  })
  .refine((v) => v.end_ms - v.start_ms >= 10_000 && v.end_ms - v.start_ms <= 90_000, { message: "length", path: ["end_ms"] });
export type CutInput = z.infer<typeof cutSchema>;
