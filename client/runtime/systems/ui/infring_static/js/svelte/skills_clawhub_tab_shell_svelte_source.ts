const COMPONENT_TAG = 'infring-skills-clawhub-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-skills-clawhub-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'skills';
  export let tabId = 'clawhub';
  export let panelRole = 'skills-tab';
  export let routeContract = 'skills:clawhub';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
