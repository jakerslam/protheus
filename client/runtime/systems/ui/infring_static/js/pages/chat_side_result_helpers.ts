// Chat side-result parsing and final-event helper methods.
'use strict';

function infringChatSideResultMethods() {
  return {
    thinkingDisplayText(msg) {
      var rawThought = String(msg && msg._thoughtText ? msg._thoughtText : '').trim();
      if (!rawThought) return '';
      var latestComplete = typeof this.nextThoughtSentenceFrame === 'function'
        ? String(this.nextThoughtSentenceFrame(msg, rawThought) || '').trim()
        : '';
      if (!latestComplete && typeof this.latestCompleteSentence === 'function') {
        latestComplete = String(this.latestCompleteSentence(rawThought) || '').trim();
      }
      if (latestComplete) {
        if (msg && typeof msg === 'object') msg._thought_last_complete_sentence = latestComplete;
        return latestComplete;
      }
      var sticky = String(msg && msg._thought_last_complete_sentence ? msg._thought_last_complete_sentence : '').trim();
      if (sticky) return sticky;
      return '';
    },
    shouldReloadHistoryForFinalEventPayload(payload) {
      return !!(
        payload &&
        typeof payload === 'object' &&
        String(payload.state || '').trim().toLowerCase() === 'final'
      );
    },
    parseChatSideResult(payload) {
      if (!payload || typeof payload !== 'object') return null;
      var candidate = payload;
      if (candidate.kind !== 'btw') return null;
      var runId = String(candidate.runId || '').trim();
      var sessionKey = String(candidate.sessionKey || '').trim();
      var question = String(candidate.question || '').trim();
      var text = String(candidate.text || '').trim();
      if (!(runId && sessionKey && question && text)) return null;
      return {
        kind: 'btw',
        runId: runId,
        sessionKey: sessionKey,
        question: question,
        text: text,
        isError: candidate.isError === true,
        ts:
          typeof candidate.ts === 'number' && Number.isFinite(candidate.ts)
            ? candidate.ts
            : Date.now()
      };
    },
  };
}
