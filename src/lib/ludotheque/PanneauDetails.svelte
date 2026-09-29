<script lang="ts">
  // La colonne de droite : le jeu sélectionné (jaquette, informations, actions) ; sans sélection, la plateforme
  // choisie et un « Jeu au hasard ».
  import { goto } from '$app/navigation';
  import { categorieDe, ICONES_CATEGORIES } from './categories';
  import Jaquette from './Jaquette.svelte';
  import { jeuAuHasard, ludo } from './ludotheque.svelte';

  const j = $derived(ludo.selection);
  const plateforme = $derived(ludo.plateformes.find((p) => p.nom === ludo.plateforme));
</script>

<aside class="colonne" aria-label="Détails">
  {#if j}
    <div class="jaquette"><Jaquette id={j.id} titre={j.titre} plateforme={j.plateforme} /></div>
    <h2>{j.titre}</h2>
    <div class="actions">
      <button class="btn primary grand" onclick={() => goto(`/jeu/${j.id}`)}>📄 Voir la fiche</button>
      <button class="btn grand" disabled title="L’installation arrive au lot 2">⬇ Installer (bientôt)</button>
    </div>
    <dl class="cx-kv">
      <dt>Plateforme</dt>
      <dd>{j.plateforme}</dd>
      {#if j.annee}<dt>Année</dt><dd>{j.annee}</dd>{/if}
      {#if j.developpeur}<dt>Développeur</dt><dd>{j.developpeur}</dd>{/if}
      {#if j.editeur}<dt>Éditeur</dt><dd>{j.editeur}</dd>{/if}
    </dl>
    {#if j.genres.length}
      <div class="genres">
        {#each j.genres as g (g)}<span class="tag">{g}</span>{/each}
      </div>
    {/if}
  {:else}
    <div class="entete">
      <span class="icone" aria-hidden="true">{ludo.plateforme ? ICONES_CATEGORIES[categorieDe(ludo.plateforme)] : '🗂'}</span>
      <h2>{ludo.plateforme ?? 'Toute la ludothèque'}</h2>
      <p class="muted">
        {#if ludo.plateforme}
          {categorieDe(ludo.plateforme)} · {(plateforme?.jeux ?? 0).toLocaleString('fr-FR')} jeu(x)
        {:else}
          {ludo.totalLudotheque.toLocaleString('fr-FR')} jeu(x) sur {ludo.plateformes.length} plateforme(s)
        {/if}
      </p>
    </div>
    <button class="btn hasard" onclick={jeuAuHasard} disabled={ludo.totalLudotheque === 0}>
      <span class="de" aria-hidden="true">🎲</span>
      Jeu au hasard
    </button>
    <p class="muted aide">Choisis un jeu dans la grille pour voir ses informations ici.</p>
  {/if}
</aside>

<style>
  .colonne {
    display: flex;
    flex-direction: column;
    gap: calc(12 * var(--u));
    overflow-y: auto;
    min-height: 0;
    padding: calc(4 * var(--u));
  }
  .jaquette {
    width: min(100%, calc(260 * var(--u)));
    align-self: center;
  }
  h2 {
    margin: 0;
    font-size: calc(22 * var(--u));
    line-height: 1.2;
  }
  .actions {
    display: grid;
    gap: calc(8 * var(--u));
  }
  .grand {
    min-height: calc(44 * var(--u));
    font-size: calc(14 * var(--u));
  }
  .genres {
    display: flex;
    flex-wrap: wrap;
    gap: calc(6 * var(--u));
  }
  .entete {
    text-align: center;
    padding: calc(20 * var(--u)) calc(10 * var(--u)) calc(6 * var(--u));
    background: linear-gradient(160deg, color-mix(in srgb, var(--accent2) 25%, transparent), color-mix(in srgb, var(--accent) 18%, transparent));
    border-radius: calc(14 * var(--u));
  }
  .entete .icone {
    font-size: calc(48 * var(--u));
  }
  .entete h2 {
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-top: calc(8 * var(--u));
  }
  .hasard {
    flex-direction: column;
    min-height: calc(110 * var(--u));
    font-size: calc(13 * var(--u));
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .de {
    font-size: calc(34 * var(--u));
  }
  .aide {
    text-align: center;
  }
</style>
