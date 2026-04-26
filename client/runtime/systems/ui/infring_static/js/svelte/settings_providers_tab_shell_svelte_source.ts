const COMPONENT_TAG = 'infring-settings-providers-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-settings-providers-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'settings';
  export let tabId = 'providers';
  export let panelRole = 'settings-tab';
  export let routeContract = 'settings:providers';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
