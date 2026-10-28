<script lang="ts">
  // La jaquette d'un jeu, servie par le cœur pour le profil ouvert.
  // La carte de remplacement (titre, plateforme) est DERRIÈRE l'image : dès que l'image est là, elle la couvre,
  // quoi qu'il arrive à l'état du composant. Un échec est retenté trois fois, de plus en plus espacées, avant
  // d'abandonner.
  import { adresseJaquette, adresseVisuel } from '$lib/api';
  import { visuelDuJeu, type Dimensions } from './visuels';
  import { nomAffiche } from './ludotheque.svelte';

  let {
    id,
    titre,
    plateforme = '',
    disponible = true,
    largeur,
    visuel,
    images,
  }: {
    id: number;
    titre: string;
    plateforme?: string;
    disponible?: boolean | null;
    /** Largeur affichée, en pixels CSS : Frogtend demande une miniature juste assez grande. */
    largeur?: number;
    /** Le visuel choisi (réglage de la ludothèque) ; la jaquette sinon. */
    visuel?: string;
    /** Les dimensions des visuels du jeu (catalogue) : la place exacte est réservée d'emblée. */
    images?: Dimensions | null;
  } = $props();

  const v = $derived(visuelDuJeu(visuel, images, pixels(largeur) ?? 0));

  /** Pixels réels à l'écran (écran haute densité, échelle du texte). */
  function pixels(l: number | undefined): number | undefined {
    if (!l) return undefined;
    const echelle = Number(getComputedStyle(document.documentElement).getPropertyValue('--echelle')) || 1;
    return l * echelle * (window.devicePixelRatio || 1);
  }

  /** Les nouveaux essais, de plus en plus espacés (essai de Seb, 28/10 : au retour vers la ludothèque, les demandes
   * d'images échouaient un court instant ; un seul nouvel essai à 1,5 s tombait encore dedans, et toutes les
   * jaquettes restaient vides jusqu'au redémarrage). */
  const DELAIS_NOUVEL_ESSAI_MS = [1500, 4000, 10000];
  let essai = $state(0);
  let abandon = $state(false);
  let chargee = $state(false);
  /** Une adresse PROPRE à cet affichage : le navigateur intégré ne ressert jamais l'échec gardé d'une demande
   * annulée quand la grille a été quittée (piste de l'expert, 28/10 : changer de visuel — nouvelle adresse — réparait). */
  const montage = Math.random().toString(36).slice(2, 8);
  let minuteur: ReturnType<typeof setTimeout> | null = null;
  const arreterMinuteur = () => {
    if (minuteur !== null) clearTimeout(minuteur);
    minuteur = null;
  };

  const src = $derived.by(() => {
    const base = v.sorte === 'jaquette' ? adresseJaquette(id, pixels(largeur)) : adresseVisuel(id, v.sorte, pixels(largeur));
    return `${base}${base.includes('?') ? '&' : '?'}m=${montage}${essai === 0 ? '' : `-${essai}`}`;
  });

  // Un autre jeu, ou une autre taille : on repart de zéro (et un essai prévu pour l'ancien est oublié). Au démontage,
  // aucun minuteur ne survit.
  $effect(() => {
    void id;
    void largeur;
    void v.sorte;
    arreterMinuteur();
    chargee = false;
    essai = 0;
    abandon = false;
    return arreterMinuteur;
  });

  function echec() {
    chargee = false;
    const delai = DELAIS_NOUVEL_ESSAI_MS[essai];
    if (delai === undefined) {
      abandon = true;
      return;
    }
    const suivant = essai + 1;
    arreterMinuteur();
    // Un léger décalage : des dizaines de jaquettes ne redemandent jamais toutes à la même milliseconde.
    minuteur = setTimeout(() => {
      minuteur = null;
      if (!abandon) essai = suivant;
    }, delai * (0.9 + Math.random() * 0.2));
  }
</script>

<div class="jaquette" class:chargee style:aspect-ratio={v.cadre}>
  <!-- La boîte au ratio RÉEL de l'image (s'il est connu) : la carte de remplacement a déjà la place de l'image. -->
  <div class="boite" class:ajustee={v.ratio !== null} style:--r={v.ratio ?? undefined}>
    <div class="remplacement" aria-hidden="true">
      <span class="titre">{titre}</span>
      {#if plateforme}<span class="plateforme">{nomAffiche(plateforme)}</span>{/if}
    </div>
    {#if (disponible !== false || v.sorte !== 'jaquette') && !abandon}
      <img {src} alt="" loading="lazy" decoding="async" class:pixels={v.pixels} onload={() => (chargee = true)} onerror={echec} />
    {/if}
  </div>
</div>

<style>
  .jaquette {
    position: relative;
    aspect-ratio: 3 / 4;
    width: 100%;
    container-type: size;
    display: grid;
    place-items: center;
  }
  .boite {
    position: relative;
    width: 100%;
    height: 100%;
    border-radius: calc(10 * var(--u));
    overflow: hidden;
  }
  /* Ratio connu : la plus grande boîte de ce ratio qui tient dans le cadre (comme l'image en « contain »). */
  .boite.ajustee {
    width: min(100cqw, calc(100cqh * var(--r)));
    height: auto;
    aspect-ratio: var(--r);
  }
  img.pixels {
    image-rendering: pixelated;
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
