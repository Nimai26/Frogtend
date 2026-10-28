<script lang="ts">
  // L'écran de lancement (lot 4c, ✅ Seb 28/10 : « un splash screen pendant qu'un jeu lent démarre ») : dans la fenêtre
  // de Frogtend, seulement si le jeu met plus de 2 secondes à ouvrir sa fenêtre (sinon il clignoterait) ; caché dès
  // que le cœur voit la fenêtre du jeu (`partie-affichee`), à la fin de la partie, ou par la personne (bouton, B /
  // Échap). Il ne prend jamais le premier plan : le jeu le prend en s'ouvrant. Pendant l'attente, la page dessous est
  // inerte (`attenteLancement.montre`, lu par la mise en page) : un A ne relance rien en aveugle.
  import { tick } from 'svelte';
  import { adresseFondDuJeu } from '$lib/api';
  import { fenetreOuverte, selectionner } from '$lib/taodbox/manette.svelte';
  import { attenteLancement, partie } from './jeu.svelte';
  import { tele } from './telechargements.svelte';

  /** Le délai avant de montrer l'attente (un jeu rapide ne la voit jamais). */
  const DELAI = 2000;

  let visible = $state(false);
  /** L'attente masquée par la personne (remise à zéro à chaque nouvelle partie). */
  let masque = $state(false);
  let fondCharge = $state(false);
  let bouton = $state<HTMLButtonElement | null>(null);

  const jeu = $derived(partie.enJeu);
  const j = $derived(jeu !== null ? tele.jeux[jeu] : undefined);
  const titre = $derived(j?.titre_fiche || j?.titre || 'Le jeu');
  // Le fond du jeu : sa fiche de Firehouse (un jeu importé relié : celle de sa fiche) ; sinon le fond du thème.
  const fiche = $derived(j?.fiche ?? (jeu !== null && jeu > 0 ? jeu : null));
  const fond = $derived(fiche !== null ? adresseFondDuJeu(fiche) : null);

  // Ne dépend que de valeurs simples (leçon du 28/10 sur les effets abonnés à un objet).
  $effect(() => {
    const [n, affichee] = [partie.enJeu, partie.affichee];
    visible = false;
    masque = false;
    fondCharge = false;
    if (n === null || affichee) return;
    const t = setTimeout(() => (visible = true), DELAI);
    return () => clearTimeout(t);
  });

  const montre = $derived(visible && jeu !== null && !masque);

  $effect(() => {
    attenteLancement.montre = montre;
    // La sélection sur le seul bouton (la page dessous est inerte).
    if (montre) void tick().then(() => selectionner(bouton));
  });

  function masquer() {
    masque = true;
  }

  /** B (Échap) masque l'attente — avant la page dessous (Taodbox reviendrait en arrière), jamais avant une fenêtre
   * ouverte par-dessus (elle se ferme d'abord). */
  function touche(e: KeyboardEvent) {
    if (!montre || e.key !== 'Escape' || fenetreOuverte()) return;
    e.preventDefault();
    e.stopPropagation();
    masquer();
  }
</script>

<svelte:window onkeydowncapture={touche} />

{#if montre}
  <div class="lancement" role="status" aria-live="polite">
    {#if fond}
      <img class="fond" class:visible={fondCharge} src={fond} alt="" onload={() => (fondCharge = true)} onerror={() => (fondCharge = false)} />
    {/if}
    <div class="voile" aria-hidden="true"></div>
    <div class="carte panel">
      <div class="roue" aria-hidden="true"></div>
      <p class="titre">{titre}</p>
      <p>se lance… Il prendra l’écran dès qu’il sera prêt.</p>
      <button class="btn" bind:this={bouton} onclick={masquer}>Revenir à Frogtend (le jeu continue de se lancer)</button>
    </div>
  </div>
{/if}

<style>
  .lancement {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    background: var(--bg);
  }
  .fond {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: blur(calc(4 * var(--u)));
    opacity: 0;
    transition: opacity calc(var(--duree) * 2);
  }
  .fond.visible {
    opacity: 1;
  }
  .voile {
    position: absolute;
    inset: 0;
    background: var(--voile);
  }
  .carte {
    position: relative;
    display: grid;
    justify-items: center;
    gap: calc(10 * var(--u));
    padding: calc(28 * var(--u)) calc(36 * var(--u));
    max-width: calc(560 * var(--u));
    text-align: center;
  }
  p {
    margin: 0;
  }
  .titre {
    font-size: calc(24 * var(--u));
    font-weight: 700;
  }
  .roue {
    width: calc(40 * var(--u));
    height: calc(40 * var(--u));
    border-radius: 50%;
    border: calc(4 * var(--u)) solid var(--line);
    border-top-color: var(--accent);
    /* Les animations réduites (Réglages ▸ Apparence) mettent --duree à 0 : la roue s'arrête. */
    animation: tourne calc(var(--duree) * 6) linear infinite;
  }
  @keyframes tourne {
    to {
      transform: rotate(1turn);
    }
  }
</style>
