import { z } from "zod";

export const topicSchema = z.object({
  name: z.string().trim().min(2, "required"),
  name_ar: z.string().trim().optional(),
  description: z.string().trim().optional(),
  compliance_tag: z.string().trim().optional(),
  is_mandatory: z.boolean(),
});
export type TopicInput = z.infer<typeof topicSchema>;
