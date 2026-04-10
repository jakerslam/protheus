const toolResponseCompactor = require('./tool_response_compactor.ts');
const { compactToolResponse, normalizeCompactorResult } = toolResponseCompactor as {
  compactToolResponse: (data: string, options?: { toolName?: string }) => any;
  normalizeCompactorResult: (result: any, fallbackContent: string) => {
    compacted: boolean;
    content: string;
    metrics: unknown;
  };
};

function extractRawPathFromContent(content: unknown): string | null {
  const txt = String(content || '');
  const m = txt.match(/📁 Raw output saved to:\s*([^\n]+)/);
  return m ? String(m[1] || '').trim() : null;
}

function compactCommandOutput(rawText: unknown, toolName: unknown): {
  text: string;
  compacted: boolean;
  raw_path: string | null;
  metrics: unknown;
} {
  const normalizedRawText = String(rawText || '');
  const normalizedToolName = String(toolName || 'command_output');
  const result = normalizeCompactorResult(
    compactToolResponse(normalizedRawText, { toolName: normalizedToolName }),
    normalizedRawText
  );
  const rawPathFromContent = extractRawPathFromContent(result.content);
  return {
    text: result.content,
    compacted: result.compacted === true,
    raw_path: rawPathFromContent || null,
    metrics: result.metrics || null
  };
}

export {
  compactCommandOutput,
  extractRawPathFromContent
};
