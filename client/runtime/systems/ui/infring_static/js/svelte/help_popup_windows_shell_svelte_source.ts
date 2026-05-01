const COMPONENT_TAG = 'infring-help-popup-windows-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-help-popup-windows-shell', shadow: 'none' }} />
<svelte:window on:keydown={handleWindowKeydown} />
<script>
  import { onDestroy, onMount } from 'svelte';

  let uiTick = 0;
  let unsubscribe = null;
  let timer = 0;

  function bridge() {
    var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
    return services && services.appStore ? services.appStore : null;
  }

  function appStore() {
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.current === 'function') {
      var current = storeBridge.current();
      if (current) return current;
    }
    if (storeBridge && typeof storeBridge.root === 'function') return storeBridge.root();
    return typeof window !== 'undefined' && window.InfringApp ? window.InfringApp : null;
  }

  function call(name) {
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.method === 'function') {
      var method = storeBridge.method(name);
      if (method) {
        var methodArgs = Array.prototype.slice.call(arguments, 1);
        try { return method.apply(null, methodArgs); } catch (_) { return undefined; }
      }
    }
    var store = appStore();
    if (!store || typeof store[name] !== 'function') return undefined;
    var args = Array.prototype.slice.call(arguments, 1);
    try { return store[name].apply(store, args); } catch (_) { return undefined; }
  }

  function bump() {
    uiTick += 1;
  }

  function isOpen(kind, _tick) {
    var store = appStore() || {};
    return String(kind || '').trim().toLowerCase() === 'report'
      ? !!store.reportIssueWindowOpen
      : !!store.helpManualWindowOpen;
  }

  function isDragging(kind, _tick) {
    var store = appStore() || {};
    var key = String(kind || '').trim().toLowerCase();
    return !!(store.popupWindowDragActive && String(store.popupWindowDragKind || '').trim().toLowerCase() === key);
  }

  function popupClass(kind, _tick) {
    var key = String(kind || '').trim().toLowerCase() === 'report' ? 'report' : 'manual';
    return 'popup-window popup-window-' + key + ' drag-bar' + (isDragging(key, uiTick) ? ' is-container-dragging' : '');
  }

  function popupStyle(kind, _tick) {
    return String(call('popupWindowStyle', kind) || 'display:none;');
  }

  function manualHtml(_tick) {
    return String(call('manualDocumentHtml') || '');
  }

  function reportDraft(_tick) {
    return String((appStore() || {}).reportIssueDraft || '');
  }

  function setReportDraft(event) {
    var value = event && event.target ? event.target.value : '';
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.set === 'function') storeBridge.set('reportIssueDraft', value);
    else {
      var store = appStore();
      if (store) store.reportIssueDraft = value;
    }
    bump();
  }

  function close(kind) {
    call('closePopupWindow', kind);
    bump();
  }

  function startDrag(kind, event) {
    call('startPopupWindowPointerDrag', kind, event);
    bump();
  }

  function submitReport() {
    call('submitReportIssueDraft');
    bump();
  }

  function handleWindowKeydown(event) {
    if (!event || event.key !== 'Escape') return;
    if (isOpen('report', uiTick)) close('report');
    if (isOpen('manual', uiTick)) close('manual');
  }

  onMount(function() {
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.subscribe === 'function') {
      unsubscribe = storeBridge.subscribe(bump);
    }
    timer = window.setInterval(bump, 250);
  });

  onDestroy(function() {
    if (typeof unsubscribe === 'function') unsubscribe();
    if (timer) window.clearInterval(timer);
  });
</script>

<infring-popup-window-shell
  class={popupClass('manual', uiTick)}
  data-popup-window-kind="manual"
  kind="manual"
  open={isOpen('manual', uiTick)}
  dragging={isDragging('manual', uiTick)}
  style={popupStyle('manual', uiTick)}
  on:pointerdown|capture={(event) => startDrag('manual', event)}
  on:mousedown|capture={(event) => startDrag('manual', event)}
>
  <div class="popup-window-header">
    <button class="popup-window-close" type="button" on:click={() => close('manual')} aria-label="Close Manual window">
      <span aria-hidden="true">×</span>
    </button>
    <h3 class="popup-window-title">Infring Manual</h3>
  </div>
  <div class="popup-window-body popup-window-body-markdown markdown-body">{@html manualHtml(uiTick)}</div>
</infring-popup-window-shell>

<infring-popup-window-shell
  class={popupClass('report', uiTick)}
  data-popup-window-kind="report"
  kind="report"
  open={isOpen('report', uiTick)}
  dragging={isDragging('report', uiTick)}
  style={popupStyle('report', uiTick)}
  on:pointerdown|capture={(event) => startDrag('report', event)}
  on:mousedown|capture={(event) => startDrag('report', event)}
>
  <div class="popup-window-header">
    <button class="popup-window-close" type="button" on:click={() => close('report')} aria-label="Close Report issue window">
      <span aria-hidden="true">×</span>
    </button>
    <h3 class="popup-window-title">Report an Issue</h3>
  </div>
  <div class="popup-window-body">
    <p class="popup-window-subtitle">Describe what happened and what you expected. This submits through the shell action bus and keeps a local backup if core submission fails.</p>
    <textarea
      class="popup-window-textarea"
      value={reportDraft(uiTick)}
      placeholder="Issue summary, steps to reproduce, screenshots, expected behavior..."
      spellcheck="true"
      on:input={setReportDraft}
    ></textarea>
    <div style="display:flex;justify-content:flex-end;align-items:center;gap:8px;padding-top:4px">
      <button class="btn btn-primary btn-sm" type="button" disabled={!String(reportDraft(uiTick) || '').trim()} on:click={submitReport}>Submit issue</button>
    </div>
  </div>
</infring-popup-window-shell>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
