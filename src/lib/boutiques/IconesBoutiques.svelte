<script lang="ts">
  // 🏷 D'où vient un jeu (Seb, 21/10) : une petite pastille par origine (Steam, GOG, Epic…, Firehouse, Disque),
  // plusieurs si la carte réunit plusieurs copies. Seb, 19/11 : l'icône du LANCEUR installé sur ce PC quand Windows la
  // donne (jamais un logo de marque copié dans Frogtend, voir icones.svelte.ts), sinon le texte, aux couleurs du thème.
  import { nomOrigine, venantDe } from './copies';
  import { demanderIcone, iconeDe } from './icones.svelte';

  let { origines, grand = false }: { origines: string[]; grand?: boolean } = $props();
  const pastilles = $derived(origines.map((o) => ({ o, nom: nomOrigine(o), icone: iconeDe(o) })));
  const libelle = $derived(venantDe(pastilles.map((p) => p.nom)));

  /** Le logo de Firehouse n'a pas pu être lu (fabrication sans lui) : le texte à la place. */
  let firehouseAbsent = $state(false);

  $effect(() => {
    for (const o of origines) demanderIcone(o);
  });
</script>

{#if pastilles.length}
  <span class="origines" class:grand role="img" aria-label={libelle} title={libelle}>
    {#each pastilles as p (p.o)}
      {#if p.icone}<span class="pastille avec-icone"><img src={p.icone} alt="" draggable="false" /></span>
      {:else if p.o === 'firehouse' && !firehouseAbsent}
        <!-- Le logo de Firehouse (Seb 19/11, son choix, gardé dans le dépôt) ;
             absent, le nom écrit revient. -->
        <span class="pastille avec-icone"><img src="/icones/firehouse.png" alt="" draggable="false" onerror={() => (firehouseAbsent = true)} /></span>
      {:else if p.o === 'disque'}
        <!-- Un jeu importé du disque (Seb 19/11) : un disque dur DESSINÉ par Frogtend (aucune marque), à la couleur du
             texte du thème. -->
        <span class="pastille avec-icone"><svg viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="6" width="18" height="12" rx="2" /><line x1="3" y1="13" x2="21" y2="13" /><circle cx="17" cy="15.5" r="0.6" fill="currentColor" /><line x1="6.5" y1="15.5" x2="11" y2="15.5" /></svg></span>
      {:else}<span class="pastille">{p.nom}</span>{/if}
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
  .avec-icone img,
  .avec-icone svg {
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
  .grand .avec-icone img,
  .grand .avec-icone svg {
    width: calc(16 * var(--u));
    height: calc(16 * var(--u));
  }
</style>
