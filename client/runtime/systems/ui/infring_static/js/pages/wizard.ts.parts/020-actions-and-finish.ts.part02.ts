    finish() {
      localStorage.setItem('infring-onboarded', 'true');
      Alpine.store('app').showOnboarding = false;
      // Navigate to agents with chat if an agent was created, otherwise overview
      if (this.createdAgent) {
        var agent = this.createdAgent;
        Alpine.store('app').pendingAgent = { id: agent.id, name: agent.name, model_provider: '?', model_name: '?' };
        window.location.hash = 'agents';
      } else {
        window.location.hash = 'overview';
      }
    },

    finishAndDismiss() {
      localStorage.setItem('infring-onboarded', 'true');
      Alpine.store('app').showOnboarding = false;
      window.location.hash = 'overview';
    }
  };
}

