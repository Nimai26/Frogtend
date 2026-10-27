<script lang="ts">
  // La fiche d'un système (contrat 1.28) dans le panneau de détails : bannière ou logo, fabricant et année,
  // description en français, et les CRÉDITS exigés par leurs licences (auteur, licence, source) là où c'est montré.
  import { adresseImageSysteme, api, type FicheSysteme } from '$lib/api';
  import { ouvrirPageWeb } from '$lib/web';
  import { creditImage, creditTexte, imageEntete, lienSur, sousTitre } from './systemes';

  let { nom }: { nom: string } = $props();

  let fiche = $state<FicheSysteme | null>(null);
  /** Les images qui n'ont pas pu s'afficher (la suivante prend leur place). */
  let echecs = $state<string[]>([]);
  let tout = $state(false);
  /** Le texte replié déborde-t-il vraiment ? Sinon, pas de « Lire la suite » sous un texte court. */
  let paragraphe = $state<HTMLParagraphElement | null>(null);
  let deborde = $state(false);
  $effect(() => {
    const p = paragraphe;
    if (!p) return;
    const mesurer = () => {
      if (!tout) deborde = p.scrollHeight > p.clientHeight + 1;
    };
    mesurer();
    const o = new ResizeObserver(mesurer);
    o.observe(p);
    return () => o.disconnect();
  });

  $effect(() => {
    const n = nom;
    fiche = null;
    echecs = [];
    tout = false;
    api
      .plateformeFiche(n)
      .then((f) => {
        if (n === nom) fiche = f;
      })
      .catch(() => {});
  });

  const entete = $derived(imageEntete(fiche, echecs));
  const sous = $derived(sousTitre(fiche));
  const credit = $derived(entete ? creditImage(entete.image) : null);
  const texte = $derived(creditTexte(fiche));
</script>

{#if fiche}
  <div class="systeme">
    {#if entete}
      <img
        class="entete-systeme"
        src={adresseImageSysteme(nom, entete.sorte, 400, entete.image.empreinte)}
        alt=""
        loading="lazy"
        onerror={() => (echecs = [...echecs, entete!.sorte])}
      />
      {#if credit}
        <p class="credit muted">
          {credit}
          {#if lienSur(entete.image.url_source)}<button class="lien" aria-label="Source de l’image" onclick={() => ouvrirPageWeb(entete!.image.url_source)}>source</button>{/if}
        </p>
      {/if}
    {/if}
    {#if sous}<p class="muted">{sous}</p>{/if}
    {#if fiche.description_fr}
      <p class="description" class:replie={!tout} bind:this={paragraphe}>{fiche.description_fr}</p>
      {#if deborde || tout}
        <button class="lien" aria-expanded={tout} onclick={() => (tout = !tout)}>{tout ? 'Réduire' : 'Lire la suite'}</button>
      {/if}
      {#if texte}
        <p class="credit muted">
          {texte}
          {#if lienSur(fiche.description_source)}<button class="lien" aria-label="Source du texte" onclick={() => ouvrirPageWeb(fiche!.description_source)}>source</button>{/if}
        </p>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .systeme {
    display: grid;
    gap: calc(6 * var(--u));
  }
  .entete-systeme {
    width: 100%;
    max-height: calc(140 * var(--u));
    object-fit: contain;
  }
  p {
    margin: 0;
  }
  .description {
    white-space: pre-line;
  }
  .description.replie {
    display: -webkit-box;
    -webkit-line-clamp: 5;
    line-clamp: 5;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .credit {
    font-size: calc(12 * var(--u));
  }
  .lien {
    background: none;
    border: none;
    padding: 0;
    /* L'encre des liens de la charte (contraste garanti), comme `a` dans base.css. */
    color: var(--accent2-texte);
    text-decoration: underline;
    cursor: pointer;
    font: inherit;
    justify-self: start;
  }
</style>
