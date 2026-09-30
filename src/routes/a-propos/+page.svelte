<script lang="ts">
  // Ancienne page : elle vit maintenant dans les Options.
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  onMount(() => goto('/reglages?rubrique=a-propos', { replaceState: true }));
</script>
