<script lang="ts">
  // 📖 La fiche d'un jeu de boutique (lot D, Seb 21/10 : prise DANS les boutiques) : la fiche française de la boutique
  // Steam, sinon celle que GOG Galaxy garde sur ce PC (en anglais). Résumé, genres, développeur, éditeur, sortie,
  // captures. Rien n'est écrit chez les boutiques.
  import { adresseImageParUrl, api, type FicheBoutique, type JeuResume } from '$lib/api';

  let { jeu }: { jeu: JeuResume } = $props();

  let fiche = $state<FicheBoutique | null>(null);

  $effect(() => {
    const j = jeu;
    fiche = null;
    const cle = j.cle_boutique;
    if (!cle) return;
    const source = j.source === 'steam' ? 'steam' : 'galaxy';
    let annule = false;
    // Une courte attente : en faisant défiler la grille, seule la carte où l'on s'arrête est demandée (la boutique Steam
    // limite le nombre de demandes).
    const minuterie = setTimeout(() => {
      api
        .boutiqueFiche(source, cle)
        .then((f) => {
          if (!annule) fiche = f;
        })
        .catch(() => {});
    }, 300);
    return () => {
      annule = true;
      clearTimeout(minuterie);
    };
  });
</script>

{#if fiche}
  <div class="fiche cx-block">
    {#if fiche.resume}
      <p class="resume">{fiche.resume}</p>
      {#if fiche.en_anglais}<p class="muted">En anglais : la boutique n’a pas de texte français.</p>{/if}
    {/if}
    {#if fiche.genres.length}
      <div class="genres">{#each fiche.genres as g (g)}<span class="tag">{g}</span>{/each}</div>
    {/if}
    <dl class="cx-kv">
      {#if fiche.developpeurs.length}<dt>Développeur</dt><dd>{fiche.developpeurs.join(', ')}</dd>{/if}
      {#if fiche.editeurs.length}<dt>Éditeur</dt><dd>{fiche.editeurs.join(', ')}</dd>{/if}
      {#if fiche.sortie ?? fiche.annee}<dt>Sortie</dt><dd>{fiche.sortie ?? fiche.annee}</dd>{/if}
    </dl>
    {#if fiche.captures.length}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div class="captures" aria-label="Captures d’écran" tabindex="0">
        {#each fiche.captures as [mini] (mini)}
          <img src={adresseImageParUrl(mini)} alt="" loading="lazy" />
        {/each}
      </div>
    {/if}
    <p class="muted source">D’après {fiche.source === 'steam' ? 'la boutique Steam' : 'GOG Galaxy'}.</p>
  </div>
{/if}

<style>
  .fiche {
    display: grid;
    gap: calc(8 * var(--u));
  }
  .resume {
    margin: 0;
    line-height: 1.5;
  }
  p {
    margin: 0;
  }
  .genres {
    display: flex;
    flex-wrap: wrap;
    gap: calc(4 * var(--u));
  }
  .captures {
    display: flex;
    gap: calc(6 * var(--u));
    overflow-x: auto;
    padding-bottom: calc(4 * var(--u));
  }
  .captures img {
    height: calc(90 * var(--u));
    border-radius: calc(4 * var(--u));
    display: block;
  }
  .source {
    font-size: calc(12 * var(--u));
  }
</style>
