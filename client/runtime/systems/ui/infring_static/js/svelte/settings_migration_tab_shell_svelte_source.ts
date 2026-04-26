const COMPONENT_TAG = 'infring-settings-migration-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-settings-migration-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'settings';
  export let tabId = 'migration';
  export let panelRole = 'settings-tab';
  export let routeContract = 'settings:migration';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
