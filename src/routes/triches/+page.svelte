<script lang="ts">
  // Triches et mods d'un jeu (lot 8). Décision de Seb (02/10) : Frogtend ne télécharge pas lui-même les bases de
  // triche ni les listes de mods ; Firehouse les fournit, jeu par jeu (contrat 13). Frogtend pose un fichier de codes
  // dans le dossier du profil de l'émulateur, sur demande ; les mods et tables Cheat Engine s'ouvrent dans le
  // navigateur, avec une copie du jeu PROPOSÉE avant de modder.
  import { page } from '$app/state';
  import { api, taille, type TrichesJeu } from '$lib/api';
  import { pourLeJeu } from '$lib/emulateurs/choix';
  import { installerParFirehouse } from '$lib/emulateurs/assistant.svelte';
  import { confirmer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { etat } from '$lib/etat.svelte';
  import { tele } from '$lib/ludotheque/telechargements.svelte';

  const id = $derived(Number(page.url.searchParams.get('jeu')));
  const jeu = $derived(tele.jeux[id]);
  let t = $state<TrichesJeu | null>(null);
  let erreur = $state<string | null>(null);

  $effect(() => {
    const n = id;
    t = null;
    erreur = null;
    api.jeuTriches(n).then((r) => (t = r)).catch((e) => (erreur = motifDuRefus(e)));
  });

  /** L'émulateur de ce jeu sur ce PC (et son identifiant Frogtend, pour ne montrer que ses codes). */
  const emulateur = $derived(jeu ? pourLeJeu(etat.pc.emulateurs, etat.pc.emulateursJeux, jeu.plateforme, id) : null);
  let idEmulateur = $state<string | null>(null);
  $effect(() => {
    const prog = emulateur?.programme?.toLowerCase();
    idEmulateur = null;
    if (prog) api.emulateursInstalles().then((l) => (idEmulateur = l.find((e) => e.programme.toLowerCase() === prog)?.id ?? null)).catch(() => {});
  });
  /** Cheat Engine sur ce PC (fourni par Firehouse, id « cheatengine »). */
  let cheatEngine = $state<string | null>(null);
  async function chercherCheatEngine() {
    const l = await api.emulateursInstalles().catch(() => []);
    cheatEngine = l.find((e) => e.id === 'cheatengine')?.programme ?? null;
  }
  $effect(() => {
    chercherCheatEngine();
  });

  async function lancerCheatEngine() {
    if (!cheatEngine) {
      const e = await installerParFirehouse('cheatengine', 'Cheat Engine');
      if (!e) return;
      cheatEngine = e.programme;
    }
    try {
      await api.cheatengineLancer(cheatEngine);
      toast('🧰 Cheat Engine démarre, avec tes réglages à toi. Ils seront rangés dans ton profil à sa fermeture.');
    } catch (e) {
      toast(`Impossible de lancer Cheat Engine : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  const codes = $derived((t?.codes ?? []).filter((c) => !idEmulateur || c.emulateur === idEmulateur));

  async function poser(c: TrichesJeu['codes'][number]) {
    if (!emulateur) return;
    const oui = await confirmer(`🎯 Ajouter « ${c.titre} » ?`, {
      message: [
        `${c.nb_codes ?? '?'} code(s), format ${c.format}${c.source ? `, source : ${c.source}` : ''}.`,
        c.confiance !== undefined && c.confiance < 1
          ? `⚠ Trouvé par ${c.correspondance === 'titre' ? 'le titre' : c.correspondance} (${Math.round(c.confiance * 100)} %) : les codes peuvent viser une autre région du jeu. Le fichier garde le nom de la base : charge-le toi-même dans le menu de l’émulateur.`
          : '',
        `Le fichier ira dans le dossier de TON profil pour ${emulateur.nom || 'l’émulateur'} ; un fichier différent déjà là est gardé à côté.`,
        'Les codes s’activent ensuite dans le menu de l’émulateur.',
      ]
        .filter(Boolean)
        .join('\n'),
      libelleValider: '🎯 Ajouter',
    });
    if (!oui) return;
    try {
      const ou = await api.tricheInstaller(id, c.cle, emulateur.programme, emulateur.ligne);
      toast(`✅ Codes ajoutés : ${ou}`);
    } catch (e) {
      toast(`Impossible : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  /** Avant d'aller chercher un mod : proposer une copie du jeu (pas obligatoire, avec un avertissement). */
  async function avantDeModder(pageMod: string | undefined) {
    if (!pageMod) return;
    if (jeu?.installation) {
      const octets = await api.jeuTailleInstallation(id).catch(() => null);
      const copier = await confirmer('💾 Copier le jeu avant de le modifier ?', {
        message: `Conseillé : un mod peut abîmer le jeu. La copie prend ${octets !== null ? taille(octets) : 'la taille du jeu'} à côté de son dossier.`,
        libelleValider: '💾 Copier d’abord',
      });
      if (copier) {
        try {
          const c = await api.jeuCopieAvantMod(id);
          toast(`✅ Copie faite : ${c}`);
        } catch (e) {
          toast(`La copie a échoué : ${motifDuRefus(e)}`, 'erreur');
          return;
        }
      } else {
        const quand_meme = await confirmer('⚠ Sans copie', {
          message: 'Si le mod abîme le jeu, il faudra le réinstaller (tes parties restent sauvegardées).',
          libelleValider: 'Continuer sans copie',
          danger: true,
        });
        if (!quand_meme) return;
      }
    }
    await ouvrirPage(pageMod);
  }

  async function ouvrirPage(u: string | undefined) {
    if (!u || !/^https?:\/\//.test(u)) return;
    try {
      const { openUrl } = await import('@tauri-apps/plugin-opener');
      await openUrl(u);
    } catch (e) {
      toast(`Impossible d’ouvrir la page : ${motifDuRefus(e)}`, 'erreur');
    }
  }
</script>

<div class="triches">
  <header>
    <h1>🎯 Triches et mods{jeu ? ` — ${jeu.titre}` : ''}</h1>
    <p class="muted">Fournis par Firehouse, jeu par jeu. Rien n’est installé sans ton accord.</p>
  </header>

  {#if erreur}
    <p class="muted">⚠ {erreur}</p>
  {:else if !t}
    <p class="muted">Chargement…</p>
  {:else if !t.codes.length && !t.cheat_engine.length && !t.mods.length && !t.page_mods}
    <p class="muted">Firehouse ne connaît encore ni codes ni mods pour ce jeu.</p>
    {#if t.note}<p class="muted">ℹ {t.note}</p>{/if}
    <button class="btn petit" onclick={lancerCheatEngine}>{cheatEngine ? '🧰 Lancer Cheat Engine' : '⬇ Installer Cheat Engine'}</button>
  {:else}
    {#if t.note}<p class="muted">ℹ {t.note}</p>{/if}
    <section class="cx-block">
      <h2>Codes de triche{emulateur ? ` (${emulateur.nom || 'émulateur'})` : ''}</h2>
      {#if !emulateur}
        <p class="muted">Ce jeu ne passe pas par un émulateur : pas de codes ici (voir Cheat Engine).</p>
      {:else if !codes.length}
        <p class="muted">Aucun code pour cet émulateur.</p>
      {:else}
        <ul>
          {#each codes as c (c.cle)}
            <li>
              <span><strong>{c.titre}</strong> <span class="muted">{c.nb_codes ?? '?'} code(s) · {c.source ?? ''}</span></span>
              <button class="btn petit" onclick={() => poser(c)}>🎯 Ajouter</button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="cx-block">
      <h2>Cheat Engine</h2>
      <p class="muted">Pour les jeux PC : modifier des valeurs en mémoire. Fourni par Firehouse, sans logiciels en plus ; chaque profil garde ses réglages.</p>
      <button class="btn petit" onclick={lancerCheatEngine}>{cheatEngine ? '🧰 Lancer Cheat Engine' : '⬇ Installer Cheat Engine'}</button>
    </section>

    {#if t.cheat_engine.length}
      <section class="cx-block">
        <h2>Tables Cheat Engine</h2>
        <ul>
          {#each t.cheat_engine as ce, i (i)}
            <li>
              <span><strong>{ce.titre}</strong> <span class="muted">{ce.version_jeu ?? ''} · {ce.source ?? ''}</span></span>
              <button class="btn petit" onclick={() => ouvrirPage(ce.page)} disabled={!ce.page}>🌐 Voir</button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if t.mods.length || t.page_mods}
      <section class="cx-block">
        <h2>Mods</h2>
        {#if t.page_mods}
          <p><button class="btn petit" onclick={() => avantDeModder(t?.page_mods ?? undefined)}>🌐 Page des mods et correctifs (PCGamingWiki…)</button></p>
        {/if}
        <ul>
          {#each t.mods as m, i (i)}
            <li>
              <span>
                <strong>{m.nom}</strong>
                <span class="muted">{m.version ?? ''}{m.auteur ? ` · ${m.auteur}` : ''} · {m.source ?? ''}</span>
                {#if m.description}<br /><span class="muted">{m.description}</span>{/if}
              </span>
              <button class="btn petit" onclick={() => avantDeModder(m.page)} disabled={!m.page}>🌐 Voir le mod</button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  {/if}
  <button class="btn retour" onclick={() => history.back()}>← Retour</button>
</div>

<style>
  .triches {
    display: grid;
    gap: calc(14 * var(--u));
    padding: calc(20 * var(--u));
    max-width: calc(900 * var(--u));
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
    align-content: start;
    overflow: auto;
    height: 100%;
  }
  h1,
  h2 {
    margin: 0;
  }
  header p {
    margin: calc(4 * var(--u)) 0 0;
  }
  ul {
    list-style: none;
    margin: calc(8 * var(--u)) 0 0;
    padding: 0;
    display: grid;
    gap: calc(8 * var(--u));
  }
  li {
    display: flex;
    gap: calc(10 * var(--u));
    align-items: center;
    justify-content: space-between;
  }
  .retour {
    justify-self: start;
  }
</style>
