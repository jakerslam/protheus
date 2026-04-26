const COMPONENT_TAG = 'infring-settings-security-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-settings-security-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'settings';
  export let tabId = 'security';
  export let panelRole = 'settings-tab';
  export let routeContract = 'settings:security';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
