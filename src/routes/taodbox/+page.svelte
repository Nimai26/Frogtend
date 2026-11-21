<script lang="ts">
  // Taodbox (lot 5) : le mode canapé. Les jeux de CE PC en grandes jaquettes, par console ; tout se fait à la manette
  // (croix, A, B) ou aux flèches. Les tailles d'après l'écran, le focus fort et la marge de la télé viennent de la charte (§ 7).
  import { goto } from '$app/navigation';
  import { onDestroy, onMount, tick, untrack } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import { adresseImageSysteme, api, type FicheSysteme, type JeuBoutique, type JeuPc } from '$lib/api';
  import { confirmer, informer, toast } from '$lib/dialogues/fenetres.svelte';
  import { creditImage, imageEntete } from '$lib/ludotheque/systemes';
  import { etat, reglerProfil, sortirDuProfil } from '$lib/etat.svelte';
  import { poserModeCanape } from '$lib/taodbox/canape';
  import { LIBELLES_TAILLE_TAODBOX, tailleTaodbox, tailleTaodboxSuivante } from '$lib/reglages/reglages';
  import { gererLaCarte, installer, jouerLaCarte, lancementImpossible, partie } from '$lib/ludotheque/jeu.svelte';
  import { cartesAInstallerTaodbox, cartesBoutiqueTaodbox, groupeDe, originesJeuPc, unParGroupe, type CarteAInstallerTaodbox, type CarteBoutiqueTaodbox } from '$lib/boutiques/copies';
  import { boutiqueSansInstallation, estBattlenet, imageCarteAInstaller, messageInstallationCanape, MESSAGE_BATTLENET } from '$lib/boutiques/jeu-boutique';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import type { JeuResume } from '$lib/api';
  import IconesBoutiques from '$lib/boutiques/IconesBoutiques.svelte';
  import Jaquette from '$lib/ludotheque/Jaquette.svelte';
  import { jeuSurPc, rechargerJeuxDuPc, tele } from '$lib/ludotheque/telechargements.svelte';
  import { chargerNomsCourts, nomAffiche } from '$lib/ludotheque/ludotheque.svelte';
  import { fenetreOuverte, selectionner, taodbox } from '$lib/taodbox/manette.svelte';
  import { adresseImageBoutique } from '$lib/api';
  import { lireSteam, ouvrirDansSteam, steam } from '$lib/boutiques/steam.svelte';
  import { galaxy, lireGalaxy, ouvrirDansGalaxy } from '$lib/boutiques/galaxy.svelte';
  import { adresseImageParUrl, adresseFondDuJeu, adresseLogoDuJeu } from '$lib/api';
  import { apresRepos, assezGrandPourUnFond, idDuFocus } from '$lib/taodbox/fond';
  import { consoleValide, oublierPlace, reprendrePlace, retenirPlace, selecteurCarte } from '$lib/taodbox/retour';

  let lanceParSunshine = $state(false);
  // Au retour d'un écran ouvert par « Gérer le jeu » (Réglages, Succès, Boutons) : la même console, la même carte.
  const place = reprendrePlace();
  let console_ = $state<string | null>(place?.console ?? null);

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
        // Un jeu de boutique sans installation (ligne gardée pour son temps de jeu) : jamais une carte jouable ; sa carte
        // « à installer » vient de la ludothèque (`aInstaller`, lot B).
        .filter((j: JeuPc) => !boutiqueSansInstallation(j))
        // Un jeu, une carte (contrat 1.21) : un import relié ne compte qu'une fois par fiche (le même que la ludothèque).
        .filter((j: JeuPc) => !(j.id < 0 && j.fiche) || jeuSurPc(j.fiche)?.id === j.id)
        .sort((a, b) => (b.derniere_partie ?? '').localeCompare(a.derniere_partie ?? '') || a.titre.localeCompare(b.titre)),
      groupes,
    ),
  );
  // Triées sur le nom MONTRÉ ; la clé reste le nom LaunchBox (le filtre compare `j.plateforme`).
  const consoles = $derived([...new Set(jeux.map((j) => j.plateforme))].sort((a, b) => nomAffiche(a).localeCompare(nomAffiche(b), 'fr')));
  const affiches = $derived(console_ ? jeux.filter((j) => j.plateforme === console_) : jeux);
  /** Les jeux installés que Steam ou GOG Galaxy montrent sans être des jeux du PC (lot A : dans la grille principale,
   * avec leur pastille ; sous « Tout » et « Windows »). */
  const cartesBoutique = $derived(
    console_ === null || console_ === 'Windows' ? cartesBoutiqueTaodbox(steam.jeux, galaxy.jeux, Object.values(tele.jeux)) : [],
  );
  /** Lot B (✅ Seb 18/11) : les jeux des boutiques PAS installés (possédés, ou désinstallés), avec leur pastille
   * « à installer » ; A ouvre l'installation par le lanceur de la boutique, jamais une partie. Sous « Tout » et
   * « Windows ». */
  let cartesLudotheque = $state<JeuResume[]>([]);
  const aInstaller = $derived(
    console_ === null || console_ === 'Windows'
      ? // Un jeu Steam que Steam dit installé (carte jouable ci-dessus) n'a jamais aussi sa carte « à installer » (lot C2).
        cartesAInstallerTaodbox(cartesLudotheque, Object.values(tele.jeux)).filter((c) => !cartesBoutique.some((b) => b.cle === c.cle))
      : [],
  );
  // Lot C2 : après chaque import Steam terminé (à l'ouverture de Frogtend, par exemple), les cartes « à installer »
  // sont relues (le premier passage est celui de onMount).
  let importsVus = steam.imports;
  $effect(() => {
    const n = steam.imports;
    if (n === importsVus) return;
    importsVus = n;
    untrack(() => void lireAInstaller());
  });
  /** Les cartes pas installées de la ludothèque (1 000 par page). */
  async function lireAInstaller() {
    const l: JeuResume[] = [];
    for (let decalage = 0; decalage < 20_000; decalage += 1000) {
      const page = await api.lister({ ludotheque: true, installe: false, limite: 1000, decalage }).catch(() => null);
      if (!page) return;
      l.push(...page.jeux);
      if (page.jeux.length < 1000) break;
    }
    cartesLudotheque = l;
  }


  // Le fond et le logo du jeu choisi (lot Médias, M3, règle dans `$lib/taodbox/fond`) : une seule image de chaque à la
  // fois, recréée à chaque jeu (`{#key}`) : jamais celle du jeu précédent ; un fond n'apparaît qu'une fois chargé, son
  // voile avec lui. Steam, GOG et les boutons : le fond du skin.
  let fondId = $state<number | null>(null);
  let fondCharge = $state(false);
  let logoCharge = $state(false);
  let logoAbsent = $state(false);
  const repos = apresRepos<number | null>((id) => choisirFond(id));
  const avecFonds = $derived(etat.profil.ludotheque.fondsTaodbox);

  /** La taille de Taodbox (Seb 19/11 : « tout est beaucoup trop gros ») : A sur le bouton passe à la suivante, tout de
   * suite à l'écran (réglage du profil, aussi au bureau dans ⚙ Options ▸ Apparence). */
  const taille = $derived(tailleTaodbox(etat.profil.apparence.tailleTaodbox));
  const FACTEURS_TAILLE = { petite: 0.8, moyenne: 1, grande: 1.25 } as const;
  // La hauteur de l'écran (celle de `--hauteur-ecran`, canape.ts), relue si la fenêtre change d'écran.
  const hauteurDeLEcran = () => (typeof screen !== 'undefined' && screen.height > 0 ? screen.height : 1080);
  let hauteurEcran = $state(hauteurDeLEcran());
  /** La largeur d'une carte en pixels CSS, sans la taille du texte (`Jaquette` l'ajoute) : la même que `--carte`
   * (l'image : 95 u au plus, 1 u = 0,139 % de la hauteur de l'écran) ; l'image demandée au cœur suit donc la vraie place. */
  const largeurCarte = $derived(Math.round(95 * 0.00139 * hauteurEcran * FACTEURS_TAILLE[taille]));
  let changementTaille = $state(false);
  async function changerTaille() {
    if (changementTaille) return;
    changementTaille = true;
    try {
      await reglerProfil('apparence.tailleTaodbox', tailleTaodboxSuivante(taille));
    } finally {
      changementTaille = false;
    }
  }
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

  /** Jouer à un jeu que seule sa boutique connaît (Steam, GOG Galaxy) : son lanceur s'ouvre, un seul toast dit comment
   * revenir à Frogtend. */
  async function jeuDeBoutique(c: CarteBoutiqueTaodbox) {
    if (lancementImpossible()) return;
    toast(`Le lanceur ${c.par === 'galaxy' ? 'GOG Galaxy' : 'Steam'} s’ouvre ; s’il demande la souris, appuie sur ⊞/Guide pour revenir à Frogtend.`, 'neutre');
    if (c.par === 'steam') await ouvrirDansSteam({ id: c.cle.slice('steam_'.length), nom: c.nom } as JeuBoutique, 'jouer');
    else await ouvrirDansGalaxy({ id: c.cle, nom: c.nom } as JeuBoutique);
  }

  /** A sur une carte « à installer » : la fenêtre d'installation de la boutique s'ouvre (le même moyen qu'au bureau).
   * Le jeu devient jouable à la relecture suivante des boutiques (ouverture du profil). */
  async function installerParLaBoutique(c: CarteAInstallerTaodbox) {
    if (lancementImpossible()) return;
    toast(messageInstallationCanape(c.boutique), 'neutre');
    try {
      await api.boutiqueOuvrir(c.boutique, c.cle, 'installer');
    } catch (e) {
      toast(motifDuRefus(e), 'erreur');
    }
  }

  onMount(async () => {
    // Les jeux Steam installés deviennent des jeux du PC à la lecture : la grille est relue ensuite.
    lireSteam().then(rechargerJeuxDuPc);
    lireAInstaller();
    lireGalaxy();
    // Les noms courts des consoles (« Mega Drive »), lus dans le cache du profil (hors ligne aussi).
    chargerNomsCourts();
    lanceParSunshine = isTauri() ? await api.taodboxLance().catch(() => false) : false;
    // Une console qui n'a plus de jeu sur ce PC : « Tout ».
    if (console_ !== null && consoleValide(place, consoles) === null) console_ = null;
    await tick();
    selectionner(document.querySelector<HTMLElement>(selecteurCarte(place)) ?? document.querySelector<HTMLElement>('.taodbox .carte'));
  });
  onDestroy(() => {
    detruit = true;
    repos.annuler();
  });

  /** Le rappel des triches est affiché (3 s avant le lancement) ; la page a été quittée. */
  let rappelEnCours = false;
  let detruit = false;

  async function jeu(j: JeuPc) {
    // Pas pendant « Gérer le jeu » (un réglage peut attendre sans fenêtre ouverte).
    if (lancementImpossible() || enGestion || rappelEnCours) return;
    // Battle.net : aucun lancement prouvé (lot A), on le dit au lieu d'échouer.
    if (estBattlenet(j)) {
      toast(MESSAGE_BATTLENET, 'alerte');
      return;
    }
    if (!j.installation) {
      // Le jeu d'une autre personne : elle seule peut l'installer.
      if (j.d_un_autre || !(await installer(j.id))) return;
    }
    // Un jeu Jaguar à triche cochée : le rappel AVANT le lancement, le temps de le lire (ensuite BigPEmu couvre
    // l'écran ; ✅ Seb 11/11).
    rappelEnCours = true;
    try {
      if (await api.bigpemuTrichesCochees(j.id).catch(() => false)) {
        toast("🎯 Triches cochées : au démarrage, une petite fenêtre en anglais s’affiche. Attends quelques secondes, puis appuie sur A, puis sur B : le jeu démarre avec tes triches.", 'neutre');
        await new Promise((r) => setTimeout(r, 3000));
        if (detruit || enGestion || fenetreOuverte() || lancementImpossible()) return;
      }
    } finally {
      rappelEnCours = false;
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
    // « Gérer » peut mener à un autre écran : la place est retenue pour le retour (oubliée si on reste ici).
    retenirPlace({ id: j.id, console: console_ });
    try {
      await gererLaCarte(j.id);
    } finally {
      enGestion = false;
      if (!detruit) oublierPlace();
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
<svelte:window oncontextmenu={(e) => e.preventDefault()} onresize={() => { poserModeCanape(true); hauteurEcran = hauteurDeLEcran(); }} />

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
        onload={(e) => {
          // Trop petite (une capture : énorme zoom flou) : comme une absence, le fond du thème.
          if (assezGrandPourUnFond((e.currentTarget as HTMLImageElement).naturalWidth, (e.currentTarget as HTMLImageElement).naturalHeight)) fondCharge = true;
          else fondEchec = true;
        }}
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
    <button class="btn" onclick={changerTaille} aria-disabled={changementTaille} title="Changer la taille de Taodbox">🔍 Taille : {LIBELLES_TAILLE_TAODBOX[taille]}</button>
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

  {#if affiches.length === 0 && cartesBoutique.length === 0 && aInstaller.length === 0}
    <p class="vide">Aucun jeu sur ce PC pour l’instant. Ajoute-en depuis le bureau (Catalogue Firehouse).</p>
  {:else}
    <div class="zone-grille"><div class="grille">
      {#each affiches as j (j.id)}
        {@const etatCarte = partie.enJeu !== null && groupeDe(j.id, groupes).includes(partie.enJeu) ? '▶ En cours' : estBattlenet(j) ? 'ouvre-le depuis Battle.net' : j.installation ? '' : '📦 À installer'}
        <button
          class="carte"
          data-id={j.id}
          onclick={() => jeu(j)}
          oncontextmenu={(e) => gerer(j, e)}
          onkeydown={(e) => e.key === 'ContextMenu' && gerer(j, e)}
          disabled={partie.enJeu !== null && !groupeDe(j.id, groupes).includes(partie.enJeu)} title={j.titre}>
          <span class="visuel">
            <Jaquette id={j.fiche ?? j.id} titre={j.titre} plateforme={j.plateforme} largeur={largeurCarte} visuel={etat.profil.ludotheque.visuel} />
            <span class="coin"><IconesBoutiques grand origines={originesJeuPc(j, groupeDe(j.id, groupes), (x) => tele.jeux[x]?.installation?.boutique?.source)} /></span>
          </span>
          <span class="titre">{j.titre_fiche ?? j.titre}</span>
          <!-- Une ligne d'état seulement quand elle dit quelque chose : la place va à la jaquette (Seb 19/11). -->
          {#if etatCarte}<span class="etat">{etatCarte}</span>{/if}
        </button>
      {/each}
      <!-- Lot A : les jeux installés que seule leur boutique connaît, dans la MÊME grille, avec leur pastille. -->
      {#each cartesBoutique as c (c.cle)}
        <button class="carte" data-boutique={c.cle} onclick={() => jeuDeBoutique(c)} disabled={partie.enJeu !== null} title={c.nom}>
          <span class="image-steam">
            {#if c.par === 'steam'}<img src={adresseImageBoutique('steam', c.cle.slice('steam_'.length))} alt="" loading="lazy" />
            {:else if c.image}<img src={adresseImageParUrl(c.image)} alt="" loading="lazy" />{/if}
            <span class="coin"><IconesBoutiques grand origines={[c.boutique]} /></span>
          </span>
          <span class="titre">{c.nom}</span>
        </button>
      {/each}
      <!-- Lot B : les jeux à installer, APRÈS les jeux jouables ; A ouvre l'installation, jamais une partie. -->
      {#each aInstaller as c (c.cle)}
        <button class="carte a-installer" data-id={c.id} onclick={() => installerParLaBoutique(c)} disabled={partie.enJeu !== null} title={c.nom}>
          <span class="image-steam">
            <!-- La jaquette RÉDUITE par le cœur (400 px, gardée réduite), Steam compris (d'après son appid) : jamais l'original de la boutique. -->
            {#if imageCarteAInstaller(c)}<img src={imageCarteAInstaller(c)} alt="" loading="lazy" onerror={(e) => ((e.currentTarget as HTMLElement).hidden = true)} />{/if}
            <span class="coin"><IconesBoutiques grand origines={[c.boutique]} /></span>
            <span class="pastille-installer">⬇ À installer</span>
          </span>
          <span class="titre">{c.nom}</span>
        </button>
      {/each}
    </div></div>
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
    /* 80 u (120 px en 1080p) : la place va d'abord aux jaquettes (Seb 19/11). */
    height: min(18vh, calc(80 * var(--u)));
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
  /* La place restante sous le bandeau : un conteneur, pour que la largeur des cartes suive SA hauteur (cqh). */
  .zone-grille {
    flex: 1;
    min-height: 0;
    container-type: size;
  }
  .grille {
    height: 100%;
    box-sizing: border-box;
    overflow: auto;
    display: grid;
    /* Des cartes de largeur FIXE, réglées par le bouton « Taille » (le facteur de --u) : l'IMAGE fait 95 u (140 px en
       1080p, Moyenne) ; une rangée entière tient toujours (titre et marges : environ 110 u). La carte = l'image + sa
       marge intérieure et sa bordure (2 × 8 u). Essais de Seb 19/11 : 360 px « trop gros », puis 82 px trop petit
       (la règle « deux rangées entières » écrasait le bouton Taille). */
    --image: max(calc(60 * var(--u)), min(calc(95 * var(--u)), calc((100cqh - 110 * var(--u)) * 2 / 3)));
    --carte: calc(var(--image) + 16 * var(--u));
    grid-template-columns: repeat(auto-fill, var(--carte));
    justify-content: center;
    gap: calc(18 * var(--u));
    align-content: start;
    /* Le cadre de sélection dépasse la carte (contour 4 u à 4 u d'écart, et l'agrandissement de 5 %) : la grille
       garde assez de marge pour ne jamais le couper, et le défilement à la manette la respecte (essai de Seb 19/11). */
    padding: calc(16 * var(--u));
    scroll-padding: calc(16 * var(--u));
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
    font-size: calc(13 * var(--u));
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .etat {
    color: var(--dim);
    min-height: 1.2em;
  }
  /* La jaquette et, dans son coin, d'où vient le jeu (Steam, Epic…) : plus de ligne à part sous le titre. */
  .visuel {
    position: relative;
    display: block;
  }
  .coin {
    position: absolute;
    top: calc(4 * var(--u));
    left: calc(4 * var(--u));
    max-width: calc(100% - 8 * var(--u));
    pointer-events: none;
  }
  .image-steam {
    position: relative;
    aspect-ratio: 2 / 3;
    border-radius: calc(6 * var(--u));
    overflow: hidden;
    background: color-mix(in srgb, var(--ink) 8%, transparent);
    /* La pastille « à installer » se règle sur la largeur de la jaquette (Seb 21/11 : « À in… » coupé en Grande). */
    container-type: inline-size;
  }
  .image-steam img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  /* Lot B : la pastille « à installer », bien visible sur la jaquette (couleurs du skin seulement). */
  .a-installer img {
    opacity: 0.75;
  }
  .pastille-installer {
    position: absolute;
    inset-inline: min(calc(4 * var(--u)), 3cqi);
    bottom: min(calc(4 * var(--u)), 3cqi);
    padding: min(calc(3 * var(--u)), 2cqi) min(calc(4 * var(--u)), 2cqi);
    border-radius: calc(6 * var(--u));
    background: var(--accent);
    color: var(--on-accent);
    /* Jamais plus grand que la jaquette ne le permet : « ⬇ À installer » tient en entier à toutes les tailles. */
    font-size: min(calc(11 * var(--u)), 9cqi);
    font-weight: 700;
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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
