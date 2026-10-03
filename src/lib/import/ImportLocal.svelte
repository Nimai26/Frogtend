<script lang="ts">
  // La fenêtre d'import des jeux déjà sur le disque : ROM d'un dossier, jeux MS-DOS (un par sous-dossier), un jeu
  // Windows, ou un ajout manuel. On CHERCHE d'abord, la personne voit la liste et coche, puis on ajoute. Rien n'est
  // copié, déplacé ni renommé.
  import { untrack } from 'svelte';
  import { api, taille } from '$lib/api';
  import { choisir, confirmer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { parcourir } from '$lib/emulateurs/assistant.svelte';
  import { PLATEFORMES_CONNUES } from '$lib/ludotheque/categories';
  import { ludo, rechargerListe, rechargerPlateformes } from '$lib/ludotheque/ludotheque.svelte';
  import { rechargerJeuxDuPc } from '$lib/ludotheque/telechargements.svelte';
  import { etat, reglerPc } from '$lib/etat.svelte';
  import { informer } from '$lib/dialogues/fenetres.svelte';
  import { pourLeJeu } from '$lib/emulateurs/choix';
  import {
    elementDe,
    emplacementDuSysteme,
    dansUnEmplacement,
    avecSource,
    apresCopie,
    couper,
    depuisInstallationDos,
    nomDeDossier,
    LIBELLES_MAME,
    OPTIONS_MAME_DEFAUT,
    depuisMame,
    type OptionsMame,
    type TriMame,
    depuisDos,
    depuisManuel,
    depuisProgramme,
    depuisRoms,
    extensionsDe,
    devinerPlateforme,
    typesACocher,
    messageBilan,
    type JeuAImporter,
    type JeuDosTrouve,
    type RomTrouvee,
  } from './local';

  let { sorte }: { sorte: 'rom' | 'dos' | 'mame' | 'windows' | 'manuel' | 'installer-dos' } = $props();

  let dossier = $state('');
  // Valeur de départ seulement : le composant est recréé quand la source change ({#key}).
  let plateforme = $state(untrack(() => (sorte === 'windows' ? 'Windows' : sorte === 'dos' ? 'MS-DOS' : '')));
  /** Les types de fichiers du dossier (extension, nombre) et ceux que la personne garde cochés. */
  let types = $state<[string, number][]>([]);
  let coches = $state<string[]>([]);
  let lecture = $state(false);
  let recursif = $state(true);
  let titre = $state('');
  let fichier = $state('');
  let enCours = $state(false);

  let roms = $state<(RomTrouvee & { pris: boolean })[]>([]);
  let dos = $state<(JeuDosTrouve & { pris: boolean; programme: string })[]>([]);
  let cherche = $state(false);
  /** Les zips de contenus additionnels (DLC, avatars…) trouvés et écartés : ils ne sont pas des jeux. */
  let contenusEcartes = $state(0);
  let optionsMame = $state<OptionsMame>({ ...OPTIONS_MAME_DEFAUT });
  let tri = $state<TriMame | null>(null);
  let mamePris = $state<boolean[]>([]);

  // --- Installer un jeu DOS ---
  const dosbox = $derived(pourLeJeu(etat.pc.emulateurs, {}, 'MS-DOS', 0)?.programme ?? '');
  let source = $state('');
  let parent = $state(untrack(() => etat.pc.emplacements.systemes['MS-DOS']?.[0] ?? etat.pc.emplacements.defaut[0] ?? ''));
  const destination = $derived(parent.trim() && titre.trim() ? `${parent.trim().replace(/[\\/]+$/, '')}\\${nomDeDossier(titre)}` : '');
  let installes = $state<string[] | null>(null);
  let programmeDos = $state('');

  async function choisirSource(image: boolean) {
    const s = image
      ? await parcourir({ titre: 'Image du disque du jeu', extensions: ['iso', 'cue', 'img', 'ima'] })
      : await parcourir({ dossier: true, titre: 'Dossier du CD ou des disquettes du jeu' });
    if (s) {
      source = s;
      if (!titre.trim()) titre = couper(s).nom.replace(/\.(iso|cue|img|ima)$/i, '');
    }
  }

  async function choisirParent() {
    const d = await parcourir({ dossier: true, titre: 'Où installer le jeu (un sous-dossier à son nom y sera créé)' });
    if (d) parent = d;
  }

  async function lancerInstallation() {
    await informer('📀 L’installation va s’ouvrir dans DOSBox', [
      `Le disque du jeu est en D: (A: pour une disquette), et le dossier d’installation en C: (${destination}).`,
      'Dans DOSBox : tape INSTALL (ou SETUP) puis Entrée, et installe le jeu sur C:.',
      'Quand c’est fini, tape EXIT pour fermer DOSBox : Frogtend cherchera alors le programme du jeu.',
    ].join('\n'));
    enCours = true;
    try {
      installes = await api.importInstallerDos(dosbox, source, destination, titre);
      programmeDos = installes[0] ?? '';
      if (!installes.length) toast('Aucun programme trouvé dans le dossier d’installation : l’installation a-t-elle abouti ?', 'alerte');
    } catch (e) {
      toast(`Installation impossible : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      enCours = false;
    }
  }

  async function ajouterJeuDosInstalle() {
    try {
      const args = await api.importArgumentsJeuDos(destination, programmeDos);
      await ajouter([depuisInstallationDos(titre, destination, dosbox, args)]);
    } catch (e) {
      toast(`Impossible : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  async function choisirListeMame() {
    const f = await parcourir({ titre: 'La liste MAME de LaunchBox (Metadata\\MAME.xml)', extensions: ['xml'] });
    if (f) await reglerPc('listeMame', f);
  }

  const plateformes = $derived([...new Set([...ludo.plateformes.map((p) => p.nom), ...PLATEFORMES_CONNUES])].sort((a, b) => a.localeCompare(b, 'fr')));

  /**
   * Rien à taper (Seb, 03/10 : « entrer l'extension à la main n'est pas user friendly ») : le dossier choisi, Frogtend
   * propose la plateforme d'après son nom et montre les types de fichiers qu'il contient, ceux des jeux déjà cochés.
   */
  async function analyser() {
    if (sorte !== 'rom' || !dossier.trim()) return;
    cherche = false;
    roms = [];
    if (!plateforme.trim()) plateforme = devinerPlateforme(dossier, plateformes) ?? '';
    lecture = true;
    try {
      types = await api.importTypesFichiers(dossier.trim(), recursif);
    } catch (e) {
      types = [];
      toast(`Dossier illisible : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      lecture = false;
    }
    await recocher();
  }

  /** Les types cochés d'office : ceux que lisent les émulateurs de la plateforme (d'après Firehouse), plus les archives. */
  async function recocher() {
    cherche = false;
    const r = plateforme.trim() ? await api.emulateursRecommandes(plateforme.trim()).catch(() => null) : null;
    coches = typesACocher(types, extensionsDe(r?.emulateurs ?? []));
  }

  async function choisirDossier() {
    const d = await parcourir({ dossier: true, titre: sorte === 'dos' ? 'Dossier de tes jeux MS-DOS' : 'Dossier de tes ROM' });
    if (d) {
      dossier = d;
      if (sorte === 'rom') plateforme = '';
      await analyser();
    }
  }

  async function choisirFichier() {
    const f = await parcourir({ titre: sorte === 'windows' ? 'Programme du jeu' : 'Fichier du jeu', extensions: sorte === 'windows' ? ['exe', 'bat', 'lnk'] : undefined });
    if (!f) return;
    fichier = f;
    if (!titre.trim()) titre = depuisProgramme(f).titre;
  }

  async function chercher() {
    enCours = true;
    try {
      if (sorte === 'rom') {
        const l = await api.importChercherRoms(dossier, coches, recursif);
        roms = l.roms.map((r) => ({ ...r, pris: true }));
        contenusEcartes = l.contenus;
      } else if (sorte === 'mame') {
        tri = await api.importChercherMame(dossier, etat.pc.listeMame, $state.snapshot(optionsMame));
        mamePris = tri.retenus.map(() => true);
      } else {
        const l = await api.importChercherDos(dossier);
        dos = l.map((j) => ({ ...j, pris: true, programme: j.programmes[0] }));
      }
      cherche = true;
    } catch (e) {
      toast(`Recherche impossible : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      enCours = false;
    }
  }

  const mameChoisis = $derived(tri ? tri.retenus.filter((_, i) => mamePris[i]) : []);
  const choisis = $derived(
    sorte === 'rom' ? roms.filter((r) => r.pris).length : sorte === 'mame' ? mameChoisis.length : dos.filter((j) => j.pris).length,
  );
  const tailleChoisie = $derived(
    (sorte === 'mame' ? mameChoisis : roms.filter((r) => r.pris)).reduce((n, r) => n + r.taille, 0),
  );
  const ecartes = $derived(tri ? Object.entries(tri.ecartes).sort((a, b) => b[1] - a[1]) : []);

  /** 1 Go de marge gardée libre sur un disque (comme pour les téléchargements). */
  const MARGE = 1024 ** 3;

  /** Où garder ces jeux (règle « pas de pieuvre ») : rend les jeux à ajouter (copiés ou non), et la source à
   * ajouter aux emplacements du système, ou `null` si la personne renonce. */
  async function ouGarder(jeux: JeuAImporter[]): Promise<{ jeux: JeuAImporter[]; source: string | null } | null> {
    const p = jeux[0].plateforme;
    const e = etat.pc.emplacements;
    const elements = jeux.map(elementDe);
    if (elements.every((x) => dansUnEmplacement(x, e, p))) return { jeux, source: null };
    // La source à ajouter : le dossier choisi pour la recherche, sinon le dossier qui contient le jeu.
    const source = sorte === 'rom' || sorte === 'dos' || sorte === 'mame' ? dossier.trim() : couper(elements[0]).dossier;
    const cible = emplacementDuSysteme(e, p, nomDeDossier);
    let detailCopie = 'aucun emplacement de jeux réglé (⚙ Options ▸ Emplacements)';
    let copiable = false;
    if (cible) {
      const [octets, libre] = await api.importMesurer(elements, cible).catch(() => [0, null] as [number, number | null]);
      copiable = libre !== null && libre >= octets + MARGE;
      detailCopie = `dans ${cible} — ${taille(octets)} à copier, ${libre === null ? 'place inconnue' : `${taille(libre)} libres`}${copiable ? '' : ' : pas assez de place'}`;
    }
    const c = await choisir<'laisser' | 'copier'>(
      `📁 Où garder ${jeux.length > 1 ? `ces ${jeux.length} jeux` : 'ce jeu'} ?`,
      [
        { valeur: 'laisser', libelle: '📌 Les laisser où ils sont', detail: `${source} devient une source de ${p} (⚙ Options ▸ Emplacements)` },
        ...(copiable ? [{ valeur: 'copier' as const, libelle: '📥 Les copier dans l’emplacement du système', detail: detailCopie }] : []),
      ],
      copiable ? 'Les fichiers ne sont jamais déplacés ni renommés : les originaux restent.' : `Copie impossible : ${detailCopie}.`,
    );
    if (!c) return null;
    if (c === 'laisser') return { jeux, source };
    enCours = true;
    try {
      const nouveaux = await api.importCopier(elements, cible!);
      toast(`✅ ${nouveaux.length} jeu(x) copiés dans ${cible}.`);
      return { jeux: apresCopie(jeux, nouveaux), source: null };
    } catch (err) {
      toast(`Copie impossible : ${motifDuRefus(err)}`, 'erreur');
      return null;
    } finally {
      enCours = false;
    }
  }

  async function ajouter(jeuxChoisis: JeuAImporter[]) {
    if (!jeuxChoisis.length) return;
    if (jeuxChoisis.length > 1) {
      const oui = await confirmer(`➕ Ajouter ${jeuxChoisis.length} jeux à ta ludothèque ?`, {
        message: `Plateforme : ${jeuxChoisis[0].plateforme}.${tailleChoisie ? ` ${taille(tailleChoisie)} sur le disque.` : ''}\nLes versions d’un même jeu (France, Europe, USA…) sont réunies sous une seule fiche, la française d’abord.
Rien n’est déplacé ni renommé. Les retirer plus tard de ta ludothèque ne les efface pas.`,
        libelleValider: `Ajouter ${jeuxChoisis.length} jeux`,
      });
      if (!oui) return;
    }
    const ou = sorte === 'installer-dos' ? { jeux: jeuxChoisis, source: null } : await ouGarder(jeuxChoisis);
    if (!ou) return;
    const jeux = ou.jeux;
    enCours = true;
    try {
      const b = await api.importAjouter(jeux);
      if (ou.source && b.ajoutes) {
        const s = avecSource(etat.pc.emplacements, jeux[0].plateforme, ou.source);
        if (s) {
          await reglerPc('emplacements.systemes', s);
          toast(`📁 ${ou.source} est maintenant une source de ${jeux[0].plateforme} (⚙ Options ▸ Emplacements).`);
        }
      }
      toast(messageBilan(b), b.refuses.length ? 'alerte' : 'ok');
      await rechargerJeuxDuPc();
      if (ludo.espace === 'ludotheque') {
        await rechargerPlateformes();
        await rechargerListe();
      }
    } catch (e) {
      toast(`Import impossible : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      enCours = false;
    }
  }

  const tout = (v: boolean) => {
    if (sorte === 'rom') roms.forEach((r) => (r.pris = v));
    else if (sorte === 'mame') mamePris = mamePris.map(() => v);
    else dos.forEach((j) => (j.pris = v));
  };
</script>

<datalist id="plateformes-connues">
  {#each plateformes as p (p)}<option value={p}></option>{/each}
</datalist>

<section class="cx-block local">
  <p class="muted">Rien n’est copié, déplacé ni renommé : Frogtend note où est le jeu. Le retirer plus tard de ta ludothèque ne l’efface jamais du disque.</p>

  {#if sorte === 'installer-dos'}
    {#if !dosbox}
      <p class="tag warn">⚠ Aucun DOSBox réglé pour MS-DOS : règle-le d’abord dans <a href="/reglages?rubrique=emulateurs">⚙ Options ▸ Émulateurs</a>.</p>
    {/if}
    <div class="ligne">
      <button class="btn" onclick={() => choisirSource(false)}>📂 Dossier du CD</button>
      <button class="btn" onclick={() => choisirSource(true)}>💿 Image (.iso, .cue, .img)</button>
      <input class="large" bind:value={source} placeholder="E:\Disques\Kings Quest V.iso" aria-label="Source" />
    </div>
    <label class="ligne">Titre <input class="large" bind:value={titre} placeholder="King's Quest V" /></label>
    <div class="ligne">
      <button class="btn" onclick={choisirParent}>📁 Où installer</button>
      <input class="large" bind:value={parent} placeholder="D:\Jeux\MS-DOS" aria-label="Dossier parent" />
    </div>
    {#if destination}<p class="muted">Le jeu sera installé dans <strong>{destination}</strong> (un dossier vide ou nouveau : Frogtend n’écrit jamais par-dessus).</p>{/if}
    <div class="ligne">
      <button class="btn primary" onclick={lancerInstallation} disabled={enCours || !dosbox || !source.trim() || !destination}>
        {enCours ? 'DOSBox est ouvert…' : '▶ Lancer l’installation dans DOSBox'}
      </button>
    </div>
    {#if installes?.length}
      <label class="ligne">
        Programme qui lance le jeu
        <select bind:value={programmeDos}>
          {#each installes as p (p)}<option value={p}>{p}</option>{/each}
        </select>
      </label>
      <div class="ligne">
        <button class="btn primary" onclick={ajouterJeuDosInstalle} disabled={enCours || !programmeDos}>➕ Ajouter à ma ludothèque</button>
      </div>
      <p class="muted">Il se lancera par DOSBox, avec C: sur son dossier d’installation, comme pendant l’installation.</p>
    {/if}
  {:else if sorte === 'mame'}
    <div class="ligne">
      <button class="btn" onclick={choisirDossier}>📂 Choisir le dossier des ROM MAME</button>
      <input class="large" bind:value={dossier} placeholder="E:\Games\MAME" aria-label="Dossier" />
    </div>
    <div class="ligne">
      <button class="btn" onclick={choisirListeMame}>📄 Choisir la liste MAME</button>
      <span class="muted large">{etat.pc.listeMame || 'pas encore choisie : le fichier Metadata\\MAME.xml de ton LaunchBox'}</span>
    </div>
    <p class="muted">La liste MAME de LaunchBox dit, pour chaque zip, si c’est un jeu, un clone, un BIOS, s’il marche… Frogtend la lit sans la modifier. (Firehouse la servira plus tard.)</p>
    <fieldset class="cases">
      <legend>Garder aussi :</legend>
      {#each LIBELLES_MAME as [cle, libelle] (cle)}
        <label><input type="checkbox" bind:checked={optionsMame[cle]} /> {libelle}</label>
      {/each}
    </fieldset>
    <div class="ligne">
      <button class="btn primary" onclick={chercher} disabled={enCours || !dossier.trim() || !etat.pc.listeMame}>
        {enCours ? 'Tri en cours… (quelques secondes)' : '🔍 Trier le dossier'}
      </button>
    </div>
    {#if tri}
      <p>
        <strong>{tri.retenus.length} jeu(x) retenu(s)</strong>
        {#if ecartes.length}
          · écartés : {ecartes.map(([m, n]) => `${n} ${m}`).join(', ')}
        {/if}
      </p>
      {#if tri.retenus.length}
        <div class="ligne">
          <strong>{choisis} coché(s)</strong>
          <span class="muted">{taille(tailleChoisie)}</span>
          <button class="btn petit" onclick={() => tout(true)}>Tout cocher</button>
          <button class="btn petit" onclick={() => tout(false)}>Tout décocher</button>
          <span class="espace"></span>
          <button class="btn primary" disabled={enCours || !choisis} onclick={() => ajouter(depuisMame(mameChoisis))}>➕ Ajouter {choisis} jeu(x)</button>
        </div>
        <ul class="liste">
          {#each tri.retenus as j, i (j.chemin)}
            <li>
              <label><input type="checkbox" bind:checked={mamePris[i]} /> {j.titre}</label>
              <span class="muted">{j.annee ?? ''} {j.editeur ?? ''} · {j.chemin.split(/[\\/]/).pop()}</span>
            </li>
          {/each}
        </ul>
        <p class="muted">Plateforme « Arcade » ; ces jeux se lancent par l’émulateur réglé pour Arcade (MAME), avec le zip tel quel.</p>
      {/if}
    {/if}
  {:else if sorte === 'rom' || sorte === 'dos'}
    <div class="ligne">
      <button class="btn" onclick={choisirDossier}>📂 Choisir le dossier</button>
      <input class="large" bind:value={dossier} onchange={() => sorte === 'rom' && analyser()} placeholder="E:\Jeux\ROM\SNES" aria-label="Dossier" />
    </div>
    {#if sorte === 'rom'}
      <label class="ligne">
        Plateforme
        <select class="large" bind:value={plateforme} onchange={recocher}>
          <option value="">— choisis la plateforme —</option>
          {#if plateforme && !plateformes.includes(plateforme)}<option value={plateforme}>{plateforme}</option>{/if}
          {#each plateformes as p (p)}<option value={p}>{p}</option>{/each}
        </select>
      </label>
      {#if dossier.trim() && !plateforme}
        <p class="muted">Frogtend n’a pas reconnu la plateforme d’après le nom du dossier : choisis-la dans la liste.</p>
      {/if}
      <label class="ligne"><input type="checkbox" bind:checked={recursif} onchange={analyser} /> chercher aussi dans les sous-dossiers</label>
      {#if lecture}
        <p class="muted">Lecture du dossier…</p>
      {:else if types.length}
        <fieldset class="types">
          <legend>Types de fichiers trouvés — ceux des jeux sont cochés</legend>
          {#each types as [ext, n] (ext)}
            <label><input type="checkbox" bind:group={coches} value={ext} /> .{ext} <span class="muted">({n.toLocaleString('fr-FR')})</span></label>
          {/each}
        </fieldset>
        <p class="muted">Pour un jeu sur CD, c’est sa feuille (.cue, .gdi, .m3u) qui compte : ses pistes .bin ne sont pas des jeux à part.</p>
      {:else if dossier.trim()}
        <p class="muted">Aucun fichier dans ce dossier.</p>
      {/if}
    {/if}
    <div class="ligne">
      <button class="btn primary" onclick={chercher} disabled={enCours || lecture || !dossier.trim() || (sorte === 'rom' && (!plateforme.trim() || !coches.length))}>
        {enCours ? 'Recherche…' : '🔍 Chercher'}
      </button>
    </div>

    {#if cherche}
      {#if sorte === 'rom' && contenusEcartes}
        <p class="muted">📦 {contenusEcartes} contenu(s) additionnel(s) trouvé(s) (DLC, avatars…) : ce ne sont pas des jeux. Ils apparaîtront dans le panneau de leur jeu, prêts à être installés si tu le souhaites.</p>
      {/if}
      {#if (sorte === 'rom' ? roms.length : dos.length) === 0}
        <p>Aucun jeu trouvé dans ce dossier{sorte === 'rom' ? ' parmi les types cochés' : ' (un jeu = un sous-dossier avec un programme .exe, .com ou .bat)'}.</p>
      {:else}
        <div class="ligne">
          <strong>{choisis} jeu(x) coché(s)</strong>
          {#if sorte === 'rom'}<span class="muted">{taille(tailleChoisie)}</span>{/if}
          <button class="btn petit" onclick={() => tout(true)}>Tout cocher</button>
          <button class="btn petit" onclick={() => tout(false)}>Tout décocher</button>
          <span class="espace"></span>
          <button
            class="btn primary"
            disabled={enCours || !choisis}
            onclick={() => ajouter(sorte === 'rom' ? depuisRoms(roms.filter((r) => r.pris), plateforme.trim()) : depuisDos(dos.filter((j) => j.pris)))}
          >
            ➕ Ajouter {choisis} jeu(x)
          </button>
        </div>
        <ul class="liste">
          {#if sorte === 'rom'}
            {#each roms as r (r.chemin)}
              <li>
                <label><input type="checkbox" bind:checked={r.pris} /> <input class="titre" bind:value={r.titre} aria-label="Titre" /></label>
                <span class="muted" title={r.chemin}>{r.chemin.split(/[\\/]/).pop()} · {taille(r.taille)}</span>
              </li>
            {/each}
          {:else}
            {#each dos as j (j.dossier)}
              <li>
                <label><input type="checkbox" bind:checked={j.pris} /> <input class="titre" bind:value={j.titre} aria-label="Titre" /></label>
                <select bind:value={j.programme} aria-label="Programme qui lance le jeu">
                  {#each j.programmes as p (p)}<option value={p}>{p}</option>{/each}
                </select>
              </li>
            {/each}
          {/if}
        </ul>
        {#if sorte === 'dos'}<p class="muted">Ces jeux se lancent par l’émulateur réglé pour MS-DOS (DOSBox), avec le programme choisi.</p>{/if}
      {/if}
    {/if}
  {:else}
    <div class="ligne">
      <button class="btn" onclick={choisirFichier}>📂 {sorte === 'windows' ? 'Choisir le programme (.exe)' : 'Choisir le fichier du jeu'}</button>
      <input class="large" bind:value={fichier} placeholder={sorte === 'windows' ? 'D:\\Jeux\\Doom\\doom.exe' : 'E:\\ROM\\N64\\Zelda (Europe).z64'} aria-label="Fichier" />
    </div>
    <label class="ligne">Titre <input class="large" bind:value={titre} /></label>
    {#if sorte === 'manuel'}
      <label class="ligne">Plateforme <input class="large" list="plateformes-connues" bind:value={plateforme} placeholder="Windows, Nintendo 64…" /></label>
      <p class="muted">Pour « Windows », le fichier est le programme du jeu ; pour une console, c’est la ROM ou l’image disque, donnée à l’émulateur réglé.</p>
    {/if}
    <div class="ligne">
      <button
        class="btn primary"
        disabled={enCours || !fichier.trim() || !titre.trim() || !plateforme.trim()}
        onclick={() => ajouter([sorte === 'windows' ? depuisProgramme(fichier.trim(), titre) : depuisManuel(titre, plateforme.trim(), fichier.trim())])}
      >
        ➕ Ajouter à ma ludothèque
      </button>
    </div>
  {/if}
</section>

<style>
  .local {
    display: grid;
    gap: calc(10 * var(--u));
  }
  p {
    margin: 0;
  }
  .ligne {
    display: flex;
    flex-wrap: wrap;
    gap: calc(8 * var(--u));
    align-items: center;
  }
  .large {
    flex: 1;
    min-width: calc(240 * var(--u));
  }
  .espace {
    flex: 1;
  }
  .types {
    display: flex;
    flex-wrap: wrap;
    gap: calc(6 * var(--u)) calc(16 * var(--u));
    margin: 0;
    padding: calc(8 * var(--u)) calc(10 * var(--u));
    border: 1px solid var(--line);
    border-radius: calc(6 * var(--u));
  }
  .types legend {
    padding: 0 calc(4 * var(--u));
    color: var(--dim);
  }
  .liste {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: calc(420 * var(--u));
    overflow: auto;
    border: 1px solid var(--line);
    border-radius: var(--radius, 6px);
  }
  .liste li {
    display: flex;
    flex-wrap: wrap;
    gap: calc(8 * var(--u));
    align-items: center;
    justify-content: space-between;
    padding: calc(4 * var(--u)) calc(8 * var(--u));
    border-bottom: 1px solid var(--line);
  }
  .liste label {
    display: flex;
    gap: calc(6 * var(--u));
    align-items: center;
  }
  .titre {
    min-width: calc(260 * var(--u));
  }
  .cases {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(calc(300 * var(--u)), 1fr));
    gap: calc(4 * var(--u)) calc(12 * var(--u));
    border: 1px solid var(--line);
    border-radius: var(--radius, 6px);
    padding: calc(8 * var(--u)) calc(12 * var(--u));
    margin: 0;
  }
  .cases label {
    display: flex;
    gap: calc(6 * var(--u));
    align-items: center;
  }
</style>
