/**
 * Tool Response Compactor - System Integration
 * 
 * This module provides a wrapper to enforce compaction on ALL tool outputs
 * before they enter working context.
 */

const { compactToolResponse, normalizeCompactorResult } = require('./tool_response_compactor.ts');
declare function execAsync(cmd: string): Promise<string>;

/**
 * Wraps any tool output through the compactor
 * Use this for ALL exec, web_fetch, read tool results
 */
function processToolOutput(toolName, rawOutput) {
  const normalizedToolName = String(toolName || 'unknown');
  const result = normalizeCompactorResult(
    compactToolResponse(rawOutput, { toolName: normalizedToolName }),
    String(rawOutput || '')
  );
  
  // Log metrics for tracking
  if (result.metrics && result.compacted) {
    console.error(
      `[COMPACTOR] ${normalizedToolName}: ${result.metrics.originalChars} → ${result.metrics.compactedChars} chars (${result.metrics.savingsPercent}% saved)`
    );
  }
  
  return result.content;
}

/**
 * Example usage for Moltbook API calls
 */
async function fetchMoltbookFeed() {
  // Simulated - replace with actual curl/exec call
  const rawResponse = await execAsync('curl -s ...');
  return processToolOutput('moltbook_feed', rawResponse);
}

module.exports = {
  processToolOutput,
  fetchMoltbookFeed
};

// Patch instructions for Infring integration:
// 1. Import this module in your tool wrapper
// 2. Call processToolOutput(toolName, rawResult) before returning to context
// 3. All outputs >1200 chars or >40 lines will be compacted automatically
export {};
