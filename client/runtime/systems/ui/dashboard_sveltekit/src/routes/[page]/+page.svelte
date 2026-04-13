<script lang="ts">
  import LegacyPageFrame from '$lib/components/LegacyPageFrame.svelte';
  import type { DashboardPage } from '$lib/dashboard';

  export let data: {
    page: DashboardPage;
  };
</script>

<LegacyPageFrame page={data.page} />
