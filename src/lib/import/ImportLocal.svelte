<script lang="ts">
  // La fenêtre d'import des jeux déjà sur le disque : ROM d'un dossier, jeux MS-DOS (un par sous-dossier), un jeu
  // Windows, ou un ajout manuel. On CHERCHE d'abord, la personne voit la liste et coche, puis on ajoute. Rien n'est
  // copié, déplacé ni renommé.
  import { untrack } from 'svelte';
  import { api, taille } from '$lib/api';
  import { confirmer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { parcourir } from '$lib/emulateurs/assistant.svelte';
  import { PLATEFORMES_CONNUES } from '$lib/ludotheque/categories';
  import { ludo, rechargerListe, rechargerPlateformes } from '$lib/ludotheque/ludotheque.svelte';
  import { rechargerJeuxDuPc } from '$lib/ludotheque/telechargements.svelte';
  import { etat, reglerPc } from '$lib/etat.svelte';
  import {
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
    lireExtensions,
    messageBilan,
    type JeuAImporter,
    type JeuDosTrouve,
    type RomTrouvee,
  } from './local';

  let { sorte }: { sorte: 'rom' | 'dos' | 'mame' | 'windows' | 'manuel' } = $props();

  let dossier = $state('');
  // Valeur de départ seulement : le composant est recréé quand la source change ({#key}).
  let plateforme = $state(untrack(() => (sorte === 'windows' ? 'Windows' : sorte === 'dos' ? 'MS-DOS' : '')));
  let extensions = $state('');
  let recursif = $state(true);
  let titre = $state('');
  let fichier = $state('');
  let enCours = $state(false);

  let roms = $state<(RomTrouvee & { pris: boolean })[]>([]);
  let dos = $state<(JeuDosTrouve & { pris: boolean; programme: string })[]>([]);
  let cherche = $state(false);
  let optionsMame = $state<OptionsMame>({ ...OPTIONS_MAME_DEFAUT });
  let tri = $state<TriMame | null>(null);
  let mamePris = $state<boolean[]>([]);

  async function choisirListeMame() {
    const f = await parcourir({ titre: 'La liste MAME de LaunchBox (Metadata\\MAME.xml)', extensions: ['xml'] });
    if (f) await reglerPc('listeMame', f);
  }

  const plateformes = $derived([...new Set([...ludo.plateformes.map((p) => p.nom), ...PLATEFORMES_CONNUES])].sort((a, b) => a.localeCompare(b, 'fr')));

  /** Les extensions lues par les émulateurs recommandés de la plateforme (modifiables). */
  async function proposerExtensions() {
    if (sorte !== 'rom' || !plateforme.trim()) return;
    const r = await api.emulateursRecommandes(plateforme.trim()).catch(() => null);
    const l = extensionsDe(r?.emulateurs ?? []);
    if (l.length) extensions = l.join(', ');
  }

  async function choisirDossier() {
    const d = await parcourir({ dossier: true, titre: sorte === 'dos' ? 'Dossier de tes jeux MS-DOS' : 'Dossier de tes ROM' });
    if (d) dossier = d;
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
        const l = await api.importChercherRoms(dossier, lireExtensions(extensions), recursif);
        roms = l.map((r) => ({ ...r, pris: true }));
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

  async function ajouter(jeux: JeuAImporter[]) {
    if (!jeux.length) return;
    if (jeux.length > 1) {
      const oui = await confirmer(`➕ Ajouter ${jeux.length} jeux à ta ludothèque ?`, {
        message: `Plateforme : ${jeux[0].plateforme}.${tailleChoisie ? ` ${taille(tailleChoisie)} sur le disque.` : ''}\nRien n’est copié, déplacé ni renommé : Frogtend note seulement où sont les jeux. Les retirer plus tard ne les efface pas.`,
        libelleValider: `Ajouter ${jeux.length} jeux`,
      });
      if (!oui) return;
    }
    enCours = true;
    try {
      const b = await api.importAjouter(jeux);
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

  {#if sorte === 'mame'}
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
      <input class="large" bind:value={dossier} placeholder="E:\Jeux\ROM\SNES" aria-label="Dossier" />
    </div>
    {#if sorte === 'rom'}
      <label class="ligne">
        Plateforme
        <input class="large" list="plateformes-connues" bind:value={plateforme} onchange={proposerExtensions} placeholder="Super Nintendo Entertainment System" />
      </label>
      <label class="ligne">
        Extensions
        <input class="large" bind:value={extensions} placeholder="sfc, smc, zip" />
      </label>
      <p class="muted">Proposées d’après les émulateurs de la plateforme. Pour les jeux sur CD, mets « cue » ou « m3u » : leurs pistes (.bin) ne comptent pas comme des jeux à part.</p>
      <label class="ligne"><input type="checkbox" bind:checked={recursif} /> chercher aussi dans les sous-dossiers</label>
    {/if}
    <div class="ligne">
      <button class="btn primary" onclick={chercher} disabled={enCours || !dossier.trim() || (sorte === 'rom' && (!plateforme.trim() || !extensions.trim()))}>
        {enCours ? 'Recherche…' : '🔍 Chercher'}
      </button>
    </div>

    {#if cherche}
      {#if (sorte === 'rom' ? roms.length : dos.length) === 0}
        <p>Aucun jeu trouvé dans ce dossier{sorte === 'rom' ? ' avec ces extensions' : ' (un jeu = un sous-dossier avec un programme .exe, .com ou .bat)'}.</p>
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
