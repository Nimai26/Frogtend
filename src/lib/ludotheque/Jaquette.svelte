<script lang="ts">
  // La jaquette d'un jeu, servie par le cœur pour le profil ouvert.
  // La carte de remplacement (titre, plateforme) est DERRIÈRE l'image : dès que l'image est là, elle la couvre,
  // quoi qu'il arrive à l'état du composant. Un échec est retenté une fois avant d'abandonner.
  import { adresseJaquette } from '$lib/api';

  let {
    id,
    titre,
    plateforme = '',
    disponible = true,
    largeur,
  }: {
    id: number;
    titre: string;
    plateforme?: string;
    disponible?: boolean | null;
    /** Largeur affichée, en pixels CSS : Frogtend demande une miniature juste assez grande. */
    largeur?: number;
  } = $props();

  /** Pixels réels à l'écran (écran haute densité, échelle du texte). */
  function pixels(l: number | undefined): number | undefined {
    if (!l) return undefined;
    const echelle = Number(getComputedStyle(document.documentElement).getPropertyValue('--echelle')) || 1;
    return l * echelle * (window.devicePixelRatio || 1);
  }

  const DELAI_NOUVEL_ESSAI_MS = 1500;
  let essai = $state(0);
  let abandon = $state(false);
  let chargee = $state(false);

  const src = $derived.by(() => {
    const base = adresseJaquette(id, pixels(largeur));
    return essai === 0 ? base : `${base}${base.includes('?') ? '&' : '?'}essai=${essai}`;
  });

  // Un autre jeu, ou une autre taille : on repart de zéro.
  $effect(() => {
    void id;
    void largeur;
    essai = 0;
    abandon = false;
  });

  function echec() {
    chargee = false;
    if (essai === 0) setTimeout(() => (essai = 1), DELAI_NOUVEL_ESSAI_MS);
    else abandon = true;
  }
</script>

<div class="jaquette" class:chargee>
  <div class="remplacement" aria-hidden="true">
    <span class="titre">{titre}</span>
    {#if plateforme}<span class="plateforme">{plateforme}</span>{/if}
  </div>
  {#if disponible !== false && !abandon}
    <img {src} alt="" loading="lazy" decoding="async" onload={() => (chargee = true)} onerror={echec} />
  {/if}
</div>

<style>
  .jaquette {
    position: relative;
    aspect-ratio: 3 / 4;
    width: 100%;
    border-radius: calc(10 * var(--u));
    overflow: hidden;
  }
  /* L'image n'apparaît qu'une fois chargée : jamais d'icône « image cassée » par-dessus la carte. */
  img {
    opacity: 0;
    position: absolute;
    inset: 0;
    z-index: 1;
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .remplacement {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: calc(4 * var(--u));
    padding: calc(12 * var(--u));
    background: linear-gradient(
      160deg,
      color-mix(in srgb, var(--accent2) 45%, var(--panel)),
      color-mix(in srgb, var(--accent) 35%, var(--panel))
    );
    border: 1px solid var(--line);
    border-radius: inherit;
  }
  /* Image chargée : elle apparaît, et la carte de remplacement s'efface (une jaquette en largeur laisse des
     bandes vides). */
  .chargee img {
    opacity: 1;
  }
  .chargee .remplacement {
    visibility: hidden;
  }
  .titre {
    font-weight: 700;
    font-size: calc(15 * var(--u));
    color: var(--ink);
    line-height: 1.2;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 4;
    line-clamp: 4;
    -webkit-box-orient: vertical;
  }
  .plateforme {
    font-size: calc(11 * var(--u));
    color: var(--dim);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
</style>
