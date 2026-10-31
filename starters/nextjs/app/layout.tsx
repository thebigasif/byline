// Every page of the blog: a light, readable column for the text.
import type { ReactNode } from "react";
import { getSettings, siteFrom } from "../lib/khalid.js";

export default async function Layout({ children }: { children: ReactNode }) {
  const settings = await getSettings(siteFrom(process.env));
  return (
    <html lang={settings.default_language}>
      <body
        style={{
          margin: 0,
          background: "#ffffff",
          color: "#1f2328",
          font: "1.0625rem/1.6 system-ui, sans-serif",
        }}
      >
        <main style={{ maxWidth: "42rem", margin: "0 auto", padding: "2rem 1rem" }}>
          {children}
        </main>
      </body>
    </html>
  );
}
