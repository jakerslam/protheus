const COMPONENT_TAG = 'infring-agent-details-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-agent-details-shell', shadow: 'none' }} />
<script>
  import { onMount, onDestroy } from 'svelte';

  export let shellPrimitive = 'agent-details';
  export let parentOwnedContracts = true;

  let rootNode = null;
  let fileInput = null;
  let timer = 0;
  let uiTick = 0;
  let state = {
    open: false,
    loading: false,
    agent: null,
    tab: 'info',
    form: {},
    savePending: false,
    editingName: false,
    editingEmoji: false,
    emojiPickerOpen: false,
    emojiSearch: '',
    avatarUrlPickerOpen: false,
    avatarUrlDraft: '',
    avatarUploading: false,
    avatarUploadError: '',
    editingModel: false,
    editingProvider: false,
    editingFallback: false,
    newModelValue: '',
    newProviderValue: '',
    newFallbackValue: '',
    archetypes: [],
    vibes: [],
    emojiRows: [],
    permissionRows: []
  };

  function page() {
    return (typeof window !== 'undefined' && window.InfringChatPage) || null;
  }
  function call(name) {
    var p = page();
    if (!p || typeof p[name] !== 'function') return undefined;
    var args = Array.prototype.slice.call(arguments, 1);
    try { return p[name].apply(p, args); } catch (_e) { return undefined; }
  }
  function list(value) {
    return Array.isArray(value) ? value : [];
  }
  function text(value, fallback) {
    var raw = String(value == null ? '' : value).trim();
    return raw || String(fallback || '');
  }
  function pageList(name) {
    var p = page();
    return p && Array.isArray(p[name]) ? p[name] : [];
  }
  function refresh() {
    var p = page();
    if (!p) return;
    var agent = p.agentDrawer || null;
    state = {
      open: !!p.showAgentDrawer,
      loading: !!p.agentDrawerLoading,
      agent: agent,
      tab: text(p.drawerTab, 'info'),
      form: (p.drawerConfigForm && typeof p.drawerConfigForm === 'object') ? p.drawerConfigForm : {},
      savePending: !!p.drawerSavePending,
      editingName: !!p.drawerEditingName,
      editingEmoji: !!p.drawerEditingEmoji,
      emojiPickerOpen: !!p.drawerEmojiPickerOpen,
      emojiSearch: String(p.drawerEmojiSearch || ''),
      avatarUrlPickerOpen: !!p.drawerAvatarUrlPickerOpen,
      avatarUrlDraft: String(p.drawerAvatarUrlDraft || ''),
      avatarUploading: !!p.drawerAvatarUploading,
      avatarUploadError: String(p.drawerAvatarUploadError || ''),
      editingModel: !!p.drawerEditingModel,
      editingProvider: !!p.drawerEditingProvider,
      editingFallback: !!p.drawerEditingFallback,
      newModelValue: String(p.drawerNewModelValue || ''),
      newProviderValue: String(p.drawerNewProviderValue || ''),
      newFallbackValue: String(p.drawerNewFallbackValue || ''),
      archetypes: pageList('drawerArchetypeOptions'),
      vibes: pageList('drawerVibeOptions'),
      emojiRows: list(call('filteredDrawerEmojiCatalog')),
      permissionRows: list(call('drawerPermissionRows'))
    };
    uiTick += 1;
  }
  function setPageValue(name, value) {
    var p = page();
    if (!p) return;
    p[name] = value;
    refresh();
  }
  function setFormValue(name, value) {
    var p = page();
    if (!p) return;
    if (!p.drawerConfigForm || typeof p.drawerConfigForm !== 'object') p.drawerConfigForm = {};
    p.drawerConfigForm[name] = value;
    refresh();
  }
  function closeDrawer() {
    call('closeAgentDrawer');
    refresh();
  }
  function setTab(tab) {
    setPageValue('drawerTab', tab);
  }
  function avatarUrl(_tick) {
    return text((state.agent && state.agent.avatar_url) || state.form.avatar_url, '');
  }
  function displayEmoji(_tick) {
    return text(state.form.emoji || (state.agent && state.agent.identity && state.agent.identity.emoji), '');
  }
  function displayName(_tick) {
    return text(state.form.name || (state.agent && (state.agent.name || state.agent.id)), 'Agent');
  }
  function createdLabel() {
    var created = state.agent && state.agent.created_at;
    if (!created) return '-';
    try { return new Date(created).toLocaleString(); } catch (_e) { return '-'; }
  }
  function fallbackRows() {
    return list(state.agent && state.agent._fallbacks);
  }
  function startProviderEdit() {
    setPageValue('drawerNewProviderValue', state.agent && state.agent.model_provider ? state.agent.model_provider : '');
    setPageValue('drawerEditingProvider', true);
  }
  function startModelEdit() {
    var provider = state.agent && state.agent.model_provider ? state.agent.model_provider : '';
    var model = state.agent && state.agent.model_name ? state.agent.model_name : '';
    setPageValue('drawerNewModelValue', (provider ? provider + '/' : '') + model);
    setPageValue('drawerEditingModel', true);
  }
  function startFallbackEdit() {
    setPageValue('drawerNewFallbackValue', '');
    setPageValue('drawerEditingFallback', true);
  }
  function uploadAvatar() {
    if (fileInput) fileInput.click();
  }
  function avatarChanged(event) {
    var input = event && event.target;
    call('uploadDrawerAvatar', input && input.files);
    if (input) input.value = '';
    refresh();
  }
  function toggleEmojiPicker() {
    call('toggleDrawerEmojiPicker');
    refresh();
  }
  function toggleAvatarUrlPicker() {
    call('toggleDrawerAvatarUrlPicker');
    refresh();
  }
  function closeAvatarMenus() {
    var p = page();
    if (!p) return;
    p.drawerEmojiPickerOpen = false;
    p.drawerAvatarUrlPickerOpen = false;
    refresh();
  }
  function selectEmoji(item) {
    call('selectDrawerEmoji', item);
    refresh();
  }
  function applyAvatarUrl() {
    call('applyDrawerAvatarUrl');
    refresh();
  }
  function permissionSummary(section, _tick) {
    var summary = call('drawerPermissionCategoryState', section) || {};
    return String(summary.allow || 0) + ' allowed / ' + String(summary.inherit || 0) + ' inherited / ' + String(summary.deny || 0) + ' denied';
  }
  function permissionState(key, _tick) {
    return Number(call('drawerPermissionState', key) || 0);
  }
  function permissionStateClass(key, _tick) {
    return String(call('drawerPermissionStateClass', permissionState(key, uiTick)) || '');
  }
  function permissionStateLabel(key, _tick) {
    return String(call('drawerPermissionStateLabel', permissionState(key, uiTick)) || 'Inherited');
  }
  function permissionDescription(key) {
    return String(call('drawerPermissionDescriptionForKey', key) || '');
  }
  function setPermission(key, value) {
    call('setDrawerPermissionState', key, value);
    refresh();
  }
  function setPermissionCategory(category, value) {
    call('setDrawerPermissionCategoryState', category, value);
    refresh();
  }
  function escapeSet(name, value) {
    setPageValue(name, value);
  }
  function saveAll() {
    call('saveDrawerAll');
    refresh();
  }
  function setMode(event) {
    call('setDrawerMode', event && event.target ? event.target.value : 'full');
    refresh();
  }
  function addFallback() {
    call('addDrawerFallback');
    refresh();
  }
  function removeFallback(index) {
    call('removeDrawerFallback', index);
    refresh();
  }
  function outsidePointer(event) {
    if (!state.open || !rootNode || !event || !rootNode.contains(event.target)) return;
    var target = event.target;
    if (target && target.closest && target.closest('.chat-agent-avatar-actions')) return;
    if ((page() || {}).drawerEmojiPickerOpen || (page() || {}).drawerAvatarUrlPickerOpen) closeAvatarMenus();
  }
  function icon(kind) {
    if (kind === 'edit') return '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20h9"/><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z"/></svg>';
    if (kind === 'emoji') return '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"></circle><path d="M8 15c1 .8 2.1 1.2 4 1.2s3-.4 4-1.2"></path><circle cx="9" cy="10" r="1"></circle><circle cx="15" cy="10" r="1"></circle></svg>';
    if (kind === 'upload') return '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><path d="M7 10l5-5 5 5"></path><path d="M12 5v12"></path></svg>';
    return '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"></circle><path d="M3 12h18"></path><path d="M12 3a13 13 0 0 1 0 18"></path><path d="M12 3a13 13 0 0 0 0 18"></path></svg>';
  }

  onMount(function() {
    refresh();
    timer = window.setInterval(refresh, 300);
    document.addEventListener('pointerdown', outsidePointer, true);
  });
  onDestroy(function() {
    if (timer) window.clearInterval(timer);
    document.removeEventListener('pointerdown', outsidePointer, true);
  });
</script>

{#if state.open}
  <div class="chat-agent-drawer-backdrop" on:click={closeDrawer}></div>
  <aside bind:this={rootNode} class="chat-agent-drawer overlay-shared-surface" data-shell-primitive={shellPrimitive} on:pointerdown|stopPropagation>
    <div class="chat-agent-drawer-head">
      <div class="chat-agent-drawer-title">Agent Details</div>
      <button class="chat-agent-drawer-close" on:click={closeDrawer} aria-label="Close details">&times;</button>
    </div>
    <div class="chat-agent-drawer-body">
      {#if state.loading}<div class="text-xs text-dim">Loading...</div>{/if}
      {#if state.agent}
        <div class="chat-agent-identity">
          <div class="chat-agent-identity-avatar-wrap">
            <div class="chat-agent-identity-avatar">
              {#if avatarUrl(uiTick)}<img src={avatarUrl(uiTick)} alt={displayName(uiTick) + ' avatar'} loading="lazy" />{:else if displayEmoji(uiTick)}<span>{displayEmoji(uiTick)}</span>{:else}<span class="infring-logo infring-logo--agent-default" aria-hidden="true"><span class="infring-logo-glyph" aria-hidden="true">&infin;</span></span>{/if}
            </div>
            <input type="file" bind:this={fileInput} accept="image/*" style="display:none" on:change={avatarChanged}>
            <div class="chat-agent-avatar-actions">
              <button class="chat-agent-identity-pencil chat-agent-identity-pencil-avatar-action" on:click|stopPropagation={toggleEmojiPicker} title="Pick emoji" aria-label="Pick emoji">{@html icon('emoji')}</button>
              <button class="chat-agent-identity-pencil chat-agent-identity-pencil-avatar-action" on:click|stopPropagation={uploadAvatar} disabled={state.avatarUploading} title="Upload avatar" aria-label="Upload avatar">{@html icon('upload')}</button>
              <button class="chat-agent-identity-pencil chat-agent-identity-pencil-avatar-action" on:click|stopPropagation={toggleAvatarUrlPicker} title="Set avatar URL" aria-label="Set avatar URL">{@html icon('url')}</button>
              {#if state.emojiPickerOpen}
                <div class="chat-agent-emoji-menu">
                  <input class="form-input chat-agent-emoji-search" value={state.emojiSearch} placeholder="Search emoji..." on:input={(event) => setPageValue('drawerEmojiSearch', event.target.value)}>
                  <div class="chat-agent-emoji-grid">{#each state.emojiRows as item (String(item.emoji || '') + '-' + String(item.name || ''))}<button class="emoji-grid-item" on:click|stopPropagation={() => selectEmoji(item)} title={item.name || ''}>{item.emoji || ''}</button>{/each}</div>
                </div>
              {/if}
              {#if state.avatarUrlPickerOpen}
                <div class="chat-agent-url-menu">
                  <input class="form-input chat-agent-url-input" value={state.avatarUrlDraft} placeholder="https://example.com/avatar.png" on:input={(event) => setPageValue('drawerAvatarUrlDraft', event.target.value)} on:keydown={(event) => { if (event.key === 'Enter') { event.preventDefault(); applyAvatarUrl(); } if (event.key === 'Escape') escapeSet('drawerAvatarUrlPickerOpen', false); }}>
                  <div class="chat-agent-url-menu-actions"><button class="btn btn-primary btn-sm" on:click|stopPropagation={applyAvatarUrl}>Use URL</button><button class="btn btn-ghost btn-sm" on:click|stopPropagation={() => setPageValue('drawerAvatarUrlPickerOpen', false)}>Cancel</button></div>
                </div>
              {/if}
            </div>
          </div>
          <div class="chat-agent-identity-name-row"><div class="chat-agent-identity-name">{displayName(uiTick)}</div><button class="chat-agent-identity-pencil" on:click|stopPropagation={() => setPageValue('drawerEditingName', true)} title="Edit name" aria-label="Edit name">{@html icon('edit')}</button></div>
          {#if state.editingEmoji && !state.emojiPickerOpen}<div class="chat-agent-identity-edit"><input class="form-input" style="font-size:12px;max-width:120px" value={state.form.emoji || ''} placeholder="Emoji" on:input={(event) => setFormValue('emoji', event.target.value)} on:keydown={(event) => { if (event.key === 'Enter' || event.key === 'Escape') setPageValue('drawerEditingEmoji', false); }}><button class="btn btn-ghost btn-sm" on:click={() => setPageValue('drawerEditingEmoji', false)} style="padding:2px 8px">Cancel</button></div>{/if}
          {#if state.avatarUploading}<div class="text-xs text-dim">Uploading avatar...</div>{/if}
          {#if state.avatarUploadError}<div class="text-xs text-danger">{state.avatarUploadError}</div>{/if}
          {#if state.editingName}<div class="chat-agent-identity-edit"><input class="form-input" style="font-size:12px;flex:1 1 auto" value={state.form.name || ''} placeholder="Agent name" on:input={(event) => setFormValue('name', event.target.value)} on:keydown={(event) => { if (event.key === 'Enter' || event.key === 'Escape') setPageValue('drawerEditingName', false); }}><button class="btn btn-ghost btn-sm" on:click={() => setPageValue('drawerEditingName', false)} style="padding:2px 8px">Cancel</button></div>{/if}
        </div>
        <div class="tabs chat-agent-drawer-tabs">
          <div class:active={state.tab === 'info'} class="tab" on:click={() => setTab('info')}>Info</div>
          <div class:active={state.tab === 'config'} class="tab" on:click={() => setTab('config')}>Config</div>
          <div class:active={state.tab === 'permissions'} class="tab" on:click={() => setTab('permissions')}>Permissions</div>
        </div>
        {#if state.tab === 'info'}
          <div class="detail-grid">
            <div class="detail-row"><span class="detail-label">ID</span><span class="detail-value text-xs" style="word-break:break-all">{state.agent.id || '-'}</span></div>
            <div class="detail-row"><span class="detail-label">State</span><span class={'badge badge-' + String(state.agent.state || 'unknown').toLowerCase()}>{state.agent.state || 'unknown'}</span></div>
            <div class="detail-row"><span class="detail-label">Mode</span><select class="form-select" style="width:140px" value={state.agent.mode || 'full'} on:change={setMode}><option value="observe">Observe</option><option value="assist">Assist</option><option value="full">Full</option></select></div>
            <div class="detail-row"><span class="detail-label">Profile</span><span class="detail-value">{state.agent.profile || '-'}</span></div>
            <div class="detail-row"><span class="detail-label">Provider</span>{#if !state.editingProvider}<span><span class="detail-value">{state.agent.model_provider || '-'}</span><button class="btn btn-ghost btn-sm" style="margin-left:8px;padding:2px 8px;font-size:11px" on:click={startProviderEdit}>Change</button></span>{:else}<span class="flex gap-1" style="align-items:center"><input class="form-input" style="width:160px;font-size:12px" value={state.newProviderValue} placeholder="provider" on:input={(event) => setPageValue('drawerNewProviderValue', event.target.value)} on:keydown={(event) => { if (event.key === 'Enter' || event.key === 'Escape') setPageValue('drawerEditingProvider', false); }}><button class="btn btn-ghost btn-sm" on:click={() => setPageValue('drawerEditingProvider', false)} style="padding:2px 8px">Cancel</button></span>{/if}</div>
            <div class="detail-row"><span class="detail-label">Model</span>{#if !state.editingModel}<span><span class="detail-value">{state.agent.model_name || '-'}</span><button class="btn btn-ghost btn-sm" style="margin-left:8px;padding:2px 8px;font-size:11px" on:click={startModelEdit}>Change</button></span>{:else}<span class="flex gap-1" style="align-items:center"><input class="form-input" style="width:240px;font-size:12px" value={state.newModelValue} placeholder="provider/model" on:input={(event) => setPageValue('drawerNewModelValue', event.target.value)} on:keydown={(event) => { if (event.key === 'Enter' || event.key === 'Escape') setPageValue('drawerEditingModel', false); }}><button class="btn btn-ghost btn-sm" on:click={() => setPageValue('drawerEditingModel', false)} style="padding:2px 8px">Cancel</button></span>{/if}</div>
            <div class="detail-row"><span class="detail-label">Created</span><span class="detail-value">{createdLabel()}</span></div>
            <div class="detail-row" style="align-items:flex-start"><span class="detail-label">Fallbacks</span><div class="detail-value" style="display:flex;flex-direction:column;gap:4px;align-items:flex-start">{#if fallbackRows().length}<div style="display:flex;flex-direction:column;gap:4px">{#each fallbackRows() as fb, fidx}<div class="flex gap-1 items-center"><span class="badge badge-muted">{(fb.provider || '') + '/' + (fb.model || '')}</span><button class="btn btn-ghost btn-sm" style="padding:1px 4px;font-size:10px;color:var(--danger)" on:click={() => removeFallback(fidx)}>&times;</button></div>{/each}</div>{:else}<span class="text-dim">None</span>{/if}{#if !state.editingFallback}<button class="btn btn-ghost btn-sm" style="padding:2px 8px;font-size:11px;margin-top:4px" on:click={startFallbackEdit}>+ Add</button>{:else}<div class="flex gap-1 mt-1" style="align-items:center"><input class="form-input" style="width:220px;font-size:12px" value={state.newFallbackValue} placeholder="provider/model" on:input={(event) => setPageValue('drawerNewFallbackValue', event.target.value)} on:keydown={(event) => { if (event.key === 'Enter') addFallback(); if (event.key === 'Escape') setPageValue('drawerEditingFallback', false); }}><button class="btn btn-primary btn-sm" on:click={addFallback} style="padding:2px 10px;font-size:11px">Add</button><button class="btn btn-ghost btn-sm" on:click={() => setPageValue('drawerEditingFallback', false)} style="padding:2px 8px;font-size:11px">Cancel</button></div>{/if}</div></div>
          </div>
        {:else if state.tab === 'config'}
          <div>
            <div class="form-group"><label>Name</label><input class="form-input" value={state.form.name || ''} on:input={(event) => setFormValue('name', event.target.value)}></div>
            <div class="form-group"><label>System Prompt</label><textarea class="form-textarea" value={state.form.system_prompt || ''} style="height:96px" on:input={(event) => setFormValue('system_prompt', event.target.value)}></textarea></div>
            <div class="form-group"><label>Emoji</label><input class="form-input" value={state.form.emoji || ''} placeholder="Emoji" on:input={(event) => setFormValue('emoji', event.target.value)}></div>
            <div class="form-group"><label>Color</label><input type="color" value={state.form.color || '#2563EB'} style="width:48px;height:32px;border:none;cursor:pointer;background:none" on:input={(event) => setFormValue('color', event.target.value)}></div>
            <div class="form-group"><label>Archetype</label><select class="form-select" value={state.form.archetype || ''} on:change={(event) => setFormValue('archetype', event.target.value)}><option value="">None</option>{#each state.archetypes as item}<option value={String(item).toLowerCase()}>{item}</option>{/each}</select></div>
            <div class="form-group"><label>Vibe</label><select class="form-select" value={state.form.vibe || ''} on:change={(event) => setFormValue('vibe', event.target.value)}><option value="">None</option>{#each state.vibes as item}<option value={item}>{String(item).charAt(0).toUpperCase() + String(item).slice(1)}</option>{/each}</select></div>
          </div>
        {:else if state.tab === 'permissions'}
          <div class="agent-permissions-grid">
            <div class="agent-permissions-note text-xs text-dim">GitHub-style permission scopes. Set access per scope or bulk-apply by category.</div>
            {#each state.permissionRows as section (section.category)}
              <section class="agent-perm-section">
                <div class="agent-perm-section-head"><div><div class="detail-label">{section.name}</div><div class="text-xs text-dim">{permissionSummary(section, uiTick)}</div></div><div class="agent-perm-bulk"><button class="btn btn-ghost btn-sm" on:click={() => setPermissionCategory(section.category, -1)}>None</button><button class="btn btn-ghost btn-sm" on:click={() => setPermissionCategory(section.category, 0)}>Inherit</button><button class="btn btn-ghost btn-sm" on:click={() => setPermissionCategory(section.category, 1)}>Allow</button></div></div>
                {#each section.permissions || [] as perm (perm.key)}
                  <div class="agent-perm-row"><div class="agent-perm-meta"><div class="agent-perm-label">{perm.label}</div><div class="agent-perm-description text-xs text-dim">{permissionDescription(perm.key)}</div><div class="agent-perm-key text-xs">{perm.key}</div></div><div class="agent-perm-control"><span class={'agent-perm-state ' + permissionStateClass(perm.key, uiTick)}>{permissionStateLabel(perm.key, uiTick)}</span><select class="form-select agent-perm-select" value={String(permissionState(perm.key, uiTick))} on:change={(event) => setPermission(perm.key, event.target.value)}><option value="-1">No access</option><option value="0">Inherited</option><option value="1">Allowed</option></select></div></div>
                {/each}
              </section>
            {/each}
          </div>
        {/if}
        <div class="chat-agent-drawer-save-row"><button class="btn btn-primary chat-agent-drawer-save-btn" on:click={saveAll} disabled={state.savePending || state.loading}>{state.savePending ? 'Saving...' : 'Save'}</button></div>
      {/if}
    </div>
  </aside>
{/if}
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
