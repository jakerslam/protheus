const COMPONENT_TAG = 'infring-scheduler-triggers-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-scheduler-triggers-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'scheduler';
  export let tabId = 'triggers';
  export let panelRole = 'scheduler-tab';
  export let routeContract = 'scheduler:triggers';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
