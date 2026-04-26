const COMPONENT_TAG = 'infring-settings-info-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-settings-info-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'settings';
  export let tabId = 'info';
  export let panelRole = 'settings-tab';
  export let routeContract = 'settings:info';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
