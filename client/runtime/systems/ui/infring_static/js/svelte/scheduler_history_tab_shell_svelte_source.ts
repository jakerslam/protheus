const COMPONENT_TAG = 'infring-scheduler-history-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-scheduler-history-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'scheduler';
  export let tabId = 'history';
  export let panelRole = 'scheduler-tab';
  export let routeContract = 'scheduler:history';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
