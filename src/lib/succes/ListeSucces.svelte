<script lang="ts">
  // 🏆 La liste des succès d'un jeu (lot C) : icône (grise tant qu'il n'est pas obtenu), nom, description, date,
  // rareté en mots. Un succès SECRET cache sa description jusqu'à ce qu'il soit obtenu, « Montrer » à la demande
  // (Seb, 21/10). Utilisée au bureau, dans le menu en jeu et en Taodbox (boutons : clavier et manette).
  import { adresseImageParUrl } from '$lib/api';
  import { compte, dateObtenu, libelleRarete, libelleSource, trier, type ListeSucces } from './liste';

  let { liste }: { liste: ListeSucces } = $props();

  const succes = $derived(trier(liste.succes));
  const total = $derived(compte(liste.succes));
  let montres = $state<string[]>([]);
  const anglais = $derived(liste.succes.some((s) => s.en_anglais));
</script>

<div class="succes-liste">
  <p class="resume">
    <strong>{total[0]} / {total[1]} obtenus</strong>
    <span class="muted">· d’après {libelleSource(liste.source)}{anglais ? ' · en anglais : pas de traduction française' : ''}</span>
  </p>
  <div class="bar"><span style="width: {total[1] ? Math.round((total[0] / total[1]) * 100) : 0}%"></span></div>
  <!-- Chaque succès est un bouton : on le parcourt au clavier et à la manette ; sur un secret, il le montre. -->
  <div class="lignes">
    {#each succes as s (s.id)}
      {@const cache = s.secret && !s.obtenu && !montres.includes(s.id)}
      {@const image = s.obtenu ? s.icone : (s.icone_grise ?? s.icone)}
      <button class="btn ligne" class:obtenu={s.obtenu} onclick={() => cache && (montres = [...montres, s.id])}>
        {#if image}
          <img src={adresseImageParUrl(image)} alt="" class:gris={!s.obtenu && !s.icone_grise} loading="lazy" />
        {:else}
          <span class="icone-vide" aria-hidden="true">{s.obtenu ? '🏆' : '🔒'}</span>
        {/if}
        <span class="texte">
          <strong>{cache ? '🔒 Succès secret' : s.nom}</strong>
          {#if cache}
            <span class="muted">La description apparaîtra quand tu l’auras obtenu. (Choisis-le pour la montrer.)</span>
          {:else if s.description}
            <span class="muted">{s.description}</span>
          {/if}
          <span class="details">
            {#if s.obtenu}<span class="tag ok">✓ obtenu{dateObtenu(s.quand) ? ` le ${dateObtenu(s.quand)}` : ''}</span>{/if}
            {#if libelleRarete(s.rarete)}<span class="muted">{libelleRarete(s.rarete)}</span>{/if}
          </span>
        </span>
      </button>
    {/each}
  </div>
</div>

<style>
  .succes-liste {
    display: grid;
    gap: calc(8 * var(--u));
  }
  .resume {
    margin: 0;
  }
  .lignes {
    display: grid;
    gap: calc(8 * var(--u));
  }
  .ligne {
    display: flex;
    gap: calc(12 * var(--u));
    align-items: flex-start;
    justify-content: flex-start;
    text-align: left;
    width: 100%;
    padding: calc(8 * var(--u));
  }
  /* Pas encore obtenu : l'icône s'efface un peu, jamais le texte (lisible, même surligné). */
  .ligne:not(.obtenu) img,
  .ligne:not(.obtenu) .icone-vide {
    opacity: 0.6;
  }
  /* La ligne choisie (clavier, manette) se remplit de la couleur d'accent : ses textes prennent sa couleur. */
  .ligne:global([data-choisi]) :is(.muted, .tag),
  .ligne:focus-visible :is(.muted, .tag) {
    color: inherit;
  }
  img,
  .icone-vide {
    width: calc(48 * var(--u));
    height: calc(48 * var(--u));
    flex: none;
    border-radius: calc(6 * var(--u));
    display: grid;
    place-items: center;
    font-size: calc(24 * var(--u));
  }
  img.gris {
    filter: grayscale(1);
  }
  .texte {
    display: grid;
    gap: calc(3 * var(--u));
    justify-items: start;
  }
  .details {
    display: flex;
    flex-wrap: wrap;
    gap: calc(8 * var(--u));
    align-items: center;
  }
</style>
