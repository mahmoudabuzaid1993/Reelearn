import { z } from "zod";

export const invitePersonSchema = z
  .object({
    full_name: z.string().trim().min(2, "required"),
    role: z.enum(["admin", "manager", "sme", "learner"]),
    email: z.string().trim().email("email").or(z.literal("")),
    phone: z
      .string()
      .trim()
      .refine((v) => !v || /^(\+9665\d{8}|05\d{8}|\+\d{8,15})$/.test(v.replace(/\s/g, "")), "phone"),
    employee_no: z.string().trim().optional(),
    preferred_language: z.enum(["ar", "en", "ur", "bn", "hi", "tl"]),
    location: z.string().trim().optional(),
    job_title: z.string().trim().optional(),
    delivery_channel: z.enum(["app", "whatsapp", "both", "teams"]),
    group_ids: z.array(z.string()),
  })
  .refine((v) => v.email || v.phone, { message: "contact", path: ["phone"] });

export type InvitePersonInput = z.infer<typeof invitePersonSchema>;

export const groupSchema = z.object({
  name: z.string().trim().min(2, "required"),
  kind: z.enum(["team", "location", "role", "custom"]),
});
export type GroupInput = z.infer<typeof groupSchema>;
