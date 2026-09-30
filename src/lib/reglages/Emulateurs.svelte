<script lang="ts">
  // Les émulateurs réglés sur CE PC, par système. Retirer un réglage ne désinstalle rien.
  import { onMount } from 'svelte';
  import { api, type EmulateurInstalle, type Plateforme } from '$lib/api';
  import { choisir, confirmer, demander, informer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { etat, reglerPc } from '$lib/etat.svelte';
  import { dossierEmulateurs, installerEmulateur, reglerEmulateur } from '$lib/emulateurs/assistant.svelte';
  import { plusRecente } from '$lib/emulateurs/versions';
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
    plateformes = await api.plateformes(false).catch(() => []);
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

  async function retirer(systeme: string) {
    const reste = { ...etat.pc.emulateurs };
    delete reste[systeme];
    await reglerPc('emulateurs', reste);
    toast(`Réglage retiré pour ${systeme} (l’émulateur n’est pas désinstallé).`);
  }

  async function ajouter() {
    const s = await choisir(
      '🕹 Pour quel système ?',
      plateformes.map((p) => ({ valeur: p.nom, libelle: p.nom, detail: `${p.jeux} jeu(x)` })),
    );
    if (s) await reglerEmulateur(s);
  }
</script>

<div class="emulateurs">
  <dl class="cx-kv">
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
    L’émulateur de chaque système, sur ce PC. Frogtend le propose tout seul la première fois que tu lances un jeu qui
    en a besoin, d’après les recommandations de Firehouse. Les jeux PC et MS-DOS n’en ont pas besoin.
  </p>
  {#if e.length === 0}
    <p class="muted">Aucun émulateur réglé pour l’instant.</p>
  {:else}
    <dl class="cx-kv">
      {#each e as [systeme, reglage] (systeme)}
        <dt>{systeme}</dt>
        <dd>
          <span class="chemin" title={`${reglage.programme} ${reglage.ligne}`}>{reglage.nom ?? reglage.programme}</span>
          <button class="btn petit" onclick={() => reglerEmulateur(systeme)}>Changer</button>
          <button class="btn petit" onclick={() => retirer(systeme)}>✕</button>
        </dd>
      {/each}
    </dl>
  {/if}
  <button class="btn" onclick={ajouter} disabled={plateformes.length === 0}>🕹 Régler un émulateur…</button>
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
  .petit {
    min-height: calc(30 * var(--u));
    padding: calc(2 * var(--u)) calc(9 * var(--u));
  }
</style>
