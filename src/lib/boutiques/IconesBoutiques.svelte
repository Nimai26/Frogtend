<script lang="ts">
  // 🏷 D'où vient un jeu (Seb, 21/10) : une petite pastille par origine (Steam, GOG, Epic…, Firehouse, Disque),
  // plusieurs si la carte réunit plusieurs copies. Du texte (jamais un logo de marque copié), aux couleurs du thème.
  import { nomOrigine } from './copies';

  let { origines, grand = false }: { origines: string[]; grand?: boolean } = $props();
  const noms = $derived(origines.map(nomOrigine));
</script>

{#if noms.length}
  <span class="origines" class:grand role="img" aria-label={`Vient de ${noms.join(' et ')}`} title={`Vient de ${noms.join(' et ')}`}>
    {#each noms as n (n)}<span class="pastille">{n}</span>{/each}
  </span>
{/if}

<style>
  .origines {
    display: inline-flex;
    flex-wrap: wrap;
    gap: calc(3 * var(--u));
  }
  .pastille {
    font-size: calc(10 * var(--u));
    line-height: 1.4;
    padding: 0 calc(5 * var(--u));
    border-radius: calc(4 * var(--u));
    background: var(--bg2);
    border: 1px solid var(--line);
    color: var(--ink);
    white-space: nowrap;
  }
  .grand .pastille {
    font-size: calc(13 * var(--u));
    padding: calc(1 * var(--u)) calc(7 * var(--u));
  }
</style>
