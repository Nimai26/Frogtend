<script lang="ts">
  // Taodbox (lot 5) : le mode canapé. Les jeux de CE PC en grandes jaquettes, par console ; tout se fait à la manette
  // (croix, A, B) ou aux flèches. L'échelle ×2, le focus fort et la marge de la télé viennent de la charte (§ 7).
  import { goto } from '$app/navigation';
  import { onDestroy, onMount, tick } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import { api, type JeuPc } from '$lib/api';
  import { confirmer } from '$lib/dialogues/fenetres.svelte';
  import { etat, sortirDuProfil } from '$lib/etat.svelte';
  import { gererJeu, installer, jouer, partie } from '$lib/ludotheque/jeu.svelte';
  import Jaquette from '$lib/ludotheque/Jaquette.svelte';
  import { tele } from '$lib/ludotheque/telechargements.svelte';
  import { fenetreOuverte, selectionner, taodbox } from '$lib/taodbox/manette.svelte';
  import { adresseImageBoutique } from '$lib/api';
  import { lireSteam, ouvrirDansSteam, steam } from '$lib/boutiques/steam.svelte';
  import { galaxy, lireGalaxy, ouvrirDansGalaxy } from '$lib/boutiques/galaxy.svelte';
  import { adresseImageParUrl, adresseFondDuJeu, adresseLogoDuJeu } from '$lib/api';
  import { apresRepos, idDuFocus } from '$lib/taodbox/fond';

  let lanceParSunshine = $state(false);
  let console_ = $state<string | null>(null);

  const jeux = $derived(
    Object.values(tele.jeux)
      .filter((j: JeuPc) => j.etat === 'telecharge')
      .sort((a, b) => (b.derniere_partie ?? '').localeCompare(a.derniere_partie ?? '') || a.titre.localeCompare(b.titre)),
  );
  const consoles = $derived([...new Set(jeux.map((j) => j.plateforme))].sort());
  const affiches = $derived(console_ ? jeux.filter((j) => j.plateforme === console_) : jeux);

  // Le fond et le logo du jeu choisi (lot Médias, M3, règle dans `$lib/taodbox/fond`) : une seule image de chaque à la
  // fois, recréée à chaque jeu (`{#key}`) : jamais celle du jeu précédent ; un fond n'apparaît qu'une fois chargé, son
  // voile avec lui. Steam, GOG et les boutons : le fond du skin.
  let fondId = $state<number | null>(null);
  let fondCharge = $state(false);
  let logoCharge = $state(false);
  let logoAbsent = $state(false);
  const repos = apresRepos<number | null>((id) => choisirFond(id));
  const avecFonds = $derived(etat.profil.ludotheque.fondsTaodbox);
  // Un jeu sorti de la liste (autre console, jeu retiré) ne garde jamais son fond.
  const choisi = $derived(fondId === null ? null : (affiches.find((j) => j.id === fondId) ?? null));
  const voile = $derived(avecFonds && choisi !== null && fondCharge);

  function surFocus(e: FocusEvent) {
    repos.pousser(idDuFocus(e.target));
  }

  function choisirFond(id: number | null) {
    if (id === fondId) return;
    fondCharge = false;
    logoCharge = false;
    logoAbsent = false;
    fondId = id;
  }

  function choisirConsole(c: string | null) {
    repos.annuler();
    choisirFond(null);
    console_ = c;
  }

  /** Les jeux Steam installés sur ce PC, jouables depuis le canapé (Steam les lance). */
  const steamInstalles = $derived(steam.jeux.filter((j) => j.installe));

  /** Les jeux installés lus dans GOG Galaxy (hors Steam, déjà montré à part). */
  const galaxyInstalles = $derived(galaxy.jeux.filter((j) => j.installe && j.plateforme !== 'steam'));

  onMount(async () => {
    lireSteam();
    lireGalaxy();
    lanceParSunshine = isTauri() ? await api.taodboxLance().catch(() => false) : false;
    await tick();
    document.querySelector<HTMLElement>('.taodbox .carte')?.focus();
  });
  onDestroy(() => repos.annuler());

  async function jeu(j: JeuPc) {
    // Pas pendant « Gérer le jeu » (un réglage peut attendre sans fenêtre ouverte).
    if (partie.enJeu !== null || enGestion) return;
    if (!j.installation) {
      if (!(await installer(j.id))) return;
    }
    await jouer(j.id);
  }

  /** Y (ou la touche Menu ≣, ou le clic droit) : « ⚙ Gérer le jeu » de la carte choisie (✅ Seb 19/10), puis le focus
   * revient à cette carte (la liste a pu être rechargée et retriée entre-temps). */
  // La vraie touche Menu peut donner `keydown` PUIS `contextmenu` : un seul menu à la fois.
  let enGestion = false;
  async function gerer(j: JeuPc, e: Event) {
    e.preventDefault();
    if (enGestion || partie.enJeu !== null || fenetreOuverte()) return;
    enGestion = true;
    try {
      await gererJeu(j.id);
    } finally {
      enGestion = false;
    }
    await tick();
    const carte = document.querySelector<HTMLElement>(`.taodbox .carte[data-id="${j.id}"]`);
    if (carte) selectionner(carte);
  }

  async function bureau() {
    await goto('/');
  }

  async function quitter() {
    if (!(await confirmer('⏻ Quitter Taodbox ?', { libelleValider: 'Quitter' }))) return;
    const { exit } = await import('@tauri-apps/plugin-process');
    await exit(0);
  }
</script>

<!-- Jamais le menu natif de la page (Précédent, Actualiser…) : la touche Menu ≣ le donne au relâchement, quand le focus
     est déjà dans la fenêtre « Gérer » ; le clic droit sur une carte ouvre « Gérer ». -->
<svelte:window oncontextmenu={(e) => e.preventDefault()} />

<div class="taodbox" class:avec-fond={voile} onfocusin={surFocus}>
  {#if avecFonds && choisi}
    {#key choisi.id}
      <img
        class="fond-jeu"
        class:visible={fondCharge}
        src={adresseFondDuJeu(choisi.id)}
        alt=""
        onload={() => (fondCharge = true)}
        onerror={() => (fondCharge = false)}
      />
    {/key}
    <!-- Le voile suit toujours l'image (même fondu) : jamais une image sans voile sous le texte. -->
    <div class="voile-jeu" class:visible={fondCharge}></div>
  {/if}
  <header>
    <img class="logo" src="/grenouille.png" alt="" />
    <h1>Taodbox</h1>
    <span class="qui">{etat.profilOuvert?.nom}</span>
    <span class="espace"></span>
    <button class="btn" onclick={() => sortirDuProfil()}>👥 Changer de joueur</button>
    {#if lanceParSunshine}
      <button class="btn" onclick={quitter}>⏻ Quitter</button>
    {:else}
      <button class="btn" onclick={bureau}>🖥 Bureau</button>
    {/if}
  </header>

  {#if consoles.length > 1}
    <nav class="consoles" aria-label="Consoles">
      <button class="btn" class:choisie={console_ === null} onclick={() => choisirConsole(null)}>Tout</button>
      {#each consoles as c (c)}
        <button class="btn" class:choisie={console_ === c} onclick={() => choisirConsole(c)}>{c}</button>
      {/each}
    </nav>
  {/if}

  {#if avecFonds && affiches.length}
    <!-- Hauteur FIXE : la grille ne bouge jamais quand le logo ou le titre change. -->
    <div class="bandeau">
      {#if choisi}
        {#if !logoAbsent}
          {#key choisi.id}
            <img
              class="logo-jeu"
              class:visible={logoCharge}
              src={adresseLogoDuJeu(choisi.id)}
              alt=""
              onload={() => (logoCharge = true)}
              onerror={() => (logoAbsent = true)}
            />
          {/key}
        {/if}
        {#if !logoCharge}<span class="titre-jeu">{choisi.titre}</span>{/if}
        <span class="console-jeu">{choisi.plateforme}</span>
      {/if}
    </div>
  {/if}

  {#if affiches.length === 0}
    <p class="vide">Aucun jeu sur ce PC pour l’instant. Ajoute-en depuis le bureau (Catalogue Firehouse).</p>
  {:else}
    <div class="grille">
      {#each affiches as j (j.id)}
        <button
          class="carte"
          data-id={j.id}
          onclick={() => jeu(j)}
          oncontextmenu={(e) => gerer(j, e)}
          onkeydown={(e) => e.key === 'ContextMenu' && gerer(j, e)}
          disabled={partie.enJeu !== null && partie.enJeu !== j.id} title={j.titre}>
          <Jaquette id={j.id} titre={j.titre} plateforme={j.plateforme} largeur={180} visuel={etat.profil.ludotheque.visuel} />
          <span class="titre">{j.titre}</span>
          <span class="etat">{partie.enJeu === j.id ? '▶ En cours' : j.installation ? '' : '📦 À installer'}</span>
        </button>
      {/each}
    </div>
  {/if}

  {#if steamInstalles.length && (console_ === null || console_ === 'Steam')}
    <h2 class="section">Steam</h2>
    <div class="grille">
      {#each steamInstalles as j (j.id)}
        <button class="carte" onclick={() => ouvrirDansSteam(j, 'jouer')} title={j.nom}>
          <span class="image-steam"><img src={adresseImageBoutique('steam', j.id)} alt="" loading="lazy" /></span>
          <span class="titre">{j.nom}</span>
          <span class="etat">Steam</span>
        </button>
      {/each}
    </div>
  {/if}

  {#if galaxyInstalles.length && console_ === null}
    <h2 class="section">GOG Galaxy</h2>
    <div class="grille">
      {#each galaxyInstalles as j (j.id)}
        <button class="carte" onclick={() => ouvrirDansGalaxy(j)} title={j.nom}>
          <span class="image-steam">{#if j.image}<img src={adresseImageParUrl(j.image)} alt="" loading="lazy" />{/if}</span>
          <span class="titre">{j.nom}</span>
          <span class="etat">GOG Galaxy</span>
        </button>
      {/each}
    </div>
  {/if}

  <footer class="aide">
    {#if taodbox.manettes}🎮 Croix pour choisir · A pour jouer · Y pour gérer le jeu · B pour revenir{:else}↑ ↓ ← → pour choisir · Entrée pour jouer · Menu (≣) pour gérer le jeu · Échap pour revenir{/if}
    · Pendant le jeu : {etat.pc.menuJeu.touche === 'Pause' ? 'Pause/Attn' : etat.pc.menuJeu.touche} pour le menu
  </footer>
</div>

<style>
  /* Marge de sécurité de la télé : environ 5 % (charte § 7). */
  .taodbox {
    /* Le fond du jeu (z-index -1) passe sous la grille, au-dessus de la vidéo du skin. */
    position: relative;
    isolation: isolate;
    box-sizing: border-box;
    height: 100vh;
    padding: 5vh 5vw;
    display: flex;
    flex-direction: column;
    gap: calc(16 * var(--u));
    overflow: hidden;
  }
  .fond-jeu {
    position: fixed;
    inset: 0;
    z-index: -1;
    width: 100%;
    height: 100%;
    object-fit: cover;
    background: var(--fond-media);
    /* 1 000 px au plus, étirés sur la télé : un léger flou le fait paraître voulu. */
    filter: blur(calc(4 * var(--u)));
    opacity: 0;
    transition: opacity calc(var(--duree) * 2);
    pointer-events: none;
  }
  .fond-jeu.visible {
    opacity: 1;
  }
  /* Sur une image, la charte demande le voile fort et les encres de voile (§ 7). */
  .voile-jeu {
    position: fixed;
    inset: 0;
    z-index: -1;
    background: var(--voile-fort);
    opacity: 0;
    transition: opacity calc(var(--duree) * 2);
    pointer-events: none;
  }
  .voile-jeu.visible {
    opacity: 1;
  }
  .avec-fond {
    color: var(--encre-voile);
  }
  .avec-fond .qui,
  .avec-fond .etat,
  .avec-fond .aide,
  .avec-fond .console-jeu {
    color: var(--encre-voile-dim);
  }
  .avec-fond .carte {
    color: var(--encre-voile);
  }
  .bandeau {
    flex: none;
    height: min(18vh, calc(110 * var(--u)));
    display: flex;
    align-items: center;
    gap: calc(16 * var(--u));
    min-width: 0;
  }
  .logo-jeu {
    height: 100%;
    max-width: 60%;
    object-fit: contain;
    object-position: left center;
    opacity: 0;
    transition: opacity calc(var(--duree) * 2);
  }
  /* Pas encore chargé : ne prend aucune place (le titre s'affiche). */
  .logo-jeu:not(.visible) {
    width: 0;
    visibility: hidden;
  }
  .logo-jeu.visible {
    opacity: 1;
  }
  .titre-jeu {
    font-size: calc(30 * var(--u));
    font-weight: 700;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .console-jeu {
    color: var(--dim);
    font-size: calc(18 * var(--u));
    white-space: nowrap;
  }
  header {
    display: flex;
    align-items: center;
    gap: calc(14 * var(--u));
  }
  .logo {
    height: calc(36 * var(--u));
  }
  h1 {
    margin: 0;
    font-size: calc(26 * var(--u));
  }
  .qui {
    color: var(--dim);
  }
  .espace {
    flex: 1;
  }
  .consoles {
    display: flex;
    gap: calc(8 * var(--u));
    flex-wrap: wrap;
  }
  .consoles .choisie {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    border-color: var(--accent);
  }
  .grille {
    flex: 1;
    overflow: auto;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(calc(180 * var(--u)), 1fr));
    gap: calc(18 * var(--u));
    align-content: start;
    padding: calc(8 * var(--u));
  }
  .carte {
    display: grid;
    gap: calc(6 * var(--u));
    padding: calc(6 * var(--u));
    background: none;
    border: calc(2 * var(--u)) solid transparent;
    border-radius: calc(10 * var(--u));
    color: var(--ink);
    text-align: left;
    cursor: pointer;
    transition: transform 0.08s;
  }
  /* Focus fort et toujours visible (charte § 7). */
  .carte:focus-visible,
  .carte:focus {
    outline: calc(4 * var(--u)) solid var(--accent);
    outline-offset: calc(4 * var(--u));
    border-color: var(--accent);
    transform: scale(1.05);
  }
  .titre {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .etat {
    color: var(--dim);
    min-height: 1.2em;
  }
  .section {
    margin: 0;
    font-size: calc(18 * var(--u));
  }
  .image-steam {
    aspect-ratio: 2 / 3;
    border-radius: calc(6 * var(--u));
    overflow: hidden;
    background: color-mix(in srgb, var(--ink) 8%, transparent);
  }
  .image-steam img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .vide {
    color: var(--dim);
    font-size: calc(18 * var(--u));
  }
  .aide {
    color: var(--dim);
    text-align: center;
  }
</style>
