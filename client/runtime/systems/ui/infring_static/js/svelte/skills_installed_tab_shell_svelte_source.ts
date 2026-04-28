const COMPONENT_TAG = 'infring-skills-installed-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-skills-installed-tab-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'skills';
  export let tabId = 'installed';
  export let panelRole = 'skills-tab';
  export let routeContract = 'skills:installed';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
