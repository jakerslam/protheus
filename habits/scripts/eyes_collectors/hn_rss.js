/**
 * habits/scripts/eyes_collectors/hn_rss.js
 *
 * Deterministic HN RSS collector.
 * - Fetches RSS feed via hnrss.org (stable + simple)
 * - Emits items with: collected_at, id(item_hash), url, title, topics, bytes
 * - NO LLM usage, NO HTML parsing, minimal dependencies
 */

const https = require("https");
const crypto = require("crypto");

function sha16(s) {
  return crypto.createHash("sha256").update(String(s)).digest("hex").slice(0, 16);
}

function nowIso() {
  return new Date().toISOString();
}

function fetchText(url, timeoutMs = 8000) {
  return new Promise((resolve, reject) => {
    const req = https.get(url, { headers: { "User-Agent": "openclaw-eyes/1.0" } }, (res) => {
      if (res.statusCode && res.statusCode >= 400) {
        reject(new Error(`HTTP ${res.statusCode} for ${url}`));
        res.resume();
        return;
      }
      let bytes = 0;
      const chunks = [];
      res.on("data", (d) => {
        bytes += d.length;
        chunks.push(d);
      });
      res.on("end", () => resolve({ text: Buffer.concat(chunks).toString("utf8"), bytes }));
    });
    req.on("error", reject);
    req.setTimeout(timeoutMs, () => {
      req.destroy(new Error(`Timeout after ${timeoutMs}ms for ${url}`));
    });
  });
}

function stripCdata(s) {
  return String(s || "").replace("<![CDATA[", "").replace("]]>", "").trim();
}

function decodeXmlEntities(s) {
  // Minimal decode for common RSS cases (deterministic + tiny)
  return String(s || "")
    .replace(/&amp;/g, "&")
    .replace(/&quot;/g, '"')
    .replace(/&apos;/g, "'")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">");
}

function extractTag(xml, tag) {
  const re = new RegExp(`<${tag}>([\\s\\S]*?)</${tag}>`, "i");
  const m = xml.match(re);
  return m ? decodeXmlEntities(stripCdata(m[1])) : "";
}

function splitItems(rssXml) {
  // Very basic RSS item splitting (works for hnrss.org output)
  return String(rssXml).split(/<item>/i).slice(1).map((chunk) => chunk.split(/<\/item>/i)[0]);
}

function keywordTopics(title, configuredTopics = []) {
  const t = String(title || "").toLowerCase();
  const out = new Set();

  // Keep configured topics if present
  for (const ct of configuredTopics) out.add(ct);

  // Lightweight signals
  if (t.includes("agent")) out.add("ai_agents");
  if (t.includes("llm") || t.includes("gpt") || t.includes("transformer")) out.add("llm");
  if (t.includes("automation") || t.includes("workflow") || t.includes("orchestration")) out.add("automation");
  if (t.includes("tool") || t.includes("sdk") || t.includes("cli") || t.includes("library")) out.add("devtools");

  // Return up to 5 to keep payload small/deterministic
  return Array.from(out).slice(0, 5);
}

async function collectHnRss(eyeConfig, budgets) {
  const started = Date.now();

  // hnrss supports many endpoints; frontpage is stable:
  // https://hnrss.org/frontpage
  const feedUrl = "https://hnrss.org/frontpage";
  const { text, bytes } = await fetchText(feedUrl, Math.min(9000, (budgets?.max_seconds || 10) * 1000));

  const itemsRaw = splitItems(text);
  const maxItems = Math.max(1, Math.min(budgets?.max_items || 20, 50));

  const items = [];
  for (const it of itemsRaw.slice(0, maxItems)) {
    const title = extractTag(it, "title");
    const url = extractTag(it, "link");
    if (!title || !url) continue;

    const item_hash = sha16(url);
    items.push({
      collected_at: nowIso(),
      id: item_hash,
      url,
      title,
      topics: keywordTopics(title, Array.isArray(eyeConfig?.topics) ? eyeConfig.topics : []),
      bytes: Math.min(512, title.length + url.length + 64)
    });
  }

  const duration_ms = Date.now() - started;
  return {
    success: true,
    items,
    duration_ms,
    requests: 1,
    bytes
  };
}

module.exports = { collectHnRss };
