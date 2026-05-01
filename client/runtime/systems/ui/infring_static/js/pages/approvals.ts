// Infring Approvals Page — Execution approval queue for sensitive agent actions
'use strict';

function approvalsRelativeTime(value) {
  if (!value) return '';
  var parsed = Date.parse(String(value));
  if (!Number.isFinite(parsed)) return '';
  var secs = Math.floor((Date.now() - parsed) / 1000);
  if (!Number.isFinite(secs) || secs < 0) secs = 0;
  if (secs < 60) return secs + 's ago';
  if (secs < 3600) return Math.floor(secs / 60) + 'm ago';
  if (secs < 86400) return Math.floor(secs / 3600) + 'h ago';
  return Math.floor(secs / 86400) + 'd ago';
}

function approvalsNormalizeRows(rows) {
  var source = Array.isArray(rows) ? rows : [];
  return source.map(function(entry) {
    var row = entry && typeof entry === 'object' ? entry : {};
    return Object.assign({}, row, {
      id: String(row.id || row.approval_id || ''),
      status: String(row.status || 'pending')
    });
  });
}

function approvalsPage() {
  return {
    approvals: [],
    filterStatus: 'all',
    loading: true,
    loadError: '',
    decisionLoading: {},

    get filtered() {
      var f = this.filterStatus;
      if (f === 'all') return this.approvals;
      return this.approvals.filter(function(a) { return a.status === f; });
    },

    get pendingCount() {
      return this.approvals.filter(function(a) { return a.status === 'pending'; }).length;
    },

    async loadData() {
      this.loading = true;
      this.loadError = '';
      try {
        var data = await InfringAPI.get('/api/approvals');
        this.approvals = approvalsNormalizeRows(data.approvals);
      } catch(e) {
        this.loadError = e.message || 'Could not load approvals.';
      }
      this.loading = false;
    },

    isDecisionBusy(id) {
      return this.decisionLoading[String(id)] === true;
    },

    setDecisionBusy(id, busy) {
      var key = String(id);
      if (busy) this.decisionLoading[key] = true;
      else delete this.decisionLoading[key];
    },

    async submitDecision(id, action) {
      if (!id || this.isDecisionBusy(id)) return;
      this.setDecisionBusy(id, true);
      try {
        await InfringAPI.post('/api/approvals/' + id + '/' + action, {});
        InfringToast.success(action === 'approve' ? 'Approved' : 'Rejected');
        await this.loadData();
      } catch(e) {
        InfringToast.error((e && e.message) ? e.message : ('Failed to ' + action));
      }
      this.setDecisionBusy(id, false);
    },

    async approve(id) {
      return this.submitDecision(id, 'approve');
    },

    async reject(id) {
      var self = this;
      InfringToast.confirm('Reject Action', 'Are you sure you want to reject this action?', async function() {
        await self.submitDecision(id, 'reject');
      });
    },

    timeAgo(dateStr) {
      return approvalsRelativeTime(dateStr);
    }
  };
}
