// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
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
          var rawSendErrorText = String(rawHttpError || (e && e.message) || '').replace(/\s+/g, ' ').trim();
          var lowerSendErrorText = rawSendErrorText.toLowerCase();
          var isTransientDisconnectError =
            lowerSendErrorText === 'fetch failed' ||
            lowerSendErrorText === 'failed to fetch' ||
            lowerSendErrorText === 'connect failed' ||
            lowerSendErrorText.indexOf('gateway connect failed') >= 0;
          var normalizedSendErrorText = (function(message) {
            var raw = String(message || '').replace(/\s+/g, ' ').trim();
            var lower = raw.toLowerCase();
            if (!raw || lower === 'unknown error') return 'Connection failed before the runtime returned a usable response. Try again after the gateway is reachable.';
            if (lower.indexOf('pairing required') >= 0) return 'Gateway pairing is required. Open Settings, pair this dashboard with the gateway, then try again.';
            if (
              lower.indexOf('device identity required') >= 0 ||
              lower.indexOf('secure context') >= 0 ||
              lower.indexOf('https/localhost') >= 0
            ) return 'This action requires HTTPS or localhost. Reopen the dashboard from a trusted origin, then try again.';
            if (
              lower.indexOf('unauthorized') >= 0 ||
              lower.indexOf('token mismatch') >= 0 ||
              lower.indexOf('token missing') >= 0 ||
              lower.indexOf('auth failed') >= 0 ||
              lower.indexOf('authentication') >= 0
            ) return 'Gateway authentication failed. Verify the API token or password in Settings, then retry.';
            if (
              lower === 'fetch failed' ||
              lower === 'failed to fetch' ||
              lower === 'connect failed' ||
              lower.indexOf('gateway connect failed') >= 0
            ) return 'Gateway connect failed. Check runtime availability, pairing, and auth settings, then retry.';
            return 'Connection error: ' + raw;
          })(rawHttpError || (e && e.message) || '');
          if (!isTransientDisconnectError) {
            this.pushSystemMessage({
              text: normalizedSendErrorText,
              meta: '',
              tools: [],
              system_origin: 'http:error',
              ts: Date.now(),
              dedupe_window_ms: 12000
            });
          }
          this._inflightPayload = null;
        } else {
          return;
