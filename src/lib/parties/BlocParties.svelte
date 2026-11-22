<script lang="ts">
  // 📁 Où sont mes parties (lot S1, 22/11) : les emplacements de parties d'un jeu PC sur CE PC, d'après Firehouse,
  // rangés pour CE profil. Le chemin résolu n'apparaît qu'ici, à l'écran, pour la personne du profil (jamais au journal).
  // `canape` (Taodbox) : lisible à la manette (pas de bouton « Ouvrir le dossier » : l'Explorateur se pilote à la
  // souris) ; chaque ligne se sélectionne, pour que la croix fasse défiler la liste.
  // Lot S2 : « 🛟 Mettre mes parties à l'abri » (au bureau ET au canapé : la confirmation est une fenêtre maison, A
  // valide, B annule), annoncé avant (nombre de fichiers, taille, profil propriétaire), puis la liste des abris du
  // profil. Rien n'est envoyé chez Firehouse ni remis ici.
  import { api, type AbriVu, type PartiesVues } from '$lib/api';
  import { confirmer, fermerToast, informer, toast, toastProgres } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import {
    abritable,
    contenu,
    libelleOrigine,
    messageAnnonce,
    messageDeja,
    messageRien,
    montres,
    noteAbri,
    ouvrable,
    texteAutresEndroits,
    texteContenuAbri,
    texteDate,
    texteEtat,
    texteFait,
    texteFiltre,
    texteNuage,
    texteSorteAbri,
    texteSource,
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
    void relireAbris(n);
  });

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

  /** Le cœur refuse une copie devenue très grosse depuis l'annonce : elle est annoncée de nouveau (jamais copiée sans
   * le dire). */
  function aConfirmer(e: unknown): boolean {
    return /confirme d.abord/.test(motifDuRefus(e));
  }

  /** Annonce (nombre de fichiers, taille, profil), confirmation maison, puis la copie ; le cœur refait tout le plan. */
  async function mettreALAbri() {
    if (etape !== null) return;
    const n = id;
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
        if (a.fichiers === 0) {
          await informer('🛟 Rien à mettre à l’abri', messageRien(a));
          return;
        }
        const oui = await confirmer('🛟 Mettre mes parties à l’abri ?', { message: messageAnnonce(a), libelleValider: 'Mettre à l’abri' });
        if (!oui) return;
        etape = 'copie';
        // Visible jusqu'à la fin, même si on quitte la page (au canapé aussi).
        const attente = toastProgres('🛟 Copie de tes parties en cours…');
        try {
          const r = await api.partiesAbriter(n, a.gros);
          toast(texteFait(r), r.echecs.length || r.en_ligne_seulement ? 'alerte' : 'ok');
          await relireAbris(n);
          return;
        } catch (e) {
          if (essai === 0 && aConfirmer(e)) continue;
          throw e;
        } finally {
          fermerToast(attente);
        }
      }
    } catch (e) {
      toast(`Échec de la mise à l’abri : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      etape = null;
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
  </div>
  {#if vues && montres(vues).length}
    <ul class="liste">
      {#each montres(vues) as e, i (i)}
        {@const details = [libelleOrigine(e.origine), texteFiltre(e), texteNuage(e)].filter(Boolean).join(' · ')}
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
  {#if vues && abritable(vues)}
    <div class="actions">
      <button class="btn primary" aria-disabled={etape !== null} onclick={mettreALAbri}>
        {etape === 'compte' ? '⏳ Frogtend compte tes parties…' : etape === 'copie' ? '⏳ Copie en cours…' : '🛟 Mettre mes parties à l’abri'}
      </button>
    </div>
  {/if}
  {#if abris && abris.length}
    <section class="abris" aria-label="Mes abris de parties">
      <h3>🛟 Mes abris</h3>
      <ul class="liste">
        {#each abris as a, i (a.date + a.sorte + i)}
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
            <li class="emplacement abri">
              <span class="contenu">{texteDate(a.date)}</span>
              <span>{texteContenuAbri(a)}</span>
              <span class="muted detail">{texteSorteAbri(a)}</span>
              {#if note}<span class="muted detail">{note}</span>{/if}
            </li>
          {/if}
        {/each}
      </ul>
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
