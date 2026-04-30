function infringCloseTaskbarHeroMenu(page) {
  page.taskbarHeroMenuOpen = false;
}

function infringCloseTaskbarTextMenu(page) {
  page.taskbarTextMenuOpen = '';
}

function infringTaskbarTextMenuIsOpen(page, menuName) {
  var key = String(menuName || '').trim().toLowerCase();
  if (!key) return false;
  return String(page.taskbarTextMenuOpen || '').trim().toLowerCase() === key;
}

function infringToggleTaskbarTextMenu(page, menuName) {
  var key = String(menuName || '').trim().toLowerCase();
  if (!key) {
    page.closeTaskbarTextMenu();
    return;
  }
  page.closeTaskbarHeroMenu();
  page.taskbarTextMenuOpen = page.taskbarTextMenuIsOpen(key) ? '' : key;
}

function infringHandleTaskbarHelpManual(page) {
  page.closeTaskbarTextMenu();
  page.openPopupWindow('manual');
}

function infringHandleTaskbarHelpReportIssue(page) {
  page.closeTaskbarTextMenu();
  page.openPopupWindow('report');
}

async function infringSubmitReportIssueDraft(page) {
  var draft = String(page.reportIssueDraft || '').trim();
  if (!draft) {
    InfringToast.error('Please add issue details before submitting.');
    return;
  }
  var entry = {
    id: 'issue-' + String(Date.now()),
    ts: Date.now(),
    text: draft,
    page: String(page.page || '').trim(),
    agent_id: String((page.currentAgent && page.currentAgent.id) || '').trim()
  };
  try {
    var raw = localStorage.getItem('infring-issue-report-drafts');
    var list = raw ? JSON.parse(raw) : [];
    if (!Array.isArray(list)) list = [];
    list.unshift(entry);
    localStorage.setItem('infring-issue-report-drafts', JSON.stringify(list.slice(0, 25)));
  } catch(_) {}
  var title = ((draft.split(/\r?\n/).find(function(line) { return String(line || '').trim(); }) || draft).replace(/\s+/g, ' ').trim().slice(0, 120) || 'Dashboard issue report');
  var issueBody = '## User Report\n\n' + draft + '\n\n## Runtime Context\n- page: ' + (entry.page || 'unknown') + '\n- agent_id: ' + (entry.agent_id || 'none') + '\n- reported_at: ' + new Date(entry.ts || Date.now()).toISOString() + '\n- client_version: ' + String(page.version || 'unknown');
  try {
    var result = await InfringAPI.post('/api/dashboard/action', {
      action: 'dashboard.github.issue.create',
      payload: { title: title, body: issueBody, source: 'dashboard_report_popup' }
    });
    var actionResult = result && typeof result === 'object' ? (result.lane || result.payload || result) : {};
    if ((result && result.ok === false) || (actionResult && actionResult.ok === false)) {
      throw new Error(String((actionResult && (actionResult.error || actionResult.message)) || (result && (result.error || result.message)) || 'issue_submit_failed'));
    }
    var issueUrl = String((actionResult && (actionResult.html_url || actionResult.issue_url)) || '').trim();
    page.reportIssueDraft = '';
    page.closePopupWindow('report');
    InfringToast.success(issueUrl ? ('Issue submitted: ' + issueUrl) : 'Issue submitted.');
  } catch (e) {
    InfringToast.error('Issue submit failed (saved locally): ' + String(e && e.message ? e.message : 'unknown error'));
  }
}
