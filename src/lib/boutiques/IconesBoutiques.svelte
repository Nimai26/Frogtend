<script lang="ts">
  // 🏷 D'où vient un jeu (Seb, 21/10) : une petite pastille par origine (Steam, GOG, Epic…, Firehouse, Disque),
  // plusieurs si la carte réunit plusieurs copies. Seb, 19/11 : l'icône du LANCEUR installé sur ce PC quand Windows la
  // donne (jamais un logo de marque copié dans Frogtend, voir icones.svelte.ts), sinon le texte, aux couleurs du thème.
  import { nomOrigine, venantDe } from './copies';
  import { demanderIcone, iconeDe } from './icones.svelte';

  let { origines, grand = false }: { origines: string[]; grand?: boolean } = $props();
  const pastilles = $derived(origines.map((o) => ({ o, nom: nomOrigine(o), icone: iconeDe(o) })));
  const libelle = $derived(venantDe(pastilles.map((p) => p.nom)));

  $effect(() => {
    for (const o of origines) demanderIcone(o);
  });
</script>

{#if pastilles.length}
  <span class="origines" class:grand role="img" aria-label={libelle} title={libelle}>
    {#each pastilles as p (p.o)}
      {#if p.icone}<span class="pastille avec-icone"><img src={p.icone} alt="" draggable="false" /></span>{:else}<span class="pastille">{p.nom}</span>{/if}
    {/each}
  </span>
{/if}

<style>
  .origines {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
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
  /* L'icône occupe la hauteur de la ligne de texte (10 × 1,4 ≈ 14) : une pastille icône a la taille d'une pastille
     texte, sur la jaquette de Taodbox comme dans la grille. */
  .avec-icone {
    display: inline-flex;
    align-items: center;
    padding: calc(1 * var(--u)) calc(2 * var(--u));
  }
  .avec-icone img {
    display: block;
    width: calc(12 * var(--u));
    height: calc(12 * var(--u));
    object-fit: contain;
  }
  .grand .pastille {
    font-size: calc(13 * var(--u));
    padding: calc(1 * var(--u)) calc(7 * var(--u));
  }
  .grand .avec-icone {
    padding: calc(2 * var(--u)) calc(3 * var(--u));
  }
  .grand .avec-icone img {
    width: calc(16 * var(--u));
    height: calc(16 * var(--u));
  }
</style>
