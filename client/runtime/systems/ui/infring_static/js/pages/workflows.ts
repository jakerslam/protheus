// Infring Workflows Page — Workflow builder + run history
'use strict';

var WORKFLOW_DEFAULT_STEP = { name: '', agent_name: '', mode: 'sequential', prompt: '{{input}}' };

function workflowNormalizeStep(step) {
  var source = step && typeof step === 'object' ? step : {};
  var name = String(source.name || '').trim() || 'step';
  var agentName = String(source.agent_name || '').trim();
  var mode = String(source.mode || 'sequential').trim() || 'sequential';
  var prompt = String(source.prompt || '{{input}}');
  return {
    name: name,
    agent_name: agentName,
    mode: mode,
    prompt: prompt || '{{input}}'
  };
}

function workflowNormalizeDraft(draft) {
  var source = draft && typeof draft === 'object' ? draft : {};
  var name = String(source.name || '').trim();
  var description = String(source.description || '');
  var steps = Array.isArray(source.steps) ? source.steps.map(workflowNormalizeStep) : [];
  steps = steps.filter(function(step) { return step.agent_name && step.prompt; });
  return { name: name, description: description, steps: steps };
}

function workflowFormatRunResult(payload) {
  if (payload && typeof payload === 'object' && typeof payload.output === 'string') return payload.output;
  try { return JSON.stringify(payload || {}, null, 2); } catch(_) {}
  return String(payload || '');
}

function workflowsPage() {
  return {
    // -- Workflows state --
    workflows: [],
    showCreateModal: false,
    runModal: null,
    runInput: '',
    runResult: '',
    running: false,
    loading: true,
    loadError: '',
    newWf: { name: '', description: '', steps: [Object.assign({}, WORKFLOW_DEFAULT_STEP)] },
    editModal: null,
    editWf: { name: '', description: '', steps: [] },

    // -- Workflows methods --
    async loadWorkflows() {
      this.loading = true;
      this.loadError = '';
      try {
        this.workflows = await InfringAPI.get('/api/workflows');
      } catch(e) {
        this.workflows = [];
        this.loadError = e.message || 'Could not load workflows.';
      }
      this.loading = false;
    },

    async loadData() { return this.loadWorkflows(); },

    async createWorkflow() {
      var draft = workflowNormalizeDraft(this.newWf);
      if (!draft.name) {
        InfringToast.error('Workflow name is required');
        return;
      }
      if (draft.steps.length === 0) {
        InfringToast.error('Add at least one workflow step with an agent');
        return;
      }
      try {
        var wfName = draft.name;
        await InfringAPI.post('/api/workflows', { name: draft.name, description: draft.description, steps: draft.steps });
        this.showCreateModal = false;
        this.newWf = { name: '', description: '', steps: [Object.assign({}, WORKFLOW_DEFAULT_STEP)] };
        InfringToast.success('Workflow "' + wfName + '" created');
        await this.loadWorkflows();
      } catch(e) {
        InfringToast.error('Failed to create workflow: ' + e.message);
      }
    },

    showRunModal(wf) {
      this.runModal = wf;
      this.runInput = '';
      this.runResult = '';
    },

    async executeWorkflow() {
      if (!this.runModal) return;
      this.running = true;
      this.runResult = '';
      try {
        var res = await InfringAPI.post('/api/workflows/' + this.runModal.id + '/run', { input: this.runInput });
        this.runResult = workflowFormatRunResult(res);
        InfringToast.success('Workflow completed');
      } catch(e) {
        this.runResult = 'Error: ' + e.message;
        InfringToast.error('Workflow failed: ' + e.message);
      }
      this.running = false;
    },

    async viewRuns(wf) {
      try {
        var runs = await InfringAPI.get('/api/workflows/' + wf.id + '/runs');
        this.runResult = JSON.stringify(runs, null, 2);
        this.runModal = wf;
      } catch(e) {
        InfringToast.error('Failed to load run history: ' + e.message);
      }
    },

    async deleteWorkflow(wf) {
      var self = this;
      InfringToast.confirm('Delete Workflow', 'Delete workflow "' + wf.name + '"? This cannot be undone.', async function() {
        try {
          await InfringAPI.delete('/api/workflows/' + wf.id);
          InfringToast.success('Workflow "' + wf.name + '" deleted');
          await self.loadWorkflows();
        } catch(e) {
          InfringToast.error('Failed to delete workflow: ' + e.message);
        }
      });
    },

    async showEditModal(wf) {
      try {
        var full = await InfringAPI.get('/api/workflows/' + wf.id);
        this.editWf = {
          name: full.name || '',
          description: full.description || '',
          steps: (full.steps || []).map(function(s) {
            return {
              name: s.name || '',
              agent_name: (s.agent && s.agent.name) || '',
              mode: s.mode || 'sequential',
              prompt: s.prompt_template || '{{input}}'
            };
          })
        };
        if (this.editWf.steps.length === 0) {
          this.editWf.steps.push({ name: '', agent_name: '', mode: 'sequential', prompt: '{{input}}' });
        }
        this.editModal = wf;
      } catch(e) {
        InfringToast.error('Failed to load workflow: ' + e.message);
      }
    },

    async saveWorkflow() {
      if (!this.editModal) return;
      var draft = workflowNormalizeDraft(this.editWf);
      if (!draft.name) {
        InfringToast.error('Workflow name is required');
        return;
      }
      if (draft.steps.length === 0) {
        InfringToast.error('Add at least one workflow step with an agent');
        return;
      }
      try {
        var wfName = draft.name;
        await InfringAPI.put('/api/workflows/' + this.editModal.id, { name: draft.name, description: draft.description, steps: draft.steps });
        this.editModal = null;
        InfringToast.success('Workflow "' + wfName + '" updated');
        await this.loadWorkflows();
      } catch(e) {
        InfringToast.error('Failed to update workflow: ' + e.message);
      }
    }
  };
}
