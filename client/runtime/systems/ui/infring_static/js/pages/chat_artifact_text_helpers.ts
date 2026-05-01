// Chat user-facing thought and artifact directive text helpers.
'use strict';

function infringChatArtifactTextMethods() {
  return {
    deriveUserFacingFromThought: function(thoughtText) {
      var thought = String(thoughtText || '').replace(/\s+/g, ' ').trim();
      if (!thought) return '';
      var skip = /^(alright|okay|ok|hmm|let me|i need to|i should|i will|first[, ]|to answer this|it seems|we need to)\b/i;
      var sentences = thought
        .split(/(?<=[.!?])\s+/)
        .map(function(part) { return String(part || '').trim(); })
        .filter(function(part) { return !!part; });
      var keep = [];
      for (var i = 0; i < sentences.length; i++) {
        var sentence = sentences[i];
        var lower = sentence.toLowerCase();
        if (skip.test(sentence) && lower.indexOf('queue depth') < 0 && lower.indexOf('scale') < 0 && lower.indexOf('recommend') < 0 && lower.indexOf('command') < 0) {
          continue;
        }
        if (lower.indexOf('user') >= 0 && lower.indexOf('request') >= 0) continue;
        if (sentence.length < 20) continue;
        keep.push(sentence);
      }
      if (!keep.length) {
        var queueLine = thought.match(/queue depth[^.?!]*[.?!]?/i);
        if (queueLine && queueLine[0]) keep.push(String(queueLine[0]).trim());
        var scaleLine = thought.match(/scale[^.?!]*instances?[^.?!]*[.?!]?/i);
        if (scaleLine && scaleLine[0]) keep.push(String(scaleLine[0]).trim());
      }
      if (!keep.length) return '';
      var message = keep.slice(0, 2).join(' ').replace(/\s+/g, ' ').trim();
      if (!message) return '';
      if (!/[.?!]$/.test(message)) message += '.';
      if (message.length > 300) message = message.slice(0, 297) + '...';
      return message;
    },
    extractArtifactDirectives: function(text) {
      var value = String(text || '');
      if (!value) return [];
      var rx = /\[\[\s*(file|folder)\s*:\s*([^\]]+?)\s*\]\]/gi;
      var out = [];
      var match;
      while ((match = rx.exec(value)) && out.length < 4) {
        var kind = String(match[1] || '').toLowerCase();
        var targetPath = String(match[2] || '').trim();
        if (!targetPath) continue;
        out.push({ kind: kind, path: targetPath });
      }
      return out;
    },
    stripArtifactDirectivesFromText: function(text) {
      var value = String(text || '');
      if (!value) return '';
      return value.replace(/\[\[\s*(file|folder)\s*:\s*[^\]]+?\s*\]\]/gi, '').replace(/\n{3,}/g, '\n\n').trim();
    },
  };
}
