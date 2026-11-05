import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";

export default defineConfig({
  plugins: [react()],
  resolve: { alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) } },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
    coverage: {
      provider: "v8",
      reporter: ["text", "html"],
      include: [
        "src/features/**/logic/**",
        "src/features/**/*.schema.ts",
        "src/lib/api/hooks/use-job-progress.ts",
        "src/lib/auth/token-store.ts",
        "src/i18n/direction.ts",
      ],
    },
  },
});
