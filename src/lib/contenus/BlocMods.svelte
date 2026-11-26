<script lang="ts">
  // 🧩 Les mods Switch d'un jeu dans la bande de droite (bureau) et dans « Réglages du jeu » (canapé) : TOUJOURS un
  // résumé et le bouton « 🧩 Ouvrir les mods (N) », même pour 2 mods (✅ Seb 26/11 : « dans plus de 300 mods, juste
  // dans la petite bande de droite, faire défiler c'est ingérable »). La liste, les onglets et la recherche sont sur la
  // page des mods (PageMods). La logique est partagée (`ModsSwitchJeu`) ; ici, seulement ses deux effets.
  import { goto } from '$app/navigation';
  import { etat } from '$lib/etat.svelte';
  import { auCanape } from '$lib/taodbox/canape';
  import { selectionner } from '$lib/taodbox/manette.svelte';
  import { tick, untrack } from 'svelte';
  import { lignesRemarque } from './contenus';
  import { ModsSwitchJeu } from './modsSwitch.svelte';
  import { adressePageMods, reprendreRetourMods, retenirRetourMods } from './pageMods';
  import {
    blocModsVisible,
    etapeModsAnnoncee,
    libelleOuvrirMods,
    libellePreparer,
    ligneAllumesPasUtilises,
    ligneEtatListe,
    ligneMusiques,
    ligneProgresMods,
    lignesDernierePose,
    lignesFinMods,
    nAllumesPasUtilises,
    preparerVisible,
    resumeMods,
  } from './mods';

  // `etatBloc` (lu par l'écran « Réglages du jeu ») : `attente` tant que la liste n'est pas lue, puis `vide` ou
  // `visible` — pour dire « Rien à régler » plutôt qu'un écran muet.
  let {
    id,
    titre,
    etatBloc = $bindable('attente'),
  }: { id: number; titre: string; etatBloc?: 'attente' | 'vide' | 'visible' } = $props();

  const ms = new ModsSwitchJeu();
  let bouton = $state<HTMLButtonElement | null>(null);
  // Au retour de la page des mods : le focus revient sur « Ouvrir les mods » (consommé une seule fois, à l'ouverture).
  let retour = $state(reprendreRetourMods(untrack(() => id)));

  $effect(() => {
    etatBloc = !ms.charge ? 'attente' : blocModsVisible(ms.liste) ? 'visible' : 'vide';
  });

  // Un autre jeu ou un autre profil : tout est remis à zéro (jamais les mods d'un autre profil).
  $effect(() => {
    const [jeu, profil] = [id, etat.profilOuvert?.id ?? null];
    untrack(() => ms.ouvrir(jeu, titre, profil));
  });

  // Pendant un téléchargement : relu toutes les 2 s ; `actif` ne change qu'au début et à la fin (une seule minuterie).
  $effect(() => {
    if (!ms.actif) return;
    const g = untrack(() => ms.debutSuivi());
    const minuterie = setInterval(() => ms.releverSuivi(g), 2000);
    return () => clearInterval(minuterie);
  });

  // Après « Préparer » (le bouton disparaît) : le focus va à « Ouvrir les mods », juste sous le résumé.
  ms.lieuDuFocus = () => ({ conteneur: null, secours: bouton });

  /** Le focus n'est pris que s'il n'est nulle part, ou sur « ← Retour » (posé d'office par « Réglages du jeu ») :
   * jamais volé à ce que la personne a choisi entre-temps. */
  const focusLibre = () => {
    const a = document.activeElement as HTMLElement | null;
    return !a || a === document.body || a === document.documentElement || a.textContent?.trim() === '← Retour';
  };

  $effect(() => {
    if (!retour || !bouton) return;
    retour = false;
    const b = bouton;
    tick().then(() => {
      if (!focusLibre()) return;
      if (auCanape()) selectionner(b);
      else b.focus();
    });
  });

  const n = $derived(nAllumesPasUtilises(ms.mods, ms.pose));

  function ouvrirPage() {
    retenirRetourMods(id);
    goto(adressePageMods(id, titre, auCanape()));
  }
</script>

{#if blocModsVisible(ms.liste) && ms.liste}
  <div class="mods cx-block">
    <p class="titre">🧩 Mods : <span aria-live="polite">{resumeMods(ms.mods)}</span></p>
    {#if ligneEtatListe(ms.liste)}<p class="muted">{ligneEtatListe(ms.liste)}</p>{/if}
    {#each lignesRemarque(ms.liste.remarque) as l (l)}<p class="muted">{l}</p>{/each}
    <!-- Ce que la dernière partie a dit (un mod allumé pas mis en place, un abri impossible…) : toujours dit. -->
    {#each lignesDernierePose(ms.pose) as l, i (i)}<p class={i ? 'muted' : ''}>{l}</p>{/each}
    {#if ligneAllumesPasUtilises(n)}<p>{ligneAllumesPasUtilises(n)}</p>{/if}
    <!-- Contrat 1.55 : les musiques des mods (préparées par Firehouse), ou pourquoi elles ne sont pas posées ; le bouton
         « Préparer » quand elles attendent un oui. -->
    {#if ligneMusiques(ms.liste)}<p>{ligneMusiques(ms.liste)}</p>{/if}
    <p class="muted" aria-live="polite">{ms.regardeTaille ? '⏳ Je regarde la taille des musiques…' : ''}</p>
    {#if preparerVisible(ms.liste)}
      <div class="actions">
        <button class="btn petit" onclick={() => ms.preparer()} disabled={ms.bloque || ms.preparation || ms.occupe || ms.regardeTaille}>
          {libellePreparer(ms.liste.musiques?.taille_connue)}
        </button>
      </div>
    {/if}
    <!-- Le lecteur d'écran n'annonce que l'ÉTAPE (le mod en cours) puis la fin, jamais le pourcentage qui bouge. -->
    <span class="annonce" aria-live="polite">{etapeModsAnnoncee(ms.suivi) || ms.annonceFin}</span>
    {#if ligneProgresMods(ms.suivi)}<p>{ligneProgresMods(ms.suivi)}</p>{/if}
    {#each lignesFinMods(ms.suivi) as l, i (i)}<p class="muted">{l}</p>{/each}
    {#if ms.mods.length}
      <div class="actions">
        <button class="btn" bind:this={bouton} onclick={ouvrirPage}>{libelleOuvrirMods(ms.mods.length)}</button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .mods {
    display: grid;
    gap: calc(6 * var(--u));
  }
  .titre {
    font-weight: 600;
  }
  .actions {
    display: flex;
    gap: calc(6 * var(--u));
    flex-wrap: wrap;
  }
  /* Les petits boutons du bloc : la cible garde 40 px au moins (charte § 4). */
  .petit {
    min-height: var(--cible-min);
    padding: calc(5 * var(--u)) calc(11 * var(--u));
    font-size: calc(12 * var(--u));
  }
  .btn {
    min-height: var(--cible-min);
  }
  /* Lu par un lecteur d'écran, jamais affiché. */
  .annonce {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  p {
    margin: 0;
  }
</style>
