// Chat slash command handler for API key and local model discovery.
'use strict';

function infringChatSlashApiKeyMethods() {
  return {
    runSlashApiKeyDiscovery: async function(cmdArgs) {
      if (!cmdArgs || !String(cmdArgs).trim()) {
        InfringToast.info('Usage: /apikey <api-key-or-local-model-path>');
        return;
      }
      try {
        var discoveryInput = String(cmdArgs || '').trim();
        this.inputText = 'Use the model_provider_coordination route to discover or index this API key or local model path exactly: ' + discoveryInput;
        await this.sendMessage();
      } catch (eApikey) {
        InfringToast.error('API key/model path request failed: ' + (eApikey && eApikey.message ? eApikey.message : eApikey));
      }
    },
  };
}
