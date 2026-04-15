// Infring Scheduler Page — Cron job management + event triggers unified view
'use strict';

function schedulerPage() {
  return {
    tab: 'jobs',

    // -- Scheduled Jobs state --
    jobs: [],
    loading: true,
    loadError: '',

    // -- Event Triggers state --
    triggers: [],
    trigLoading: false,
    trigLoadError: '',

    // -- Run History state --
    history: [],
    historyLoading: false,

    // -- Create Job form --
    showCreateForm: false,
    newJob: {
      name: '',
      cron: '',
      agent_id: '',
      message: '',
      enabled: true
    },
    creating: false,

    // -- Run Now state --
    runningJobId: '',

    // Cron presets
    cronPresets: [
      { label: 'Every minute', cron: '* * * * *' },
      { label: 'Every 5 minutes', cron: '*/5 * * * *' },
      { label: 'Every 15 minutes', cron: '*/15 * * * *' },
      { label: 'Every 30 minutes', cron: '*/30 * * * *' },
      { label: 'Every hour', cron: '0 * * * *' },
      { label: 'Every 6 hours', cron: '0 */6 * * *' },
      { label: 'Daily at midnight', cron: '0 0 * * *' },
      { label: 'Daily at 9am', cron: '0 9 * * *' },
      { label: 'Weekdays at 9am', cron: '0 9 * * 1-5' },
      { label: 'Every Monday 9am', cron: '0 9 * * 1' },
      { label: 'First of month', cron: '0 0 1 * *' }
    ],

    // ── Lifecycle ──

    async loadData() {
      this.loading = true;
      this.loadError = '';
      try {
        await this.loadJobs();
      } catch(e) {
        this.loadError = e.message || 'Could not load scheduler data.';
      }
      this.loading = false;
    },

    async loadJobs() {
      var data = await InfringAPI.get('/api/cron/jobs');
      var raw = data.jobs || [];
      // Normalize cron API response to flat fields the UI expects
      this.jobs = raw.map(function(j) {
        var cron = '';
        if (j.schedule) {
          if (j.schedule.kind === 'cron') cron = j.schedule.expr || '';
          else if (j.schedule.kind === 'every') cron = 'every ' + j.schedule.every_secs + 's';
          else if (j.schedule.kind === 'at') cron = 'at ' + (j.schedule.at || '');
        }
        return {
          id: j.id,
          name: j.name,
          cron: cron,
          agent_id: j.agent_id,
          message: j.action ? j.action.message || '' : '',
          enabled: j.enabled,
          last_run: j.last_run,
          next_run: j.next_run,
          delivery: j.delivery ? j.delivery.kind || '' : '',
          created_at: j.created_at
        };
      });
    },

    async loadTriggers() {
      this.trigLoading = true;
      this.trigLoadError = '';
      try {
        var data = await InfringAPI.get('/api/triggers');
        this.triggers = Array.isArray(data) ? data : [];
      } catch(e) {
        this.triggers = [];
        this.trigLoadError = e.message || 'Could not load triggers.';
      }
      this.trigLoading = false;
    },

    async loadHistory() {
      this.historyLoading = true;
      try {
        var historyItems = [];
        var jobs = this.jobs || [];
        for (var i = 0; i < jobs.length; i++) {
          var job = jobs[i];
          if (job.last_run) {
            historyItems.push({
              timestamp: job.last_run,
              name: job.name || '(unnamed)',
              type: 'schedule',
              status: 'completed',
              run_count: 0
            });
          }
        }
        var triggers = this.triggers || [];
        for (var j = 0; j < triggers.length; j++) {
          var t = triggers[j];
          if (t.fire_count > 0) {
            historyItems.push({
              timestamp: t.created_at,
              name: 'Trigger: ' + this.triggerType(t.pattern),
              type: 'trigger',
              status: 'fired',
              run_count: t.fire_count
            });
          }
        }
        historyItems.sort(function(a, b) {
          return new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime();
        });
        this.history = historyItems;
      } catch(e) {
        this.history = [];
      }
      this.historyLoading = false;
    },

    // ── Job CRUD ──

    async createJob() {
      if (!this.newJob.name.trim()) {
        InfringToast.warn('Please enter a job name');
        return;
      }
      if (!this.newJob.cron.trim()) {
        InfringToast.warn('Please enter a cron expression');
        return;
      }
      this.creating = true;
      try {
        var jobName = this.newJob.name;
        var body = {
          agent_id: this.newJob.agent_id,
          name: this.newJob.name,
          schedule: { kind: 'cron', expr: this.newJob.cron },
          action: { kind: 'agent_turn', message: this.newJob.message || 'Scheduled task: ' + this.newJob.name },
          delivery: { kind: 'last_channel' },
          enabled: this.newJob.enabled
        };
        await InfringAPI.post('/api/cron/jobs', body);
        this.showCreateForm = false;
        this.newJob = { name: '', cron: '', agent_id: '', message: '', enabled: true };
        InfringToast.success('Schedule "' + jobName + '" created');
        await this.loadJobs();
      } catch(e) {
        InfringToast.error('Failed to create schedule: ' + (e.message || e));
      }
      this.creating = false;
    },

    async toggleJob(job) {
      try {
        var newState = !job.enabled;
        await InfringAPI.put('/api/cron/jobs/' + job.id + '/enable', { enabled: newState });
        job.enabled = newState;
        InfringToast.success('Schedule ' + (newState ? 'enabled' : 'paused'));
      } catch(e) {
        InfringToast.error('Failed to toggle schedule: ' + (e.message || e));
      }
    },

    deleteJob(job) {
      var self = this;
      var jobName = job.name || job.id;
      InfringToast.confirm('Delete Schedule', 'Delete "' + jobName + '"? This cannot be undone.', async function() {
        try {
          await InfringAPI.del('/api/cron/jobs/' + job.id);
          self.jobs = self.jobs.filter(function(j) { return j.id !== job.id; });
          InfringToast.success('Schedule "' + jobName + '" deleted');
        } catch(e) {
          InfringToast.error('Failed to delete schedule: ' + (e.message || e));
        }
      });
    },

    async runNow(job) {
      this.runningJobId = job.id;
      try {
        var result = await InfringAPI.post('/api/schedules/' + job.id + '/run', {});
        if (result.status === 'completed') {
          InfringToast.success('Schedule "' + (job.name || 'job') + '" executed successfully');
          job.last_run = new Date().toISOString();
        } else {
          InfringToast.error('Schedule run failed: ' + (result.error || 'Unknown error'));
        }
      } catch(e) {
        InfringToast.error('Run Now is not yet available for cron jobs');
      }
      this.runningJobId = '';
    },
