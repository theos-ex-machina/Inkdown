<script lang="ts">
  import { toasts } from "../lib/toast.svelte";
</script>

<div class="toast-stack" aria-live="polite">
  {#each toasts.toasts as t (t.id)}
    <button class="toast {t.kind}" onclick={() => toasts.dismiss(t.id)}>
      {t.message}
    </button>
  {/each}
</div>
