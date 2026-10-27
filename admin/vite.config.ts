import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// The admin lives under /admin on the blog's own server, so every asset
// address starts there.
export default defineConfig({
  base: "/admin/",
  plugins: [react(), tailwindcss()],
  test: {
    environment: "jsdom",
    setupFiles: ["./test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
