import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const styleRel = "shell/browser-v2/browser_shell_v2.css";
const outRel = "core/local/artifacts/browser_shell_v2_legacy_visual_parity_guard_current.json";
const stylePath = path.join(root, styleRel);
const outPath = path.join(root, outRel);

const maxCssLines = 500;
const requiredLegacyTokens = [
  "--bg",
  "--bg-primary",
  "--chrome-bg",
  "--sidebar-bg",
  "--surface",
  "--surface2",
  "--surface3",
  "--border",
  "--border-light",
  "--text",
  "--text-secondary",
  "--text-dim",
  "--text-muted",
  "--accent",
  "--accent-subtle",
  "--agent-bg",
  "--user-bg",
  "--font-sans",
  "--font-mono",
];
const forbiddenCustomSkinMarkers = [
  "radial-gradient(circle at 15% 10%",
  "#fff6df",
  "#ccb58e",
  "#1c6d58",
  "#ffe3b8",
  "#dcecff",
  "#d8ebff",
  "backdrop-filter",
];
const requiredLegacyLayoutMarkers = [
  "var(--sidebar-width",
  "48px",
  "var(--radius-lg",
  "var(--agent-bg",
  "var(--user-bg",
];

function countLines(text: string): number {
  return text.split(/\r?\n/).length;
}

const css = fs.readFileSync(stylePath, "utf8");
const lineCount = countLines(css);
const violations: string[] = [];

if (lineCount > maxCssLines) {
  violations.push(`css_file_over_cap:${lineCount}>${maxCssLines}`);
}
for (const token of requiredLegacyTokens) {
  if (!css.includes(`var(${token}`)) violations.push(`missing_legacy_token:${token}`);
}
for (const marker of forbiddenCustomSkinMarkers) {
  if (css.includes(marker)) violations.push(`forbidden_custom_skin_marker:${marker}`);
}
for (const marker of requiredLegacyLayoutMarkers) {
  if (!css.includes(marker)) violations.push(`missing_legacy_layout_marker:${marker}`);
}

const payload = {
  trace_id: `validation:${new Date().toISOString()}:browser-shell-v2-legacy-visual-parity`,
  source_domain: "validation",
  ok: violations.length === 0,
  type: "browser_shell_v2_legacy_visual_parity_guard",
  generated_at: new Date().toISOString(),
  style_path: styleRel,
  css_line_count: lineCount,
  max_css_lines: maxCssLines,
  violations,
};

fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (violations.length > 0) process.exitCode = 1;
