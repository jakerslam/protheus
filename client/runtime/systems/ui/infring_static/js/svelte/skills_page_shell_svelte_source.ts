const COMPONENT_TAG = 'infring-skills-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement="infring-skills-page-shell" />
<script>
  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'skills';
  export let panelRole = 'page';
  export let routeContract = 'skills';
  export let parentOwnedData = true;
</script>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
