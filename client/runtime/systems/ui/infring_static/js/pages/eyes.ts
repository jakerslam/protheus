// Infring Eyes Page — system eye catalog sync + manual URL/API-key onboarding
'use strict';

var EYES_ALLOWED_STATUS = { active: true, paused: true, dormant: true };

function eyesNumber(value, fallback) {
  var parsed = Number(value);
  if (!Number.isFinite(parsed)) return Number(fallback || 0);
  return parsed;
}

function eyesNormalizeStatus(value) {
  var lowered = String(value || 'active').trim().toLowerCase();
  return EYES_ALLOWED_STATUS[lowered] ? lowered : 'active';
}

function eyesNormalizeTopics(raw) {
  var source = String(raw || '');
  var out = [];
  var seen = Object.create(null);
  source.split(/[,\n]/).forEach(function(piece) {
    var topic = String(piece || '').trim();
    if (!topic) return;
    var key = topic.toLowerCase();
    if (seen[key]) return;
    seen[key] = true;
    out.push(topic);
  });
  return out;
}

function eyesSafeHost(urlValue) {
  try {
    return new URL(String(urlValue || '')).host || '';
  } catch (_) {
    return '';
  }
}

function normalizeEyeRow(row) {
  var source = row && typeof row === 'object' ? row : {};
  var endpointUrl = String(source.endpoint_url || source.url || '').trim();
  return {
    id: String(source.id || ''),
    name: String(source.name || '').trim(),
    status: eyesNormalizeStatus(source.status),
    cadence_hours: Math.max(1, Math.min(168, Math.round(eyesNumber(source.cadence_hours, 4)))),
    endpoint_url: endpointUrl,
    endpoint_host: String(source.endpoint_host || eyesSafeHost(endpointUrl) || '').trim(),
    source: String(source.source || '').trim() || 'system',
    api_key_present: !!source.api_key_present,
    topics: Array.isArray(source.topics) ? source.topics : eyesNormalizeTopics(source.topics),
    updated_at: String(source.updated_at || source.ts || '').trim()
  };
}

function normalizeEyeFormPayload(form) {
  var source = form && typeof form === 'object' ? form : {};
  var name = String(source.name || '').trim();
  var url = String(source.url || '').trim();
  var apiKey = String(source.apiKey || '').trim();
  var topics = eyesNormalizeTopics(source.topics);
  var cadence = Math.max(1, Math.min(168, Math.round(eyesNumber(source.cadenceHours, 4))));
  return {
    name: name || (url ? eyesSafeHost(url) : 'eye'),
    status: eyesNormalizeStatus(source.status),
    url: url,
    api_key: apiKey,
    cadence_hours: cadence,
    topics: topics
  };
}

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
      if (eye.endpoint_url) return eyesSafeHost(eye.endpoint_url) || eye.endpoint_url;
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
        this.eyes = (Array.isArray(data && data.eyes) ? data.eyes : []).map(normalizeEyeRow);
      } catch (e) {
        this.eyes = [];
        this.loadError = e && e.message ? e.message : 'Could not load eyes.';
      }
      this.loading = false;
    },

    async addEye() {
      this.formError = '';
      var payload = normalizeEyeFormPayload(this.form);
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
        var normalizedEye = normalizeEyeRow(data && data.eye ? data.eye : payload);
        var addedName = normalizedEye.name || payload.name || 'eye';
        InfringToast.success((data.created ? 'Added ' : 'Updated ') + '"' + addedName + '"');
        this.form.apiKey = '';
        this.form.url = '';
        this.form.topics = (payload.topics || []).join(', ');
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
