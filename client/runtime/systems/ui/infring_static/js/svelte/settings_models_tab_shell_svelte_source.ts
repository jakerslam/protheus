const COMPONENT_TAG = 'infring-settings-models-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-settings-models-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'settings';
  export let tabId = 'models';
  export let panelRole = 'settings-tab';
  export let routeContract = 'settings:models';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
