import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig, type Plugin } from "vite";

const panelRoot = fileURLToPath(new URL(".", import.meta.url));

function bundledLicenses(): Plugin {
  return {
    name: "demo-bundled-licenses",
    generateBundle(_, bundle) {
      const packages = new Set(["tailwindcss", "vite"]);

      for (const output of Object.values(bundle)) {
        if (output.type !== "chunk") continue;
        for (const moduleId of Object.keys(output.modules)) {
          const path = moduleId.split("/node_modules/").at(-1);
          if (!path || path === moduleId) continue;
          const segments = path.split("/");
          packages.add(segments[0].startsWith("@") ? segments.slice(0, 2).join("/") : segments[0]);
        }
      }

      const notices = [...packages].sort().flatMap((name) => {
        const dir = join(panelRoot, "node_modules", name);
        const files = readdirSync(dir)
          .filter((file) => /^(LICEN[CS]E|NOTICE|COPYRIGHTNOTICE)(\.(txt|md))?$/i.test(file))
          .sort();
        return files.map((file) => `=== ${name}/${file} ===\n\n${readFileSync(join(dir, file), "utf8").trim()}\n`);
      });

      this.emitFile({
        type: "asset",
        fileName: "third-party-licenses.txt",
        source: notices.join("\n").replace(/\r\n/g, "\n"),
      });
    },
  };
}

export default defineConfig({
  root: fileURLToPath(new URL("./demo/", import.meta.url)),
  base: "./",
  publicDir: false,
  plugins: [react(), tailwindcss(), bundledLicenses()],
  server: { fs: { allow: [panelRoot] } },
  build: {
    outDir: fileURLToPath(new URL("./dist-demo/", import.meta.url)),
    emptyOutDir: true,
  },
});
