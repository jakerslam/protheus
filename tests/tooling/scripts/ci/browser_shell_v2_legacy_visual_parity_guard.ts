import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const styleRel = "shell/browser-v2/browser_shell_v2.css";
const buildRel = "shell/browser-v2/browser_shell_v2_build.ts";
const artifactCssRel = "core/local/artifacts/browser_shell_v2_app/browser_shell_v2.css";
const outRel = "core/local/artifacts/browser_shell_v2_legacy_visual_parity_guard_current.json";
const stylePath = path.join(root, styleRel);
const buildPath = path.join(root, buildRel);
const artifactCssPath = path.join(root, artifactCssRel);
const outPath = path.join(root, outRel);

const maxCssLines = 5;
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
];
const requiredLegacyLayoutMarkers = [
  "var(--sidebar-width",
  "48px",
  "var(--radius-lg",
  "var(--agent-bg",
  "var(--user-bg",
];
const requiredLegacySurfaceMarkers = [
  "legacySurfaceCss",
  "theme.css",
  "layout.css.parts",
  "components.css.parts",
  "app-layout",
  "global-taskbar",
  "taskbar-reorder-box",
  "taskbar-reorder-item",
  "infring-taskbar-system-items-shell",
  "notif-wrap",
  "theme-opt",
  "bottom_dock_shell.bundle",
  "infring-bottom-dock-shell",
  "dock-icon-defs",
  "sidebar drag-bar overlay-shared-surface",
  "sidebar-pulltab",
  "infring-sidebar-agent-list-shell",
  "chat-wrapper",
  "infring-chat-header-shell",
  "chat-thread-profile-center",
  "chat-thread-profile-info-pill",
  "infring-messages-surface-shell",
  "messages",
  "message-bubble markdown-body",
  "chat-map",
  "infring-chat-input-footer-shell",
  "input-area",
  "input-row",
  "composer-display-pill",
  "composer-menu-pill",
  "composer-input-pill",
  "composer-controls-pill",
];
const forbiddenInventedSurfaceMarkers = [
  "browser-shell-v2__topbar",
  "browser-shell-v2__workspace",
  "browser-shell-v2__rail",
  "browser-shell-v2--legacy-surface",
  "Gateway Projection",
];

function countLines(text: string): number {
  return text.split(/\r?\n/).length;
}

const css = fs.readFileSync(stylePath, "utf8");
const build = fs.readFileSync(buildPath, "utf8");
const artifactCss = fs.existsSync(artifactCssPath) ? fs.readFileSync(artifactCssPath, "utf8") : "";
const visualSurface = `${css}\n${build}\n${artifactCss}`;
const lineCount = countLines(css);
const violations: string[] = [];
const legacyCssDir = ["client", "runtime", "systems", "ui", "infring" + "_static", "css"].join("/");
const legacyCssPaths = [
  "theme.css",
  ...fs.readdirSync(path.join(root, legacyCssDir, "layout.css.parts")).sort().map((name) => `layout.css.parts/${name}`),
  ...fs.readdirSync(path.join(root, legacyCssDir, "components.css.parts")).sort().map((name) => `components.css.parts/${name}`),
];
const expectedArtifactCss = legacyCssPaths
  .map((relPath) => fs.readFileSync(path.join(root, legacyCssDir, relPath), "utf8"))
  .join("");

if (lineCount > maxCssLines) {
  violations.push(`css_file_over_cap:${lineCount}>${maxCssLines}`);
}
if (!css.includes("Intentionally empty") || css.includes("{") || css.includes("}")) {
  violations.push("v2_css_must_not_define_visual_rules");
}
if (artifactCss !== expectedArtifactCss) {
  violations.push("artifact_css_not_exact_legacy_bundle");
}
for (const token of requiredLegacyTokens) {
  if (!visualSurface.includes(`var(${token}`) && !visualSurface.includes(`${token}:`)) violations.push(`missing_legacy_token:${token}`);
}
for (const marker of forbiddenCustomSkinMarkers) {
  if (visualSurface.includes(marker)) violations.push(`forbidden_custom_skin_marker:${marker}`);
}
for (const marker of requiredLegacyLayoutMarkers) {
  const cssVariableName = marker.match(/var\((--[^,)]+)/)?.[1];
  if (!visualSurface.includes(marker) && !(cssVariableName && visualSurface.includes(`${cssVariableName}:`))) violations.push(`missing_legacy_layout_marker:${marker}`);
}
for (const marker of requiredLegacySurfaceMarkers) {
  if (!visualSurface.includes(marker)) violations.push(`missing_legacy_surface_marker:${marker}`);
}
for (const marker of forbiddenInventedSurfaceMarkers) {
  if (build.includes(marker) || css.includes(marker)) violations.push(`forbidden_invented_surface_marker:${marker}`);
}

const payload = {
  trace_id: `validation:${new Date().toISOString()}:browser-shell-v2-legacy-visual-parity`,
  source_domain: "validation",
  ok: violations.length === 0,
  type: "browser_shell_v2_legacy_visual_parity_guard",
  generated_at: new Date().toISOString(),
  style_path: styleRel,
  build_path: buildRel,
  artifact_css_path: artifactCssRel,
  css_line_count: lineCount,
  max_css_lines: maxCssLines,
  violations,
};

fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (violations.length > 0) process.exitCode = 1;
