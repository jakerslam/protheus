// Canonical Shell helper source: bootstrap/runtime identity projection helpers.
// Loaded before app.ts by the dashboard asset router.
'use strict';

function infringNormalizeDashboardAssistantIdentity(payload) {
  var source = payload && typeof payload === 'object' ? payload : {};
  var name = normalizeDashboardOptionalString(
    source.name ||
    source.assistant_name ||
    source.display_name ||
    source.label
  );
  var avatar = normalizeDashboardOptionalString(
    source.avatar ||
    source.avatar_url ||
    source.assistant_avatar
  );
  var agentId = normalizeDashboardOptionalString(
    source.agent_id ||
    source.assistant_agent_id ||
    source.id
  );
  return {
    name: name || 'Assistant',
    avatar: avatar || '',
    agentId: agentId || ''
  };
}

function infringApplyBootstrapRuntimeState(target, statusObj, versionObj) {
  var page = target && typeof target === 'object' ? target : {};
  var status = statusObj && typeof statusObj === 'object' ? statusObj : {};
  var version = versionObj && typeof versionObj === 'object' ? versionObj : {};
  var assistantPayload =
    (status.assistant_identity && typeof status.assistant_identity === 'object' && status.assistant_identity) ||
    (status.assistant && typeof status.assistant === 'object' && status.assistant) ||
    (version.assistant_identity && typeof version.assistant_identity === 'object' && version.assistant_identity) ||
    (version.assistant && typeof version.assistant === 'object' && version.assistant) ||
    {
      name: status.assistant_name || version.assistant_name || '',
      avatar: status.assistant_avatar || version.assistant_avatar || '',
      agent_id: status.assistant_agent_id || version.assistant_agent_id || ''
    };
  var assistantIdentity = infringNormalizeDashboardAssistantIdentity(assistantPayload);
  page.assistantName = assistantIdentity.name || page.assistantName || 'Assistant';
  page.assistantAvatar = assistantIdentity.avatar || page.assistantAvatar || null;
  page.assistantAgentId = assistantIdentity.agentId || page.assistantAgentId || null;

  var serverVersion = normalizeDashboardOptionalString(version.version || version.tag || status.version).replace(/^[vV]/, '');
  if (serverVersion) page.serverVersion = serverVersion;

  var previewRoots = status.local_media_preview_roots || version.local_media_preview_roots;
  if (!Array.isArray(previewRoots) && status.media && typeof status.media === 'object') {
    previewRoots = status.media.local_preview_roots;
  }
  if (!Array.isArray(previewRoots) && version.media && typeof version.media === 'object') {
    previewRoots = version.media.local_preview_roots;
  }
  if (Array.isArray(previewRoots)) {
    page.localMediaPreviewRoots = previewRoots
      .map(function(root) { return normalizeDashboardOptionalString(root); })
      .filter(function(root) { return !!root; });
  }

  var sandboxMode = normalizeDashboardOptionalString(
    status.embed_sandbox_mode ||
    (status.embed && status.embed.sandbox_mode) ||
    version.embed_sandbox_mode ||
    (version.embed && version.embed.sandbox_mode)
  );
  if (sandboxMode) page.embedSandboxMode = sandboxMode;

  var allowExternal = status.allow_external_embed_urls;
  if (typeof allowExternal !== 'boolean' && status.embed && typeof status.embed === 'object') {
    allowExternal = status.embed.allow_external_urls;
  }
  if (typeof allowExternal !== 'boolean') {
    allowExternal = version.allow_external_embed_urls;
  }
  if (typeof allowExternal !== 'boolean' && version.embed && typeof version.embed === 'object') {
    allowExternal = version.embed.allow_external_urls;
  }
  if (typeof allowExternal === 'boolean') page.allowExternalEmbedUrls = allowExternal;
}
