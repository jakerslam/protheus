          this._inflightPayload = null;
          this.refreshPromptSuggestions(true, 'post-http-abort');
          this.scheduleConversationPersist();
          return;
        }
        if (
          !opts.retry_from_agent_rebind &&
          (lowerHttpError.indexOf('agent_not_found') >= 0 || lowerHttpError.indexOf('agent not found') >= 0)
        ) {
          var reboundAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
          if (!reboundAgent || String(reboundAgent.id || '') === String(targetAgentId || '')) {
            reboundAgent = await this.rebindCurrentAgentAuthoritative({
              preferred_id: targetAgentId,
              clear_when_missing: true
            });
          }
          var reboundAgentId = reboundAgent && reboundAgent.id ? String(reboundAgent.id) : '';
          if (reboundAgentId && reboundAgentId !== targetAgentId) {
            this.addNoticeEvent({
              notice_label:
                'Active agent reference expired. Switched to ' +
                String(reboundAgent.name || reboundAgent.id || reboundAgentId) +
                ' and retried.',
              notice_type: 'warn',
              ts: Date.now(),
            });
            await this._sendPayload(finalText, uploadedFiles, msgImages, {
              agent_id: reboundAgentId,
              retry_from_agent_rebind: true,
            });
            return;
          }
        }
        var noModelsError =
          lowerHttpError.indexOf('no_models_available') >= 0 ||
          lowerHttpError.indexOf('no models available') >= 0;
        if (noModelsError) {
          this.injectNoModelsGuidance('send_error');
          this._inflightPayload = null;
          this.scheduleConversationPersist();
          return;
        }
        handedOffToRecovery = await this.attemptAutomaticFailoverRecovery(
          'http_error',
          rawHttpError,
          { remove_last_agent_failure: false }
        );
        if (!handedOffToRecovery) {
          this.pushSystemMessage({
            text: 'Error: ' + e.message,
            meta: '',
            tools: [],
            system_origin: 'http:error',
            ts: Date.now(),
            dedupe_window_ms: 12000
          });
          this._inflightPayload = null;
        } else {
          return;
