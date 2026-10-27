<script lang="ts">
  // Le résumé d'un jeu importé de ton disque, trouvé dans la base de Firehouse (contrat 1.32) : titre, résumé, genres.
  // Rien tant que le jeu n'est pas identifié. Un résumé en anglais est dit, et replié (Seb ne lit pas l'anglais).
  import { api } from '$lib/api';
  import type { FicheBase } from '$lib/import/identification';

  let { jeu, genresConnus }: { jeu: number; genresConnus: boolean } = $props();
  let fiche = $state<FicheBase | null>(null);

  $effect(() => {
    const id = jeu;
    fiche = null;
    api.ficheBase(id).then((f) => {
      if (id === jeu) fiche = f;
    }).catch(() => {});
  });
</script>

{#if fiche}
  <div class="base">
    {#if fiche.titre_fr && fiche.titre_fr !== fiche.titre}<p><strong>{fiche.titre_fr}</strong> <span class="muted">({fiche.titre})</span></p>{/if}
    {#if fiche.resume}
      {#if fiche.resume_langue && fiche.resume_langue !== 'fr'}
        <p class="muted">Résumé {fiche.resume_langue === 'en' ? 'en anglais' : 'dans une autre langue'} : Firehouse n’en a pas encore de version française.</p>
        <details><summary>Lire le résumé</summary><p class="resume">{fiche.resume}</p></details>
      {:else}
        <p class="resume">{fiche.resume}</p>
      {/if}
    {/if}
    {#if !genresConnus && fiche.genres?.length}
      <div class="genres">
        {#each fiche.genres as g (g)}<span class="tag">{g}</span>{/each}
      </div>
    {/if}
    <p class="muted">Fiche et images : base de Firehouse.</p>
  </div>
{/if}

<style>
  .base {
    display: grid;
    gap: calc(6 * var(--u));
  }
  .resume {
    white-space: pre-line;
  }
  .genres {
    display: flex;
    flex-wrap: wrap;
    gap: calc(6 * var(--u));
  }
</style>
