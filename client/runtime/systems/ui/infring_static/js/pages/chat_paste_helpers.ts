'use strict';

function chatEstimateTokenCountFromText(text) {
  return Math.max(0, Math.round(String(text || '').length / 4));
}

function chatShouldConvertLargePasteToAttachment(page, rawText) {
  if (!page.pasteToMarkdownEnabled) return false;
  var text = String(rawText == null ? '' : rawText);
  if (!text.trim()) return false;
  var chars = text.trim().length;
  var lines = text.split(/\r?\n/g).length;
  var charThreshold = Number(page.pasteToMarkdownCharThreshold || 2000);
  var lineThreshold = Number(page.pasteToMarkdownLineThreshold || 40);
  if (!Number.isFinite(charThreshold) || charThreshold < 256) charThreshold = 2000;
  if (!Number.isFinite(lineThreshold) || lineThreshold < 8) lineThreshold = 40;
  return chars >= charThreshold || lines >= lineThreshold;
}

function chatBuildLargePasteMarkdownAttachment(rawText) {
  if (typeof File !== 'function') return null;
  var text = String(rawText == null ? '' : rawText);
  if (!text.trim()) return null;
  var normalized = text.replace(/\r\n?/g, '\n');
  try {
    var file = new File([normalized], 'Pasted markdown.md', {
      type: 'text/markdown;charset=utf-8',
      lastModified: Date.now()
    });
    return { file: file, preview: '', uploading: false, pasted_markdown: true };
  } catch (_) {
    return null;
  }
}

function infringChatPasteDelegateMethods() {
  return {
    shouldConvertLargePasteToAttachment(rawText) {
      return chatShouldConvertLargePasteToAttachment(this, rawText);
    },

    buildLargePasteMarkdownAttachment(rawText) {
      return chatBuildLargePasteMarkdownAttachment(rawText);
    },
  };
}
