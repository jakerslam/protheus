const COMPONENT_TAG = 'infring-scheduler-jobs-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-scheduler-jobs-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'scheduler';
  export let tabId = 'jobs';
  export let panelRole = 'scheduler-tab';
  export let routeContract = 'scheduler:jobs';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
