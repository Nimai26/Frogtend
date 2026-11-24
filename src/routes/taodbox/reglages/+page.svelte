<script lang="ts">
  // ⚙ Réglages du jeu, au canapé (✅ Seb 18/11 : « Il faut pouvoir faire des réglages depuis le canapé voyons ») : les
  // interrupteurs des mises à jour et DLC Switch et les mods, entièrement à la manette (croix, A = basculer / valider,
  // B = retour). `?jeu=<id>&titre=…` ; sous /taodbox, le mode canapé (échelle ×2, manette, plein écran) reste actif.
  // Les MÊMES blocs que le panneau du bureau (BlocContenus, BlocMods) : mêmes règles, mêmes fenêtres maison.
  import { page } from '$app/state';
  import { tick } from 'svelte';
  import BlocContenus from '$lib/contenus/BlocContenus.svelte';
  import { etat, reglerProfil } from '$lib/etat.svelte';
  import { tele } from '$lib/ludotheque/telechargements.svelte';
  import { aDesEmplacements } from '$lib/parties/parties';
  import { fenetreOuverte, selectionner } from '$lib/taodbox/manette.svelte';

  const jeu = $derived(Number(page.url.searchParams.get('jeu')));
  const titre = $derived(page.url.searchParams.get('titre') ?? tele.jeux[jeu]?.titre ?? '');
  // Un jeu inconnu de ce PC : aucune plateforme (jamais « Switch » par défaut : rien n'est lu pour un autre jeu).
  const plateforme = $derived(tele.jeux[jeu]?.plateforme);
  const connu = $derived(Number.isFinite(jeu) && jeu !== 0);
  const auto = $derived(!!etat.profil.modsSwitch?.auto);
  // Lot S3 (Seb 23/11) : l'abri des parties après chaque partie, réglable aussi au canapé (un jeu PC seulement).
  const jeuPc = $derived(aDesEmplacements(plateforme));
  const abriApres = $derived(etat.profil.parties?.abriApresPartie !== false);
  // Lot S7 (Seb 24/11) : rattraper à l'ouverture du profil les parties jouées hors de Frogtend (à part de l'abri après
  // une partie), réglable aussi au canapé.
  const rattrapage = $derived(etat.profil.parties?.rattrapage !== false);
  let vide = $state(false);
  let enregistrement = $state(false);
  let page_ = $state<HTMLElement | null>(null);

  // Le focus part sur « ← Retour » : la croix descend ensuite vers les interrupteurs (jamais un écran sans focus).
  $effect(() => {
    void jeu;
    tick().then(() => selectionner(page_?.querySelector<HTMLButtonElement>('button')));
  });

  /** « Mettre à jour tout seul les mods » : le réglage du profil, défait d'ici aussi (pas d'options à la souris au
   * canapé). */
  async function basculerAuto() {
    if (enregistrement) return;
    enregistrement = true;
    try {
      await reglerProfil('modsSwitch.auto', !auto);
    } finally {
      enregistrement = false;
    }
  }

  /** « Mettre mes parties à l'abri après chaque partie » : le réglage du profil (pour tous ses jeux PC). */
  async function basculerAbri() {
    if (enregistrement) return;
    enregistrement = true;
    try {
      await reglerProfil('parties.abriApresPartie', !abriApres);
    } finally {
      enregistrement = false;
    }
  }

  /** « Rattraper les parties jouées hors de Frogtend » : le réglage du profil (pour tous ses jeux PC). */
  async function basculerRattrapage() {
    if (enregistrement) return;
    enregistrement = true;
    try {
      await reglerProfil('parties.rattrapage', !rattrapage);
    } finally {
      enregistrement = false;
    }
  }

  /** Échap (ou B à la manette) : revenir à la grille, sauf si une fenêtre maison est ouverte (elle se ferme d'abord). */
  function touche(e: KeyboardEvent) {
    if (e.key !== 'Escape' || fenetreOuverte()) return;
    e.preventDefault();
    history.back();
  }
</script>

<svelte:window onkeydown={touche} />

<div class="page-reglages" bind:this={page_}>
  <header>
    <button class="btn" onclick={() => history.back()}>← Retour</button>
    <h1>⚙ Réglages du jeu{titre ? ` — ${titre}` : ''}</h1>
  </header>
  {#if connu}
    {#if plateforme === 'Nintendo Switch'}
      <div class="auto">
        <span id="auto-libelle">🧩 Mettre à jour tout seul les mods déjà téléchargés (pour tous tes jeux)</span>
        <button
          class="btn"
          role="switch"
          aria-checked={auto}
          aria-labelledby="auto-libelle"
          aria-describedby="auto-etat"
          aria-disabled={enregistrement}
          onclick={basculerAuto}
        >
          <span id="auto-etat">{auto ? '✅ Oui' : '⬜ Non : Frogtend demande'}</span>
        </button>
      </div>
    {/if}
    {#if jeuPc}
      <div class="auto">
        <span id="abri-libelle">🛟 Mettre mes parties à l’abri sur ce PC après chaque partie (pour tous tes jeux PC)</span>
        <button
          class="btn"
          role="switch"
          aria-checked={abriApres}
          aria-labelledby="abri-libelle"
          aria-describedby="abri-etat"
          aria-disabled={enregistrement}
          onclick={basculerAbri}
        >
          <span id="abri-etat">{abriApres ? '✅ Oui (conseillé)' : '⬜ Non'}</span>
        </button>
      </div>
      <div class="auto">
        <span id="rattrapage-libelle">🛟 Rattraper les parties jouées hors de Frogtend (pour tous tes jeux PC)</span>
        <button
          class="btn"
          role="switch"
          aria-checked={rattrapage}
          aria-labelledby="rattrapage-libelle"
          aria-describedby="rattrapage-etat"
          aria-disabled={enregistrement}
          onclick={basculerRattrapage}
        >
          <span id="rattrapage-etat">{rattrapage ? '✅ Oui (conseillé)' : '⬜ Non'}</span>
        </button>
      </div>
    {/if}
    <BlocContenus id={jeu} {titre} {plateforme} deplie bind:vide />
    {#if vide && !jeuPc}<p class="muted">Rien à régler pour ce jeu : il n’a ni mise à jour, ni DLC, ni mod.</p>{/if}
  {:else}
    <p class="muted">Ce jeu n’est pas sur ce PC.</p>
  {/if}
  <footer class="aide">🎮 Croix pour choisir · A pour allumer, éteindre ou valider · B pour revenir</footer>
</div>

<style>
  /* Marge de sécurité de la télé (charte § 7) ; la page défile, la manette suit le focus. */
  .page-reglages {
    display: grid;
    gap: calc(14 * var(--u));
    padding: 5vh 5vw;
    max-width: calc(900 * var(--u));
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
    align-content: start;
    overflow: auto;
    height: 100vh;
  }
  header {
    display: flex;
    align-items: center;
    gap: calc(14 * var(--u));
    flex-wrap: wrap;
  }
  h1 {
    margin: 0;
    font-size: calc(22 * var(--u));
    overflow-wrap: anywhere;
  }
  .auto {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: calc(10 * var(--u));
    align-items: center;
  }
  .aide {
    color: var(--dim);
    text-align: center;
  }
</style>
