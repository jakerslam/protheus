// Infring Eyes Page — system eye catalog sync + manual URL/API-key onboarding
'use strict';

function eyesPage() {
  return {
    eyes: [],
    loading: true,
    loadError: '',
    saving: false,
    formError: '',
    form: {
      name: '',
      status: 'active',
      url: '',
      apiKey: '',
      cadenceHours: 4,
      topics: '',
    },

    get activeCount() {
      return this.eyes.filter(function(eye) {
        return String(eye && eye.status ? eye.status : '').toLowerCase() === 'active';
      }).length;
    },

    statusBadge(eye) {
      var status = String(eye && eye.status ? eye.status : 'active').toLowerCase();
      if (status === 'active') return { text: 'active', cls: 'badge-success' };
      if (status === 'paused') return { text: 'paused', cls: 'badge-warn' };
      if (status === 'dormant') return { text: 'dormant', cls: 'badge-muted' };
      return { text: status || 'disabled', cls: 'badge-dim' };
    },

    sourceLabel(eye) {
      if (!eye || typeof eye !== 'object') return 'system';
      if (eye.endpoint_host) return eye.endpoint_host;
      if (eye.endpoint_url) return eye.endpoint_url;
      if (eye.api_key_present) return 'api-key';
      return eye.source || 'system';
    },

    formatUpdated(ts) {
      var raw = String(ts || '').trim();
      if (!raw) return '-';
      var d = new Date(raw);
      if (Number.isNaN(d.getTime())) return '-';
      return d.toLocaleString([], {
        month: 'short',
        day: 'numeric',
        hour: 'numeric',
        minute: '2-digit',
      });
    },

    resetForm() {
      this.form = {
        name: '',
        status: 'active',
        url: '',
        apiKey: '',
        cadenceHours: 4,
        topics: '',
      };
      this.formError = '';
    },

    async loadEyes() {
      this.loading = true;
      this.loadError = '';
      try {
        var data = await InfringAPI.get('/api/eyes');
        this.eyes = Array.isArray(data && data.eyes) ? data.eyes : [];
      } catch (e) {
        this.eyes = [];
        this.loadError = e && e.message ? e.message : 'Could not load eyes.';
      }
      this.loading = false;
    },

    async addEye() {
      this.formError = '';
      var payload = {
        name: this.form.name,
        status: this.form.status,
        url: this.form.url,
        api_key: this.form.apiKey,
        cadence_hours: this.form.cadenceHours,
        topics: this.form.topics,
      };
      if (!String(payload.url || '').trim() && !String(payload.api_key || '').trim()) {
        this.formError = 'Provide a source URL, an API key, or both.';
        return;
      }
      this.saving = true;
      try {
        var data = await InfringAPI.post('/api/eyes', payload);
        if (!data || data.ok === false) {
          throw new Error((data && data.error) ? String(data.error) : 'Eyes update failed');
        }
        var addedName = data && data.eye && data.eye.name ? data.eye.name : (payload.name || 'eye');
        InfringToast.success((data.created ? 'Added ' : 'Updated ') + '"' + addedName + '"');
        this.form.apiKey = '';
        this.form.url = '';
        this.form.topics = '';
        if (data.created) this.form.name = '';
        await this.loadEyes();
      } catch (e) {
        this.formError = e && e.message ? e.message : 'Could not save eye.';
        InfringToast.error(this.formError);
      }
      this.saving = false;
    },
  };
}
