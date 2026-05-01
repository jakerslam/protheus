// Chat response text cleanup and formatting helpers.
'use strict';

function infringChatResponseTextFormatMethods() {
  return {
    stripModelPrefix: function(text) {
      if (!text) return text;
      var out = String(text);
      var lowered = out.toLowerCase();
      var recallIdx = lowered.indexOf('recalled context:');
      if (recallIdx >= 0) {
        var prefix = lowered.slice(0, recallIdx);
        var looksLikeMemoryMeta = prefix.indexOf('persistent memory') >= 0 ||
          prefix.indexOf('stored messages') >= 0 ||
          prefix.indexOf('session(s)') >= 0 ||
          prefix.indexOf(' sessions') >= 0;
        if (looksLikeMemoryMeta) {
          var leakedTail = out.slice(recallIdx + 'recalled context:'.length).trim();
          var finalIdx = leakedTail.toLowerCase().indexOf('final answer:');
          if (finalIdx >= 0) {
            out = leakedTail.slice(finalIdx + 'final answer:'.length).trim();
          } else {
            out = '';
          }
        }
      }
      if (/persistent memory is enabled for this agent across/i.test(out)) {
        var finalAnswerMatch = out.match(/(?:^|\n)\s*final answer\s*:\s*/i);
        if (finalAnswerMatch && Number.isFinite(Number(finalAnswerMatch.index))) {
          out = out.slice(Number(finalAnswerMatch.index) + String(finalAnswerMatch[0] || '').length).trim();
        } else {
          var strippedLines = out.split(/\r?\n/).filter(function(line) {
            var value = String(line || '').trim().toLowerCase();
            if (!value) return false;
            if (/^e2e-\d+-res$/.test(value)) return false;
            if (value.indexOf('persistent memory is enabled for this agent across') === 0) return false;
            if (value.indexOf('recalled context:') === 0) return false;
            if (value.indexOf('stored messages') >= 0) return false;
            return true;
          });
          out = strippedLines.join('\n').trim();
        }
      }
      for (var i = 0; i < 6; i++) {
        var prior = out;
        out = out.replace(/^\s*\[[^\]\n]{2,96}\]\s*/, '');
        // Strip leaked transcript wrappers like "User: ... Agent: <answer>".
        var transcriptLead = out.match(
          /^\s*(?:[-*]\s*)?(?:\*\*)?(?:user|human|you)(?:\*\*)?\s*:\s*[\s\S]{0,1200}?(?:\*\*)?(?:agent|assistant|model|ai|jarvis)(?:\*\*)?\s*:\s*/i
        );
        if (transcriptLead && transcriptLead[0]) {
          out = out.slice(transcriptLead[0].length);
          continue;
        }
        out = out.replace(
          /^\s*(?:[-*]\s*)?(?:\*\*)?(?:agent|assistant|system|model|ai|jarvis|user|human|you)(?:\*\*)?\s*:\s*/i,
          ''
        );
        if (out === prior) break;
      }
      out = out.replace(/^e2e-\d+-res\s*/i, '').trim();
      out = out.replace(
        /\s*i could not produce a final answer this turn\.\s*please retry or clarify what you want next\.\s*/ig,
        '\n'
      ).trim();
      return out;
    },

    formatToolJson: function(text) {
      if (!text) return '';
      try { return JSON.stringify(JSON.parse(text), null, 2); }
      catch(e) { return text; }
    },
  };
}
