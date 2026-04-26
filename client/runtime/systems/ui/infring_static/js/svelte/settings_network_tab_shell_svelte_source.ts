const COMPONENT_TAG = 'infring-settings-network-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-settings-network-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'settings';
  export let tabId = 'network';
  export let panelRole = 'settings-tab';
  export let routeContract = 'settings:network';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
