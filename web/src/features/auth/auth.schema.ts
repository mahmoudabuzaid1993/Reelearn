import { z } from "zod";

export const signInSchema = z.object({
  tenant_slug: z
    .string()
    .trim()
    .min(2)
    .regex(/^[a-z0-9][a-z0-9-]{1,62}$/, "slug"),
  email: z.string().trim().email("email"),
});
export type SignInInput = z.infer<typeof signInSchema>;

export const otpSchema = z.object({
  code: z.string().trim().regex(/^\d{4,8}$/, "code"),
});
export type OtpInput = z.infer<typeof otpSchema>;

export const signUpSchema = z.object({
  company_name: z.string().trim().min(2, "company_name"),
  slug: z
    .string()
    .trim()
    .regex(/^[a-z0-9][a-z0-9-]{1,62}$/, "slug"),
  admin_name: z.string().trim().min(2, "admin_name"),
  admin_email: z.string().trim().email("email"),
  admin_phone: z
    .string()
    .trim()
    .optional()
    .refine((v) => !v || /^(\+9665\d{8}|05\d{8})$/.test(v.replace(/\s/g, "")), "phone"),
  default_language: z.enum(["ar", "en"]),
  region: z.enum(["ksa", "uae", "eu"]),
  sector: z.string().trim().optional(),
});
export type SignUpInput = z.infer<typeof signUpSchema>;

/** Turn a company name into a slug suggestion: "Al Wadi Retail" -> "al-wadi-retail". */
export function suggestSlug(companyName: string): string {
  return companyName
    .toLowerCase()
    .normalize("NFKD")
    .replace(/[^a-z0-9\s-]/g, "")
    .trim()
    .replace(/[\s-]+/g, "-")
    .slice(0, 63);
}
