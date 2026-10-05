<script lang="ts">
  // Les émulateurs réglés sur CE PC, par système. Retirer un réglage ne désinstalle rien.
  import { onMount } from 'svelte';
  import { api, type EmulateurInstalle, type Plateforme } from '$lib/api';
  import { choisir, confirmer, demander, informer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { etat, reglerPc } from '$lib/etat.svelte';
  import { dossierEmulateurs, installerEmulateur, parcourir, reglerEmulateur } from '$lib/emulateurs/assistant.svelte';
  import { plusRecente } from '$lib/emulateurs/versions';
  import { retirer as retirerEmulateur, systemesProposes } from '$lib/emulateurs/choix';
  import { PLATEFORMES_CONNUES } from '$lib/ludotheque/categories';
  import { tele } from '$lib/ludotheque/telechargements.svelte';
  import { GENRES_MANETTE } from '$lib/ludotheque/commandes';

  const e = $derived(Object.entries(etat.pc.emulateurs));
  let plateformes = $state<Plateforme[]>([]);
  let installes = $state<EmulateurInstalle[]>([]);
  /** Dernière version connue, par émulateur (après « Chercher des mises à jour »). */
  let dernieres = $state<Record<string, string>>({});
  let recherche = $state(false);
  let traces = $state<{ id: string; nom: string; dossiers: string[] }[]>([]);

  async function recharger() {
    installes = await api.emulateursInstalles().catch(() => []);
    traces = await api.emulateursTraces().catch(() => []);
  }
  onMount(async () => {
    // Les jeux de ce PC (importés compris) et ceux du catalogue ; puis toutes les plateformes connues.
    const [pc, catalogue] = await Promise.all([api.plateformes(true).catch(() => []), api.plateformes(false).catch(() => [])]);
    plateformes = systemesProposes([...pc, ...catalogue], [...PLATEFORMES_CONNUES, ...Object.keys(etat.pc.emulateurs)]) as Plateforme[];
    await recharger();
  });

  async function chercherMisesAJour() {
    recherche = true;
    let n = 0;
    for (const e of installes.filter((e) => e.par_frogtend)) {
      try {
        const p = await api.emulateurDerniereVersion(e.id);
        dernieres[e.id] = p.version;
        if (e.version && plusRecente(p.version, e.version)) n++;
      } catch {
        // hors ligne : on n'en sait pas plus
      }
    }
    recherche = false;
    toast(n ? `⬆ ${n} mise(s) à jour disponible(s).` : '✅ Tes émulateurs sont à jour.');
  }

  async function mettreAJour(e: EmulateurInstalle) {
    if (await installerEmulateur(e.id, e.nom, true)) await recharger();
  }

  /** Ce que la manette par défaut règle, par émulateur (les autres reconnaissent déjà les manettes seuls). */
  const MANETTE: Record<string, string> = {
    retroarch: 'Profils de manette officiels (téléchargés depuis libretro s’ils manquent) ; menu à la manette : L3 + R3.',
    duckstation: 'Manette du joueur 1 (Xbox, PlayStation, Switch Pro, 8BitDo…) ; menu de pause : Select + Start.',
    pcsx2: 'Manette du joueur 1 (Xbox, PlayStation, Switch Pro, 8BitDo…) ; menu de pause : Select + Start.',
    dolphin: 'Manette GameCube du joueur 1 sur une manette Xbox (XInput), pour ton profil.',
  };

  async function manetteParDefaut(i: EmulateurInstalle) {
    const oui = await confirmer(`🎮 Remettre la manette par défaut dans ${i.nom} ?`, {
      message: [
        MANETTE[i.id],
        'Ce que tu avais réglé pour la manette du joueur 1 est remplacé ; les touches du clavier sont gardées.',
        'La configuration d’avant est d’abord copiée à part, dans le dossier de l’émulateur.',
      ].join('\n'),
      libelleValider: '🎮 Remettre par défaut',
    });
    if (!oui) return;
    try {
      const r = await api.emulateurReglerManette(i.id, i.programme);
      toast(`✅ Manette réglée dans ${i.nom}.` + (r.profils_ajoutes ? ` ${r.profils_ajoutes} profil(s) de manette ajouté(s).` : ''));
    } catch (e) {
      toast(`Impossible de régler la manette : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  /** Les émulateurs dont Frogtend sait reprendre les profils de manette. */
  const AVEC_PROFILS = ['dolphin', 'duckstation', 'pcsx2'];

  /** Le micrologiciel PS3 (fichier officiel de Sony, jamais téléchargé par Frogtend) installé par RPCS3. */
  async function micrologicielPs3(i: EmulateurInstalle) {
    const actuel = await api.rpcs3Micrologiciel(i.programme).catch(() => null);
    const ok = await confirmer('💿 Micrologiciel PS3', {
      message: [
        actuel ? `Installé : version ${actuel}.` : 'Pas encore installé : les jeux PS3 du commerce en ont besoin.',
        'Bientôt, Firehouse le fournira à jour et Frogtend l’installera tout seul. En attendant : télécharge le fichier PS3UPDAT.PUP sur le site officiel de PlayStation (page « Mise à jour du logiciel système PS3 »), puis montre-le ici. L’installation se fait sans aucune fenêtre (quelques secondes).',
      ].join('\n'),
      libelleValider: '📂 Choisir PS3UPDAT.PUP',
    });
    if (!ok) return;
    const pup = await parcourir({ titre: 'Fichier PS3UPDAT.PUP', extensions: ['pup', 'PUP'] });
    if (!pup) return;
    toast('💿 Installation du micrologiciel…');
    try {
      const v = await api.rpcs3InstallerMicrologiciel(i.programme, pup);
      toast(`✅ Micrologiciel PS3 ${v} installé. La première partie de chaque jeu prépare RPCS3 quelques minutes, une seule fois.`);
    } catch (e) {
      toast(`Impossible : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  /** Faire d'un profil enregistré dans l'émulateur un réglage de référence de Frogtend. */
  async function reprendreReference(i: EmulateurInstalle) {
    let natifs;
    try {
      natifs = await api.profilsManetteEmulateur(i.id, i.programme);
    } catch (e) {
      toast(`Impossible de lire les profils de ${i.nom} : ${motifDuRefus(e)}`, 'erreur');
      return;
    }
    if (natifs.length === 0) {
      await informer(
        `📌 Aucun profil de manette dans ${i.nom}`,
        `Ouvre ${i.nom} (depuis son dossier : ${i.dossier}), règle la manette dans ses options, puis enregistre-la comme profil avec son bouton « Enregistrer » (ou « Save »). Elle apparaîtra ici.`,
      );
      return;
    }
    const refs = await api.referencesManette(i.id).catch(() => []);
    const p = await choisir(
      `📌 Quel profil de ${i.nom} devient une référence ?`,
      natifs.map((n) => ({
        valeur: n,
        libelle: `${GENRES_MANETTE[n.genre] ?? n.genre} : ${n.nom}`,
        detail: refs.some((r) => r.genre === n.genre && r.nom === n.nom) ? 'remplacera la référence de même nom' : 'nouvelle référence',
      })),
      'Une référence est proposée pour chaque jeu (⚙ Gérer le jeu ▸ 🎮 Commandes), et déposée dans les profils de tous.',
    );
    if (!p) return;
    const nom = await demander('📌 Nom de la référence', {
      message:
        'Garde le même nom qu’une référence existante pour la remplacer (par exemple « Wiimote + Nunchuk », utilisée d’office pour la Wii).',
      valeur: p.nom,
      libelleValider: '📌 En faire une référence',
    });
    if (!nom) return;
    try {
      const r = await api.referenceReprendre(i.id, i.programme, p.genre, p.chemin, nom);
      toast(`✅ « ${r.nom} » est un réglage de référence de ${i.nom} sur ce PC.`);
    } catch (e) {
      toast(`Impossible d’en faire une référence : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  /** Combien de jeux de ce système ont choisi cet émulateur pour eux (une clé n'est unique que dans son système). */
  function jeuxAvec(systeme: string, cle: string) {
    const n = Object.entries(etat.pc.emulateursJeux).filter(([id, c]) => c === cle && tele.jeux[Number(id)]?.plateforme === systeme).length;
    return n ? `${n} jeu(x) l’ont choisi` : '';
  }

  async function parDefaut(systeme: string, cle: string) {
    const s = etat.pc.emulateurs[systeme];
    await reglerPc('emulateurs', { ...etat.pc.emulateurs, [systeme]: { ...s, defaut: cle } });
    toast(`⭐ ${s.liste.find((x) => x.cle === cle)?.nom} : par défaut pour ${systeme}.`);
  }

  async function retirer(systeme: string, cle: string) {
    const s = etat.pc.emulateurs[systeme];
    const nom = s.liste.find((x) => x.cle === cle)?.nom;
    const reste = { ...etat.pc.emulateurs };
    const apres = retirerEmulateur(s, cle);
    if (apres.liste.length) reste[systeme] = apres;
    else delete reste[systeme];
    await reglerPc('emulateurs', reste);
    toast(`${nom} retiré de ${systeme} (il n’est pas désinstallé).`);
  }

  async function ajouter() {
    const s = await choisir(
      '🕹 Pour quel système ?',
      plateformes.map((p) => ({ valeur: p.nom, libelle: p.nom, detail: p.jeux ? `${p.jeux} jeu(x)` : undefined })),
    );
    if (s) {
      await reglerEmulateur(s);
      // Un émulateur vient peut-être d’être installé : « Sur ce PC » le montre aussitôt (vu par Seb le 03/10).
      await recharger();
    }
  }
</script>

<div class="emulateurs">
  <dl class="cx-kv">
    <dt>Touche du menu en jeu</dt>
    <dd>
      <select
        value={etat.pc.menuJeu.touche}
        title="Pendant une partie, cette touche ouvre le menu de Frogtend par-dessus le jeu (reprendre, sauvegarde rapide, manuel, quitter…)."
        onchange={(e) => reglerPc('menuJeu.touche', e.currentTarget.value)}
      >
        <option value="Pause">Pause/Attn</option>
        <option value="ScrollLock">Arrêt défil</option>
        <option value="Ctrl+Shift+M">Ctrl + Maj + M</option>
      </select>
    </dd>
    <dt>Combinaison de la manette</dt>
    <dd>
      <select
        value={etat.pc.menuJeu.manette ?? 'Select + R1'}
        title="Pendant une partie, ces deux boutons tenus 1 seconde ouvrent le menu de Frogtend par-dessus le jeu. Choisis une combinaison que tes jeux n’utilisent pas. Prend effet à la prochaine partie."
        onchange={(e) => reglerPc('menuJeu.manette', e.currentTarget.value)}
      >
        <option value="Select + R1">Select + R1 (View + RB), tenus 1 s</option>
        <option value="Select + L1">Select + L1 (View + LB), tenus 1 s</option>
        <option value="L3 + R3">L3 + R3 (clic des deux sticks), tenus 1 s</option>
        <option value="Select + Start">Select + Start, tenus 1 s</option>
      </select>
    </dd>
    <dt>Après « Quitter le jeu »</dt>
    <dd>
      <select
        value={String(etat.pc.menuJeu.delaiQuitter ?? 10)}
        title="Si l'émulateur reste en vie sans fenêtre après « Quitter le jeu », Frogtend l'arrête au bout de ce délai. Un émulateur qui a encore sa fenêtre n'est jamais arrêté (il enregistre peut-être ta partie)."
        onchange={(e) => reglerPc('menuJeu.delaiQuitter', Number(e.currentTarget.value))}
      >
        <option value="5">l'arrêter après 5 secondes s'il reste caché</option>
        <option value="10">l'arrêter après 10 secondes s'il reste caché</option>
        <option value="30">l'arrêter après 30 secondes s'il reste caché</option>
      </select>
    </dd>
    <dt>Dossier des émulateurs</dt>
    <dd>
      <span class="chemin">{etat.pc.dossierEmulateurs || 'pas encore choisi'}</span>
      <button class="btn petit" onclick={() => dossierEmulateurs(true)}>Changer</button>
    </dd>
  </dl>

  {#if traces.length}
    <div class="cx-block alerte">
      <h3>⚠ Hors du dossier des émulateurs</h3>
      <p class="muted">
        Ces dossiers ont été créés dans ton dossier utilisateur de Windows par un émulateur qui n’était pas en mode
        portable (installation ancienne ou faite à la main). Frogtend ne les touche pas : vérifie ce qu’ils
        contiennent (des parties ?) avant de les ranger ou de les effacer toi-même.
      </p>
      <ul>
        {#each traces as t (t.id)}
          {#each t.dossiers as d (d)}<li><strong>{t.nom}</strong> : <span class="chemin">{d}</span></li>{/each}
        {/each}
      </ul>
    </div>
  {/if}

  {#if installes.length}
    <div class="cx-block">
      <h3>Sur ce PC</h3>
      <dl class="cx-kv">
        {#each installes as i (i.id)}
          <dt>{i.nom}</dt>
          <dd>
            <span class="muted">{i.version ?? 'installé à la main'}</span>
            {#if i.par_frogtend && dernieres[i.id] && i.version && plusRecente(dernieres[i.id], i.version)}
              <button class="btn petit primary" onclick={() => mettreAJour(i)}>⬆ {dernieres[i.id]}</button>
            {/if}
            {#if MANETTE[i.id]}
              <button class="btn petit" title={MANETTE[i.id]} onclick={() => manetteParDefaut(i)}>🎮 Manette par défaut</button>
            {/if}
            {#if i.id === 'rpcs3'}
              <button class="btn petit" title="Les jeux PS3 du commerce en ont besoin" onclick={() => micrologicielPs3(i)}>
                💿 Micrologiciel PS3…
              </button>
            {/if}
            {#if AVEC_PROFILS.includes(i.id)}
              <button class="btn petit" title="Faire d’un profil de manette de l’émulateur un réglage de référence" onclick={() => reprendreReference(i)}>
                📌 Réglages de référence
              </button>
            {/if}
          </dd>
        {/each}
      </dl>
      <button class="btn" onclick={chercherMisesAJour} disabled={recherche}>
        {recherche ? 'Recherche…' : '🔎 Chercher des mises à jour'}
      </button>
    </div>
  {/if}

  <p class="muted">
    Les émulateurs de chaque système, sur ce PC : autant que tu veux (par exemple plusieurs cœurs RetroArch), ⭐ celui
    par défaut. Un jeu peut en avoir un autre (⚙ Gérer le jeu ▸ 🕹 Émulateur), et « Jouer avec… » en choisit un pour une
    seule partie. Frogtend propose tout seul le recommandé par Firehouse la première fois. Les jeux PC et MS-DOS n’en
    ont pas besoin.
  </p>
  {#if e.length === 0}
    <p class="muted">Aucun émulateur réglé pour l’instant.</p>
  {:else}
    {#each e as [systeme, reglage] (systeme)}
      <div class="cx-block systeme">
        <h3>{systeme}</h3>
        <ul>
          {#each reglage.liste as emu (emu.cle)}
            <li>
              <button
                class="etoile"
                class:actif={emu.cle === reglage.defaut}
                title={emu.cle === reglage.defaut ? 'Par défaut pour ce système' : 'En faire celui par défaut'}
                onclick={() => parDefaut(systeme, emu.cle)}
              >
                {emu.cle === reglage.defaut ? '⭐' : '☆'}
              </button>
              <span class="chemin" title={`${emu.programme} ${emu.ligne}`}>{emu.nom || emu.programme}</span>
              <span class="muted">{jeuxAvec(systeme, emu.cle)}</span>
              <button class="btn petit" title="Retirer de ce système (rien n’est désinstallé)" onclick={() => retirer(systeme, emu.cle)}>✕</button>
            </li>
          {/each}
        </ul>
        <button class="btn petit" onclick={async () => { await reglerEmulateur(systeme); await recharger(); }}>➕ Ajouter un émulateur</button>
      </div>
    {/each}
  {/if}
  <button class="btn" onclick={ajouter} disabled={plateformes.length === 0}>🕹 Régler un autre système…</button>
</div>

<style>
  .emulateurs {
    display: grid;
    gap: calc(10 * var(--u));
  }
  .emulateurs > .btn {
    justify-self: start;
  }
  .muted {
    margin: 0;
  }
  .alerte {
    box-shadow: inset calc(3 * var(--u)) 0 0 var(--warn);
  }
  .alerte ul {
    margin: calc(6 * var(--u)) 0 0;
    padding-left: calc(18 * var(--u));
  }
  dd {
    display: flex;
    gap: calc(6 * var(--u));
    align-items: center;
    justify-content: flex-end;
  }
  .chemin {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .systeme h3 {
    margin: 0 0 calc(6 * var(--u));
  }
  .systeme ul {
    list-style: none;
    margin: 0 0 calc(8 * var(--u));
    padding: 0;
    display: grid;
    gap: calc(4 * var(--u));
  }
  .systeme li {
    display: flex;
    align-items: center;
    gap: calc(8 * var(--u));
  }
  .systeme li .chemin {
    flex: 1;
    white-space: nowrap;
  }
  .etoile {
    background: none;
    border: 0;
    padding: 0 calc(2 * var(--u));
    font-size: 1.1em;
    cursor: pointer;
    color: var(--dim);
  }
  .etoile.actif {
    color: var(--warn);
  }
  .petit {
    min-height: calc(30 * var(--u));
    padding: calc(2 * var(--u)) calc(9 * var(--u));
  }
</style>
