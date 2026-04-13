<script lang="ts">
  import type { DashboardPage } from '$lib/dashboard';
  import { dashboardClassicHref, dashboardEmbeddedFallbackHref } from '$lib/dashboard';

  export let page: DashboardPage;

  $: frameTitle = `${page.title} fallback`;
  $: frameSrc = dashboardEmbeddedFallbackHref(page.key);
  $: fullHref = dashboardClassicHref(page.key);
</script>

<section class="fallback">
  <div class="notice">
    <div>
      <h2>{page.title}</h2>
      <p>This view still runs through the classic dashboard while we migrate the highest-churn screens into SvelteKit.</p>
    </div>
    <a href={fullHref}>Open full classic view</a>
  </div>
  <iframe title={frameTitle} src={frameSrc}></iframe>
</section>

<style>
  .fallback {
    display: grid;
    gap: 16px;
    min-height: calc(100vh - 180px);
  }

  .notice {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 18px 20px;
    border-radius: 22px;
    border: 1px solid rgba(158, 188, 255, 0.16);
    background: rgba(11, 22, 39, 0.82);
  }

  .notice h2 {
    margin: 0 0 6px;
    font-size: 1rem;
  }

  .notice p {
    margin: 0;
    color: #bdd0f0;
  }

  .notice a {
    color: inherit;
    text-decoration: none;
    border-radius: 16px;
    padding: 0.75rem 1rem;
    border: 1px solid rgba(158, 188, 255, 0.18);
    background: rgba(255, 255, 255, 0.04);
    white-space: nowrap;
  }

  iframe {
    width: 100%;
    min-height: calc(100vh - 260px);
    border: 1px solid rgba(158, 188, 255, 0.12);
    border-radius: 24px;
    background: rgba(11, 22, 39, 0.72);
  }

  @media (max-width: 760px) {
    .notice {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
