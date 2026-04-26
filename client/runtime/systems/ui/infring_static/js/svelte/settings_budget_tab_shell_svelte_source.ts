const COMPONENT_TAG = 'infring-settings-budget-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-settings-budget-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'settings';
  export let tabId = 'budget';
  export let panelRole = 'settings-tab';
  export let routeContract = 'settings:budget';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
