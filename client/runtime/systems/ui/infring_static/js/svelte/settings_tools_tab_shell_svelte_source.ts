const COMPONENT_TAG = 'infring-settings-tools-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-settings-tools-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'settings';
  export let tabId = 'tools';
  export let panelRole = 'settings-tab';
  export let routeContract = 'settings:tools';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
