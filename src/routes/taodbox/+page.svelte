<script lang="ts">
  // Taodbox (lot 5) : le mode canapé. Les jeux de CE PC en grandes jaquettes, par console ; tout se fait à la manette
  // (croix, A, B) ou aux flèches. L'échelle ×2, le focus fort et la marge de la télé viennent de la charte (§ 7).
  import { goto } from '$app/navigation';
  import { onDestroy, onMount, tick, untrack } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import { adresseImageSysteme, api, type FicheSysteme, type JeuPc } from '$lib/api';
  import { confirmer, informer } from '$lib/dialogues/fenetres.svelte';
  import { creditImage, imageEntete } from '$lib/ludotheque/systemes';
  import { etat, sortirDuProfil } from '$lib/etat.svelte';
  import { gererLaCarte, installer, jouerLaCarte, partie } from '$lib/ludotheque/jeu.svelte';
  import { groupeDe, originesJeuPc, unParGroupe } from '$lib/boutiques/copies';
  import IconesBoutiques from '$lib/boutiques/IconesBoutiques.svelte';
  import Jaquette from '$lib/ludotheque/Jaquette.svelte';
  import { jeuSurPc, rechargerJeuxDuPc, tele } from '$lib/ludotheque/telechargements.svelte';
  import { chargerNomsCourts, nomAffiche } from '$lib/ludotheque/ludotheque.svelte';
  import { fenetreOuverte, selectionner, taodbox } from '$lib/taodbox/manette.svelte';
  import { adresseImageBoutique } from '$lib/api';
  import { lireSteam, ouvrirDansSteam, steam } from '$lib/boutiques/steam.svelte';
  import { galaxy, lireGalaxy, ouvrirDansGalaxy } from '$lib/boutiques/galaxy.svelte';
  import { adresseImageParUrl, adresseFondDuJeu, adresseLogoDuJeu } from '$lib/api';
  import { apresRepos, idDuFocus } from '$lib/taodbox/fond';

  let lanceParSunshine = $state(false);
  let console_ = $state<string | null>(null);

  // Les jeux possédés sur plusieurs boutiques (A6c) : une seule carte, Frogtend demande laquelle lancer.
  let groupes = $state<number[][]>([]);
  $effect(() => {
    void Object.keys(tele.jeux).length;
    api.ludothequeGroupes().then((g) => (groupes = g)).catch(() => {});
  });
  const jeux = $derived(
    unParGroupe(
      Object.values(tele.jeux)
        .filter((j: JeuPc) => j.etat === 'telecharge')
        // Un jeu, une carte (contrat 1.21) : un import relié ne compte qu'une fois par fiche (le même que la ludothèque).
        .filter((j: JeuPc) => !(j.id < 0 && j.fiche) || jeuSurPc(j.fiche)?.id === j.id)
        .sort((a, b) => (b.derniere_partie ?? '').localeCompare(a.derniere_partie ?? '') || a.titre.localeCompare(b.titre)),
      groupes,
    ),
  );
  // Triées sur le nom MONTRÉ ; la clé reste le nom LaunchBox (le filtre compare `j.plateforme`).
  const consoles = $derived([...new Set(jeux.map((j) => j.plateforme))].sort((a, b) => nomAffiche(a).localeCompare(nomAffiche(b), 'fr')));
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

  // Les fiches des consoles (contrat 1.28), lues une fois chacune dans le cache du profil (hors ligne aussi) : le logo
  // (sinon la bannière) sur leur bouton, la photo de la console en fond quand elle est choisie.
  let fiches = $state<Record<string, FicheSysteme | null>>({});
  /** Les images de bouton qui n'ont pas pu s'afficher (« console:sorte ») : la suivante, puis le nom en texte. */
  let logosEnEchec = $state<string[]>([]);
  $effect(() => {
    const liste = consoles;
    // Lire `fiches` sans s'y abonner : la remplir ne relance pas cet effet.
    untrack(() => {
      for (const c of liste) {
        if (c in fiches) continue;
        fiches[c] = null;
        api
          .plateformeFiche(c)
          .then((f) => (fiches[c] = f))
          .catch(() => {});
      }
    });
  });
  const logoDe = (c: string) =>
    imageEntete(
      fiches[c],
      logosEnEchec.filter((e) => e.startsWith(`${c}:`)).map((e) => e.slice(c.length + 1)),
      ['logo', 'banniere'],
    );
  // La photo de la console choisie : sous le fond d'un jeu (celui du jeu passe devant quand il est chargé).
  const ficheChoisie = $derived(console_ ? (fiches[console_] ?? null) : null);
  const fondSysteme = $derived(ficheChoisie?.images?.fond?.empreinte ? ficheChoisie.images.fond : null);
  let fondSystemeCharge = $state(false);
  /** Le fond du jeu choisi n'a pas pu s'afficher : la photo de la console reste. */
  let fondEchec = $state(false);
  // La photo de la console : seulement sans jeu choisi, ou si son fond a échoué (jamais un aller-retour entre deux jeux).
  const photoVisible = $derived(fondSystemeCharge && (choisi === null || fondEchec));

  /** Les crédits de ce que Taodbox montre de la console (logo, photo) : pas de navigateur au canapé, l'adresse en texte. */
  function credits(c: string) {
    const f = fiches[c];
    const lignes = [logoDe(c)?.image, avecFonds && fondSystemeCharge ? f?.images?.fond : null]
      .filter((i) => i?.licence)
      .map((i) => `${creditImage(i)}${i!.url_source ? `\n${i!.url_source}` : ''}`);
    informer(`ⓘ Crédits — ${nomAffiche(c)}`, lignes.join('\n\n'));
  }
  const aDesCredits = $derived(
    console_ !== null && [logoDe(console_)?.image, avecFonds && fondSystemeCharge ? fondSysteme : null].some((i) => i?.licence),
  );
  const voile = $derived(avecFonds && ((choisi !== null && fondCharge) || (fondSysteme !== null && photoVisible)));

  function surFocus(e: FocusEvent) {
    repos.pousser(idDuFocus(e.target));
  }

  function choisirFond(id: number | null) {
    if (id === fondId) return;
    fondCharge = false;
    fondEchec = false;
    logoCharge = false;
    logoAbsent = false;
    fondId = id;
  }

  function choisirConsole(c: string | null) {
    repos.annuler();
    choisirFond(null);
    fondSystemeCharge = false;
    console_ = c;
  }

  /** Les jeux Steam installés sur ce PC, jouables depuis le canapé (Steam les lance). */
  // Ceux dont le dossier est connu sont des jeux du PC : dans la grille principale, jamais en double ici.
  const steamInstalles = $derived(steam.jeux.filter((j) => j.installe && !j.dossier));

  /** Les jeux installés lus dans GOG Galaxy (hors Steam, déjà montré à part). */
  // Un jeu GOG avec son dossier est un jeu du PC (A6a) : déjà dans la grille, jamais deux fois.
  const galaxyInstalles = $derived(galaxy.jeux.filter((j) => j.installe && j.plateforme !== 'steam' && !(j.plateforme === 'gog' && j.dossier)));

  onMount(async () => {
    // Les jeux Steam installés deviennent des jeux du PC à la lecture : la grille est relue ensuite.
    lireSteam().then(rechargerJeuxDuPc);
    lireGalaxy();
    // Les noms courts des consoles (« Mega Drive »), lus dans le cache du profil (hors ligne aussi).
    chargerNomsCourts();
    lanceParSunshine = isTauri() ? await api.taodboxLance().catch(() => false) : false;
    await tick();
    selectionner(document.querySelector<HTMLElement>('.taodbox .carte'));
  });
  onDestroy(() => repos.annuler());

  async function jeu(j: JeuPc) {
    // Pas pendant « Gérer le jeu » (un réglage peut attendre sans fenêtre ouverte).
    if (partie.enJeu !== null || partie.lancement !== null || enGestion) return;
    if (!j.installation) {
      // Le jeu d'une autre personne : elle seule peut l'installer.
      if (j.d_un_autre || !(await installer(j.id))) return;
    }
    await jouerLaCarte(j.id);
  }

  /** Y (ou la touche Menu ≣, ou le clic droit) : « ⚙ Gérer le jeu » de la carte choisie (✅ Seb 19/10), puis le focus
   * revient à cette carte (la liste a pu être rechargée et retriée entre-temps). */
  // La vraie touche Menu peut donner `keydown` PUIS `contextmenu` : un seul menu à la fois.
  let enGestion = false;
  async function gerer(j: JeuPc, e: Event) {
    e.preventDefault();
    if (enGestion || partie.enJeu !== null || fenetreOuverte() || j.d_un_autre) return;
    enGestion = true;
    try {
      await gererLaCarte(j.id);
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
  {#if avecFonds && console_ && fondSysteme}
    {#key `${console_}-${fondSysteme.empreinte}`}
      <img
        class="fond-jeu"
        class:visible={photoVisible}
        src={adresseImageSysteme(console_, 'fond', 1000, fondSysteme.empreinte)}
        alt=""
        onload={() => (fondSystemeCharge = true)}
        onerror={() => (fondSystemeCharge = false)}
      />
    {/key}
  {/if}
  {#if avecFonds && choisi}
    {#key choisi.id}
      <img
        class="fond-jeu"
        class:visible={fondCharge}
        src={adresseFondDuJeu(choisi.fiche ?? choisi.id)}
        alt=""
        onload={() => (fondCharge = true)}
        onerror={() => {
          fondCharge = false;
          fondEchec = true;
        }}
      />
    {/key}
  {/if}
  <!-- Le voile suit toujours l'image (même fondu) : jamais une image sans voile sous le texte. -->
  {#if avecFonds}<div class="voile-jeu" class:visible={voile}></div>{/if}
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
        {@const logo = logoDe(c)}
        <button class="btn" class:choisie={console_ === c} class:avec-logo={logo} aria-label={nomAffiche(c)} title={nomAffiche(c)} onclick={() => choisirConsole(c)}>
          {#if logo}
            <img
              class="logo-console"
              src={adresseImageSysteme(c, logo.sorte, 300, logo.image.empreinte)}
              alt=""
              onerror={() => (logosEnEchec = [...logosEnEchec, `${c}:${logo.sorte}`])}
            />
          {:else}
            {nomAffiche(c)}
          {/if}
        </button>
      {/each}
      {#if aDesCredits && console_}<button class="btn" onclick={() => credits(console_!)}>ⓘ Crédits</button>{/if}
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
              src={adresseLogoDuJeu(choisi.fiche ?? choisi.id)}
              alt=""
              onload={() => (logoCharge = true)}
              onerror={() => (logoAbsent = true)}
            />
          {/key}
        {/if}
        {#if !logoCharge}<span class="titre-jeu">{choisi.titre_fiche ?? choisi.titre}</span>{/if}
        <span class="console-jeu">{nomAffiche(choisi.plateforme)}</span>
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
          disabled={partie.enJeu !== null && !groupeDe(j.id, groupes).includes(partie.enJeu)} title={j.titre}>
          <Jaquette id={j.fiche ?? j.id} titre={j.titre} plateforme={j.plateforme} largeur={180} visuel={etat.profil.ludotheque.visuel} />
          <span class="titre">{j.titre_fiche ?? j.titre}</span>
          <IconesBoutiques grand origines={originesJeuPc(j, groupeDe(j.id, groupes), (x) => tele.jeux[x]?.installation?.boutique?.source)} />
          <span class="etat">{partie.enJeu !== null && groupeDe(j.id, groupes).includes(partie.enJeu) ? '▶ En cours' : j.installation ? '' : '📦 À installer'}</span>
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
  .logo-console {
    display: block;
    /* Une place réservée avant le chargement : la rangée ne s'élargit pas d'un coup. */
    min-width: calc(60 * var(--u));
    height: calc(28 * var(--u));
    max-width: calc(140 * var(--u));
    object-fit: contain;
  }
  .consoles .avec-logo {
    padding-block: calc(4 * var(--u));
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
