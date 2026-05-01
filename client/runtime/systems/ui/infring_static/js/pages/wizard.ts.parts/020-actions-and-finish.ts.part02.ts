    finish() {
      localStorage.setItem('infring-onboarded', 'true');
      var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
        ? InfringSharedShellServices.appStore
        : null;
      if (bridge && typeof bridge.set === 'function') bridge.set('showOnboarding', false);
      // Navigate to agents with chat if an agent was created, otherwise overview
      if (this.createdAgent) {
        var agent = this.createdAgent;
        if (bridge && typeof bridge.set === 'function') {
          bridge.set('pendingAgent', { id: agent.id, name: agent.name, model_provider: '?', model_name: '?' });
        }
        window.location.hash = 'agents';
      } else {
        window.location.hash = 'overview';
      }
    },

    finishAndDismiss() {
      localStorage.setItem('infring-onboarded', 'true');
      var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
        ? InfringSharedShellServices.appStore
        : null;
      if (bridge && typeof bridge.set === 'function') bridge.set('showOnboarding', false);
      window.location.hash = 'overview';
    }
  };
}
