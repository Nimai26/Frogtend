<script lang="ts">
  // 📁 Où sont mes parties (lot S1, 22/11) : les emplacements de parties d'un jeu PC sur CE PC, d'après Firehouse,
  // rangés pour CE profil. Le chemin résolu n'apparaît qu'ici, à l'écran, pour la personne du profil (jamais au journal).
  // `canape` (Taodbox) : lisible à la manette (pas de bouton « Ouvrir le dossier » : l'Explorateur se pilote à la
  // souris) ; chaque ligne se sélectionne, pour que la croix fasse défiler la liste.
  // Lot S2 : « 🛟 Mettre mes parties à l'abri » (au bureau ET au canapé : la confirmation est une fenêtre maison, A
  // valide, B annule), annoncé avant (nombre de fichiers, taille, profil propriétaire), puis la liste des abris du
  // profil. Rien n'est envoyé chez Firehouse ni remis ici.
  // Lot S3 : « 🛟 Dernier abri : … », la liste RELUE à chaque abri automatique (événement), une ligne qui reste quand
  // le dernier abri automatique n'a pas pu tout copier, et une liste longue repliée (5 plus récents + le dernier de
  // chaque « avant … »).
  // Lot S5 : « 📥 Reprendre mes parties de Firehouse » (bureau ET canapé) : seulement les parties de CE jeu, annoncées
  // (fichiers, Mo), jamais par-dessus une partie différente (fenêtre de conflit maison, pilotable à la manette).
  import { api, type AbriVu, type PartiesVues } from '$lib/api';
  import { listen } from '@tauri-apps/api/event';
  import type { ProgresAbri } from '$lib/api';
  import { choisir, confirmer, fermerToast, informer, majProgres, toast, toastProgres } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { tick } from 'svelte';
  import { abrisAuto, chargerLigneGardee, signalerAbri } from '$lib/parties/abriAuto.svelte';
  import { selectionner } from '$lib/taodbox/manette.svelte';
  import { reprendreMesParties, reprise } from '$lib/parties/reprise.svelte';
  import {
    abrisAMontrer,
    abritable,
    contenu,
    ligneAbriAuto,
    libelleOrigine,
    messageAnnonce,
    messageChoix,
    messageDeja,
    messageRien,
    montres,
    noteAbri,
    optionsAbri,
    ouvrable,
    texteAutresEndroits,
    texteContenuAbri,
    texteDate,
    texteDernierAbri,
    texteEtat,
    texteFait,
    texteFiltre,
    texteNuage,
    texteSorteAbri,
    texteSource,
    texteTotalAbris,
    titreOrigine,
  } from './parties';

  let { id, canape = false }: { id: number; canape?: boolean } = $props();
  let vues = $state<PartiesVues | undefined>(undefined);
  let erreur = $state<string | null>(null);
  /** Le dossier en cours d'ouverture (gardien : un seul à la fois). */
  let ouverture = $state<string | null>(null);
  /** Les abris de ce jeu pour le profil ouvert (`undefined` : pas encore lus, ou illisibles). */
  let abris = $state<AbriVu[] | undefined>(undefined);
  /** Où en est la mise à l'abri (gardien : une seule à la fois) : `compte` pendant l'annonce (rien n'est copié avant le
   * oui), `copie` après. */
  let etape = $state<null | 'compte' | 'copie'>(null);
  /** La liste longue dépliée (« Voir les N plus anciens »). */
  let tous = $state(false);
  let sectionAbris = $state<HTMLElement | null>(null);
  const vus = $derived(abris ? abrisAMontrer(abris, tous) : { montres: [], caches: 0 });
  /** Le dernier abri automatique de CE jeu, s'il n'a pas pu tout copier (la ligne qui reste). */
  const ligneAuto = $derived(ligneAbriAuto(abrisAuto.derniers[id]));
  const dernier = $derived(texteDernierAbri(abris));
  const total = $derived(texteTotalAbris(abris));

  $effect(() => {
    const n = id;
    vues = undefined;
    erreur = null;
    abris = undefined;
    api
      .partiesEmplacements(n)
      .then((v) => {
        if (n === id) vues = v;
      })
      .catch((e) => {
        if (n === id) erreur = motifDuRefus(e);
      });
    tous = false;
    void relireAbris(n);
    // M6 : la ligne d'un abri automatique raté, gardée par le cœur (elle survit au redémarrage).
    void chargerLigneGardee(n);
  });

  // Un abri automatique vient d'être fait (ou n'a pas pu l'être) : la liste est relue.
  let versionVue = abrisAuto.version;
  $effect(() => {
    const v = abrisAuto.version;
    if (v === versionVue) return;
    versionVue = v;
    void relireAbris(id);
  });

  /** « Voir les N plus anciens » : le bouton disparaît, le focus va sur le premier abri nouvellement montré (jamais un
   * focus perdu, au clavier comme à la manette). */
  async function voirTous() {
    const avant = new Set(vus.montres);
    const i = (abris ?? []).findIndex((a) => !avant.has(a));
    tous = true;
    await tick();
    const li = sectionAbris?.querySelectorAll<HTMLElement>('li')[Math.max(i, 0)];
    if (!li) return;
    if (canape) selectionner(li);
    else li.focus();
  }

  async function relireAbris(n: number) {
    try {
      const l = await api.partiesAbris(n);
      if (n === id) abris = l;
    } catch {
      // La liste des abris est un plus : son échec ne cache jamais les emplacements.
      if (n === id) abris = undefined;
    }
  }

  async function ouvrir(dossier: string) {
    if (ouverture !== null) return;
    ouverture = dossier;
    try {
      await api.partiesOuvrirDossier(id, dossier);
    } catch (e) {
      toast(`Impossible d’ouvrir le dossier : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      ouverture = null;
    }
  }

  /** 📥 Reprendre mes parties de Firehouse (lot S5) ; ensuite, les emplacements et les abris sont relus. */
  async function reprendre() {
    const n = id;
    if (!(await reprendreMesParties(n))) return;
    if (n !== id) return;
    void relireAbris(n);
    api
      .partiesEmplacements(n)
      .then((v) => {
        if (n === id) vues = v;
      })
      .catch(() => {});
  }

  /** Le cœur refuse une copie devenue très grosse depuis l'annonce : elle est annoncée de nouveau (jamais copiée sans
   * le dire). */
  function aConfirmer(e: unknown): boolean {
    return /confirme d.abord/.test(motifDuRefus(e));
  }

  /** Annonce (nombre de fichiers, taille, profil), confirmation maison, puis la copie ; le cœur refait tout le plan. */
  async function mettreALAbri() {
    // Lot S5 : jamais pendant une reprise de Firehouse (et la reprise attend la fin de l'abri).
    if (etape !== null || reprise.enCours !== null) return;
    const n = id;
    reprise.abriEnCours = n;
    try {
      for (let essai = 0; essai < 2; essai++) {
        etape = 'compte';
        const a = await api.partiesAbriApercu(n);
        if (a.deja) {
          const reste = messageDeja(a);
          if (reste) await informer('✅ Déjà à l’abri', reste);
          else toast('✅ Déjà à l’abri : rien n’a changé depuis ton dernier abri.', 'ok');
          return;
        }
        // Des fichiers seulement dans OneDrive (Seb, 22/11) : proposer de les télécharger d'abord — jamais sans ce
        // choix (« Annuler » est toujours là).
        let rapatrier = false;
        if (a.en_ligne_seulement > 0) {
          const c = await choisir('🛟 Mettre mes parties à l’abri ?', optionsAbri(a), messageChoix(a));
          if (!c) return;
          rapatrier = c === 'telecharger';
        } else if (a.fichiers === 0) {
          await informer('🛟 Rien à mettre à l’abri', messageRien(a));
          return;
        } else if (!(await confirmer('🛟 Mettre mes parties à l’abri ?', { message: messageAnnonce(a), libelleValider: 'Mettre à l’abri' }))) {
          return;
        }
        etape = 'copie';
        // Visible jusqu'à la fin, même si on quitte la page (au canapé aussi).
        const attente = toastProgres(rapatrier ? '☁ Téléchargement depuis OneDrive…' : '🛟 Copie de tes parties en cours…');
        let arreter: (() => void) | null = null;
        try {
          if (rapatrier) {
            arreter = await listen<ProgresAbri>('parties-abri-progres', (e) => {
              const { jeu, faits, total } = e.payload;
              if (jeu !== n) return;
              majProgres(attente, { etape: `Téléchargement depuis OneDrive… ${faits}/${total}`, chiffres: '', pourcent: total ? Math.round((100 * faits) / total) : 0 });
            });
          }
          const r = await api.partiesAbriter(n, rapatrier ? a.gros_avec_nuage : a.gros, rapatrier);
          toast(texteFait(r), r.echecs.length || r.en_ligne_seulement ? 'alerte' : 'ok');
          // Fait à la main : la ligne d'un abri automatique raté ne vaut plus.
          if (r.date && !r.echecs.length && !r.en_ligne_seulement) signalerAbri(n);
          await relireAbris(n);
          return;
        } catch (e) {
          if (essai === 0 && aConfirmer(e)) continue;
          throw e;
        } finally {
          arreter?.();
          fermerToast(attente);
        }
      }
    } catch (e) {
      toast(`Échec de la mise à l’abri : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      etape = null;
      reprise.abriEnCours = null;
    }
  }
</script>

<div class="parties" class:canape>
  <!-- Seule la ligne d'état est annoncée par un lecteur d'écran (jamais toute la liste à chaque changement). -->
  <div class="etat" aria-live="polite">
    {#if erreur}
      <p class="muted">⚠ {erreur}</p>
    {:else if vues === undefined}
      <p class="muted">⏳ Frogtend cherche où sont tes parties…</p>
    {:else}
      {#if texteSource(vues)}<p class="tag warn">{texteSource(vues)}</p>{/if}
      {#if texteEtat(vues)}<p class="muted">{texteEtat(vues)}</p>{/if}
    {/if}
    {#if ligneAuto}<p class="tag warn ligne-auto">{ligneAuto}</p>{/if}
  </div>
  {#if vues && montres(vues).length}
    <ul class="liste">
      {#each montres(vues) as e, i (i)}
        {@const details = [libelleOrigine(e.origine, e.moteur), texteFiltre(e), texteNuage(e)].filter(Boolean).join(' · ')}
        {#if canape}
          <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
          <li class="emplacement" tabindex="0">
            <span class="dossier">{e.dossier}</span>
            <span class="contenu" class:ok={ouvrable(e) && e.fichiers > 0}>{contenu(e)}</span>
            <span class="muted detail" title={titreOrigine(e.origine, e.source)}>{details}</span>
          </li>
        {:else}
          <li class="emplacement">
            <span class="dossier">{e.dossier}</span>
            <span class="contenu" class:ok={ouvrable(e) && e.fichiers > 0}>{contenu(e)}</span>
            <span class="muted detail" title={titreOrigine(e.origine, e.source)}>{details}</span>
            {#if e.dossier && ouvrable(e)}
              {@const d = e.dossier}
              <button class="btn" aria-disabled={ouverture !== null} onclick={() => ouvrir(d)}>
                {ouverture === d ? '⏳ Ouverture…' : '📂 Ouvrir le dossier'}
              </button>
            {/if}
          </li>
        {/if}
      {/each}
    </ul>
  {/if}
  {#if vues && texteAutresEndroits(vues)}<p class="muted">{texteAutresEndroits(vues)}</p>{/if}
  {#if vues}
    <div class="actions">
      {#if abritable(vues)}
        <button class="btn primary" aria-disabled={etape !== null || reprise.enCours !== null} onclick={mettreALAbri}>
          {etape === 'compte' ? '⏳ Frogtend compte tes parties…' : etape === 'copie' ? '⏳ Copie en cours…' : '🛟 Mettre mes parties à l’abri'}
        </button>
      {/if}
      <button class="btn" aria-disabled={reprise.enCours !== null || etape !== null} onclick={reprendre}>
        {reprise.enCours === id ? '⏳ Reprise de tes parties…' : '📥 Reprendre mes parties de Firehouse'}
      </button>
    </div>
  {/if}
  {#if abris && abris.length}
    <section class="abris" aria-label="Mes abris de parties" bind:this={sectionAbris}>
      <h3>🛟 Mes abris</h3>
      {#if dernier}<p class="dernier">{dernier}</p>{/if}
      {#if total}<p class="muted">{total}</p>{/if}
      <ul class="liste">
        {#each vus.montres as a, i (a.date + a.sorte + i)}
          {@const note = noteAbri(a)}
          {#if canape}
            <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
            <li class="emplacement abri" tabindex="0">
              <span class="contenu">{texteDate(a.date)}</span>
              <span>{texteContenuAbri(a)}</span>
              <span class="muted detail">{texteSorteAbri(a)}</span>
              {#if note}<span class="muted detail">{note}</span>{/if}
            </li>
          {:else}
            <li class="emplacement abri" tabindex="-1">
              <span class="contenu">{texteDate(a.date)}</span>
              <span>{texteContenuAbri(a)}</span>
              <span class="muted detail">{texteSorteAbri(a)}</span>
              {#if note}<span class="muted detail">{note}</span>{/if}
            </li>
          {/if}
        {/each}
      </ul>
      {#if vus.caches > 0}
        <div class="actions">
          <button class="btn" onclick={voirTous}>Voir les {vus.caches.toLocaleString('fr-FR')} plus ancien{vus.caches > 1 ? 's' : ''}</button>
        </div>
      {/if}
    </section>
  {/if}
</div>

<style>
  .parties {
    display: grid;
    gap: calc(8 * var(--u));
  }
  .parties p {
    margin: 0;
  }
  .etat {
    display: grid;
    gap: calc(6 * var(--u));
  }
  .liste {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: calc(8 * var(--u));
  }
  .emplacement {
    display: grid;
    gap: calc(4 * var(--u));
    justify-items: start;
    padding: calc(10 * var(--u)) calc(12 * var(--u));
    border: 1px solid var(--line);
    border-radius: calc(10 * var(--u));
    background: var(--bg2);
  }
  .emplacement:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .dossier {
    font-family: var(--mono);
    font-size: calc(12.5 * var(--u));
    overflow-wrap: anywhere;
  }
  .contenu {
    font-weight: 600;
  }
  .contenu.ok {
    color: var(--ok-texte);
  }
  .detail {
    overflow-wrap: anywhere;
  }
  .actions {
    display: flex;
    gap: calc(8 * var(--u));
    flex-wrap: wrap;
  }
  .abris {
    display: grid;
    gap: calc(8 * var(--u));
  }
  .dernier {
    font-weight: 600;
  }
  .abris h3 {
    margin: calc(6 * var(--u)) 0 0;
    font-size: calc(15 * var(--u));
  }
  /* À 3 m de la télé : un peu plus grand. */
  .canape .dossier,
  .canape .contenu,
  .canape .detail,
  .canape .abri span {
    font-size: calc(15 * var(--u));
  }
  /* Un bouton « aria-disabled » reste focalisable (il dit qu'il attend) mais se montre éteint. */
  button[aria-disabled='true'] {
    opacity: 0.6;
    cursor: progress;
  }
</style>
