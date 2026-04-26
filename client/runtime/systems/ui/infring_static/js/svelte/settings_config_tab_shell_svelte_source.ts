const COMPONENT_TAG = 'infring-settings-config-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-settings-config-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'settings';
  export let tabId = 'config';
  export let panelRole = 'settings-tab';
  export let routeContract = 'settings:config';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
