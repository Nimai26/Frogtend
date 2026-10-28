<script lang="ts">
  // La fiche d'un jeu, en deux niveaux (décision de Seb) :
  //  - le JEU : résumé, genres, informations, annexes — vrai quelle que soit la version ;
  //  - les VERSIONS : chacune avec SES fichiers et SES notes, qui ne valent pas pour une autre.
  import { tick } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { api, estErreurCoeur, taille, type Annexe, type DocumentJeu, type Fiche } from '$lib/api';
  import { confirmer, informer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import Jaquette from '$lib/ludotheque/Jaquette.svelte';
  import LecteurLivret from '$lib/ludotheque/LecteurLivret.svelte';
  import LecteurMedia from '$lib/ludotheque/LecteurMedia.svelte';
  import VisionneuseImages from '$lib/ludotheque/VisionneuseImages.svelte';
  import { grosPixels, largeurUtile, libelleMedia, ordonner, type MediaJeu } from '$lib/ludotheque/galerie';
  import { adresseMedia } from '$lib/api';
  import { documentsMontres, libelleDocument, ouvrirDocument, texteChargement, type DocumentOuvert } from '$lib/ludotheque/documents';
  import { arreterDocument, suiviDocument } from '$lib/ludotheque/documentsProgres.svelte';
  import { ageConseille, dateDeSortie, datePrecise, noteSurCinq, typeDeSortie, type InfosJeu } from '$lib/ludotheque/infos';
  import { ajouterUneVersion, jeuSurPc, mettreDansLaLudotheque, rechargerJeuxDuPc, reprendre, retirerUneVersion, tele } from '$lib/ludotheque/telechargements.svelte';
  import type { AvisVersion } from '$lib/ludotheque/versions';
  import { aCartes, cartesSurPc } from '$lib/ludotheque/cartes';
  import { nomAffiche } from '$lib/ludotheque/ludotheque.svelte';
  import { emulateursDuJeu, gererJeu, installer, jouer, jouerAvec, partie } from '$lib/ludotheque/jeu.svelte';
  import { libellePreparation } from '$lib/emulateurs/pret.svelte';

  let fiche = $state<Fiche | null>(null);
  // Les infos du jeu (contrat 1.15) : celles de la fiche lue en ligne, ou relues en direct pour un jeu du PC.
  let infos = $state<InfosJeu | null>(null);
  let horsLigne = $state(false);
  let locale = $state(false);
  let erreur = $state('');
  let annexe = $state<{ titre: string; texte: string } | null>(null);
  // Les documents du jeu (contrat 1.7 : livret…), relus à chaque ouverture de la fiche ; le livret ouvert.
  let documents = $state<DocumentJeu[]>([]);
  let ouverture = $state<number | null>(null);
  let ouvert = $state<DocumentOuvert | null>(null);
  // La galerie du jeu (lot Médias M2) : ses visuels, l'image ouverte en grand, les vignettes qui ne s'affichent pas.
  let medias = $state<MediaJeu[]>([]);
  let imageOuverte = $state<number | null>(null);
  let ratees = $state<number[]>([]);
  const boutonsMedias: Record<number, HTMLButtonElement> = {};
  /** La largeur d'une vignette (cellule fixe) : l'image s'y loge en entier. */
  const VIGNETTE = 160;
  // En pixels réels : l'échelle du texte (réglage) et la densité de l'écran comptent.
  const pixelsEcran = () =>
    VIGNETTE * (Number(getComputedStyle(document.documentElement).getPropertyValue('--echelle')) || 1) * (window.devicePixelRatio || 1);
  // Les boutons des documents, pour y rendre la sélection quand le lecteur se ferme.
  const boutonsDocuments: Record<number, HTMLButtonElement> = {};

  const id = $derived(Number(page.params.id));

  async function charger(n: number) {
    complements = false;
    fiche = null;
    infos = null;
    erreur = '';
    annexe = null;
    documents = [];
    ouvert = null;
    medias = [];
    ratees = [];
    imageOuverte = null;
    try {
      const r = await api.fiche(n);
      // Une fiche réunie à une autre chez Firehouse (contrat 1.14) : la nouvelle, à SON adresse.
      if (r.reunie_dans && r.reunie_dans !== n) {
        toast('Ce jeu a été réuni à une autre fiche chez Firehouse : voici la nouvelle.');
        await goto(`/jeu/${r.reunie_dans}`, { replaceState: true });
        return;
      }
      fiche = r.fiche;
      infos = r.fiche.infos ?? null;
      horsLigne = r.hors_ligne;
      locale = r.locale;
    } catch (e) {
      erreur = estErreurCoeur(e) ? e.motif : motifDuRefus(e);
      complements = true;
      return;
    }
    // Les documents sont un plus : s'ils ne viennent pas, la fiche reste complète sans eux. Une autre fiche ouverte
    // entre-temps ne reçoit jamais ceux-ci.
    // La galerie aussi est un plus : pas de section si elle ne vient pas (ou si le jeu n'a aucune image). Les deux
    // listes arrivent ensemble.
    // Un jeu du PC : sa fiche vient du disque (gardée à l'installation) ; ses infos sont relues en direct (Firehouse :
    // elles peuvent changer sans que le jeu revienne par la synchronisation). Les autres les portent déjà.
    const [l, g, i] = await Promise.all([
      api.documents(n).catch(() => []),
      api.medias(n).catch(() => []),
      locale ? api.infosJeu(n).catch(() => null) : Promise.resolve(null),
    ]);
    if (n === id) {
      documents = documentsMontres(l);
      medias = ordonner(g);
      // Le cœur a tranché (caché, réuni, sans infos : rien) : jamais les anciennes infos du disque à la place.
      if (locale) infos = i;
      complements = true;
    }
  }

  /** Ouvre un document : téléchargé d'abord s'il n'est pas sur le PC (sa taille est sur le bouton : le clic vaut accord). */
  /** « Arrêter » cliqué pendant le téléchargement : l'arrêt voulu n'est pas une erreur. */
  let arretDemande = false;
  let boutonArret = $state<HTMLButtonElement | null>(null);
  function arreterDoc(jeu: number, doc: number) {
    arretDemande = true;
    arreterDocument(jeu, doc);
  }

  async function ouvrir(d: DocumentJeu) {
    if (!fiche || ouverture !== null) return;
    const jeu = fiche.id;
    ouverture = d.id;
    arretDemande = false;
    // Le bouton du document est désactivé pendant le téléchargement : la sélection va sur « Arrêter » (manette).
    if (!d.garde) {
      await tick();
      boutonArret?.focus();
    }
    let lecteur = false;
    try {
      const o = await ouvrirDocument(jeu, d);
      if (fiche?.id !== jeu) return; // une autre fiche ouverte pendant le téléchargement
      ouvert = o;
      lecteur = true;
      documents = documents.map((x) => (x.id === d.id ? { ...x, garde: true } : x));
    } catch (e) {
      if (arretDemande) toast('⏹ Téléchargement arrêté : il reprendra où il en était.', 'neutre');
      else toast(`Impossible d’ouvrir ce document : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      ouverture = null;
      // Rien d'ouvert (arrêt, erreur) : la sélection revient sur le bouton du document.
      if (!lecteur && fiche?.id === jeu) {
        await tick();
        boutonsDocuments[d.id]?.focus();
      }
    }
  }
  $effect(() => {
    charger(id);
  });

  /** La visionneuse se ferme : la sélection revient sur la vignette de la dernière image vue. */
  async function fermerImage(media: number) {
    imageOuverte = null;
    await tick();
    boutonsMedias[media]?.focus();
  }

  /** Le lecteur se ferme : la sélection revient sur le bouton du document. */
  async function fermer() {
    const doc = ouvert?.doc;
    ouvert = null;
    await tick();
    if (doc) boutonsDocuments[doc]?.focus();
  }

  /** Libellé lisible de la qualité (la nature d'une version, qui décide de l'installation). */
  function nature(q: string): { libelle: string; ton: string } {
    const t = q.toLowerCase();
    if (t.includes('prêt à jouer')) return { libelle: '✅ Prêt à jouer', ton: 'ok' };
    if (t.includes('rom')) return { libelle: '💾 ROM', ton: '' };
    if (t.includes('image disque')) return { libelle: '💿 Image disque', ton: '' };
    if (t.includes('repack')) return { libelle: '📦 Repack (installeur)', ton: 'warn' };
    if (t.includes('installeur')) return { libelle: '🧰 Installeur d’origine', ton: 'warn' };
    return { libelle: `❔ ${q || 'Nature non dite'}`, ton: '' };
  }

  async function ouvrirAnnexe(a: Annexe) {
    if (!fiche) return;
    if (!a.texte) {
      if (!locale) {
        await informer(
          `📎 ${a.titre}`,
          `Ce document (${taille(a.taille)}) arrive sur le PC quand tu mets le jeu dans ta ludothèque.`,
        );
        return;
      }
      try {
        await api.ouvrirAnnexe(fiche.id, a.i);
      } catch (e) {
        toast(`Impossible d’ouvrir « ${a.titre} » : ${motifDuRefus(e)}`, 'erreur');
      }
      return;
    }
    try {
      const r = await api.annexeTexte(fiche.id, a.i, a.cle);
      annexe = { titre: r.titre ?? a.titre, texte: r.texte ?? '' };
    } catch (e) {
      if (estErreurCoeur(e) && e.sorte === 'conflit') {
        toast('⚠ La fiche a changé entre-temps : je la relis.', 'alerte');
        await charger(fiche.id);
      } else {
        toast(`Impossible d’ouvrir « ${a.titre} » : ${motifDuRefus(e)}`, 'erreur');
      }
    }
  }

  const ICONES_ANNEXES: Record<string, string> = {
    manuel: '📘',
    solution: '🧭',
    astuces: '💡',
    lancement: '▶',
    presse: '📰',
  };

  /** Markdown simple → paragraphes et gras, sans HTML venu du serveur (tout est échappé). */
  function paragraphes(texte: string): { gras: boolean; t: string }[][] {
    return texte
      .split(/\n{2,}/)
      .map((p) => p.split(/(\*\*[^*]+\*\*)/).filter(Boolean).map((m) =>
        m.startsWith('**') ? { gras: true, t: m.slice(2, -2) } : { gras: false, t: m.replace(/`/g, '') },
      ));
  }

  /** Les versions AJOUTÉES du jeu sur ce PC (leur état) et l'avis du cœur sur chaque version (cochable ou pas). */
  let ajoutees = $state<{ version: number; etat: 'attente' | 'en_cours' | 'pause' | 'telecharge' | 'erreur'; message: string | null }[]>([]);
  let avis = $state<AvisVersion[]>([]);
  const ETATS_VERSION = { attente: '⏳ En attente', en_cours: '⬇ En cours d’arrivée', pause: '⏸ En pause', telecharge: '✅ Reçue', erreur: '⚠ Pas arrivée' } as const;
  $effect(() => {
    const j = fiche ? jeuSurPc(fiche.id) : undefined;
    // Relu à chaque changement du jeu sur le PC (événements du téléchargement).
    // (Et quand une version change d'état : `tele.versions` du jeu.)
    void (j && tele.versions[j.id] && Object.values(tele.versions[j.id]).map((v) => v.etat).join());
    if (j && j.id === fiche?.id && !aCartes(j.plateforme)) api.versionsAjoutees(j.id).then((l) => (ajoutees = l)).catch(() => (ajoutees = []));
    else ajoutees = [];
  });
  $effect(() => {
    if (fiche && (fiche.versions?.length ?? 0) > 1) api.versionsAvis(fiche).then((l) => (avis = l)).catch(() => (avis = []));
    else avis = [];
  });

  /** Peut-on ajouter une autre version à ce jeu du PC ? (Au moins une version cochable, pas encore là, et celle
   * d'origine cochable aussi.) Sinon la section le dit, au lieu de rester muette. */
  const ajoutPossible = $derived.by(() => {
    const j = fiche ? jeuSurPc(fiche.id) : undefined;
    if (!fiche || !j || j.id !== fiche.id || aCartes(j.plateforme) || j.etat !== 'telecharge') return null;
    if (j.installation?.lanceur || j.installation?.boutique) return false;
    const coch = (id: number) => !!avis.find((a) => a.telechargement_id === id)?.cochable;
    return coch(j.version) && (fiche.versions ?? []).some((v) => coch(v.telechargement_id) && !cartesSurPc(j).includes(v.telechargement_id));
  });

  // Venu de « ⚙ Gérer ▸ Ajouter une autre version » (`?vers=versions`) : la page descend aux versions UNE fois la fiche
  // et l'avis du cœur là, la sélection sur le premier « ➕ » (jamais sur un « 🗑 Retirer »), sinon sur le titre.
  // (Après la galerie, les documents et les infos, qui s'insèrent AU-DESSUS : essai de Seb, 28/10 — on atterrissait
  // au niveau des images. Défilement instantané, refait une fois si la page a encore bougé, sans jamais reprendre la
  // main à quelqu'un qui défile lui-même.)
  let versFaitPour = '';
  let complements = $state(false);
  $effect(() => {
    if (versFaitPour === page.url.href || page.url.searchParams.get('vers') !== 'versions' || !fiche || !complements || ((fiche.versions?.length ?? 0) > 1 && !avis.length)) return;
    versFaitPour = page.url.href;
    const descendre = () => document.getElementById('versions')?.scrollIntoView({ behavior: 'auto', block: 'start' });
    void tick().then(() => {
      descendre();
      const s = document.getElementById('versions');
      (s?.querySelector<HTMLElement>('[data-ajout]') ?? s?.querySelector<HTMLElement>('h3'))?.focus({ preventScroll: true });
      let bouge = false;
      const geste = () => (bouge = true);
      for (const t of ['wheel', 'keydown', 'pointerdown', 'touchstart', 'focusin']) window.addEventListener(t, geste, { once: true, capture: true });
      setTimeout(() => {
        for (const t of ['wheel', 'keydown', 'pointerdown', 'touchstart', 'focusin']) window.removeEventListener(t, geste, { capture: true });
        const r = document.getElementById('versions')?.getBoundingClientRect();
        if (!bouge && r && Math.abs(r.top) > 40) descendre();
      }, 900);
    });
  });

  /** Retire UNE carte e-Reader du PC (ses fichiers seulement ; les parties sont dans la cartouche). */
  async function retirerUneCarte(id: number, version: number, nom: string) {
    const oui = await confirmer(`🗑 Retirer la carte « ${nom} » ?`, {
      message: 'Son fichier est retiré de ce PC ; les autres cartes et tes parties restent. Tu pourras la remettre plus tard.',
      libelleValider: '🗑 Retirer',
    });
    if (!oui) return;
    try {
      await api.retirerCarte(id, version);
      await rechargerJeuxDuPc();
      toast('🗑 Carte retirée.');
    } catch (e) {
      toast(`Impossible : ${motifDuRefus(e)}`, 'erreur');
    }
  }
</script>

<div class="page">
  <button class="btn retour" onclick={() => history.back()}>← Revenir</button>

  {#if erreur}
    <div class="panel message">
      <p>⛔ {erreur}</p>
    </div>
  {:else if !fiche}
    <p class="muted">Chargement de la fiche…</p>
  {:else}
    {#if horsLigne}
      <p class="tag warn">Hors ligne : fiche de la dernière consultation</p>
    {:else if locale}
      <p class="tag ok">✅ Dans ta ludothèque : fiche et documents gardés sur ce PC</p>
    {/if}
    {#if jeuSurPc(fiche.id)?.etat === 'telecharge'}
      {@const surPc = jeuSurPc(fiche.id)!}
      <div class="actions-jeu">
        {#if !surPc.installation}
          <button class="btn primary" onclick={() => installer(surPc.id)} disabled={partie.installation === surPc.id}>📦 Installer</button>
        {:else}
          <button class="btn primary" onclick={() => jouer(surPc.id)} disabled={partie.enJeu !== null || partie.lancement !== null || libellePreparation(surPc.plateforme) !== null}>{libellePreparation(surPc.plateforme) ?? (partie.lancement === surPc.id ? 'Lancement…' : '▶ Jouer')}</button>
          {#if emulateursDuJeu(surPc.plateforme).liste.length > 1 || (surPc.installation.versions?.length ?? 0) > 1}
            <button class="btn" onclick={() => jouerAvec(surPc.id)} disabled={partie.enJeu !== null || partie.lancement !== null || libellePreparation(surPc.plateforme) !== null}>▶ Jouer avec…</button>
          {/if}
        {/if}
        {#if !surPc.d_un_autre}<button class="btn" onclick={() => gererJeu(surPc.id)} disabled={partie.enJeu === surPc.id}>⚙ Gérer le jeu</button>{/if}
        <a class="btn" href={`/assistant?jeu=${fiche.id}`}>💬 Demander à l’assistant</a>
        <a class="btn" href={`/triches?jeu=${surPc.id}`}>🎯 Triches et mods</a>
      </div>
    {/if}

    <section class="jeu">
      <div class="jaquette"><Jaquette id={fiche.id} titre={fiche.titre} plateforme={fiche.plateforme} largeur={260} /></div>
      <div class="infos">
        <h1>{fiche.titre}</h1>
        {#if fiche.titre_en && fiche.titre_en !== fiche.titre}<p class="muted">{fiche.titre_en}</p>{/if}
        <p class="sous">
          {fiche.plateforme_affichee || nomAffiche(fiche.plateforme)}{fiche.annee ? ` · ${fiche.annee}` : ''}
        </p>
        {#if fiche.genres?.length || typeDeSortie(infos?.type_sortie)}
          <div class="genres">
            <!-- Un jeu pas sorti normalement (accès anticipé, jamais sorti…) : signalé d'emblée, pas en bas de liste. -->
            {#if typeDeSortie(infos?.type_sortie)}<span class="tag warn">{typeDeSortie(infos?.type_sortie)}</span>{/if}
            {#each fiche.genres ?? [] as g (g)}<span class="tag">{g}</span>{/each}
          </div>
        {/if}
        {#if fiche.resume}<p class="resume">{fiche.resume}</p>{/if}
        <dl class="cx-kv">
          <!-- La date de sortie seulement si elle dit plus que l'année (déjà sous le titre), ou si l'année manque. -->
          {#if dateDeSortie(infos?.date_sortie) && (datePrecise(infos?.date_sortie) || !fiche.annee)}
            <dt>Sortie</dt><dd>{dateDeSortie(infos?.date_sortie)}</dd>
          {/if}
          {#if fiche.developpeur}<dt>Développeur</dt><dd>{fiche.developpeur}</dd>{/if}
          {#if fiche.editeur}<dt>Éditeur</dt><dd>{fiche.editeur}</dd>{/if}
          {#if fiche.joueurs || infos?.joueurs_massif}
            <dt>Joueurs</dt>
            <dd>
              {fiche.joueurs
                ? [fiche.joueurs, fiche.cooperatif ? '(coopératif)' : '', infos?.joueurs_massif ? '(massivement multijoueur)' : '']
                    .filter(Boolean)
                    .join(' ')
                : 'Massivement multijoueur'}
            </dd>
          {/if}
          {#if ageConseille(infos)}<dt>Âge</dt><dd>{ageConseille(infos)}</dd>{/if}
          {#if noteSurCinq(infos?.note)}<dt>Avis des joueurs</dt><dd><span aria-hidden="true">★</span> {noteSurCinq(infos?.note)}</dd>{/if}
        </dl>
      </div>
    </section>

    {#if medias.some((m) => !ratees.includes(m.id))}
      <section class="cx-block">
        <h3>🖼 Images du jeu</h3>
        <div class="galerie">
          {#each medias as m (m.id)}
            {#if !ratees.includes(m.id)}
              <button class="vignette" bind:this={boutonsMedias[m.id]} title={libelleMedia(m)} onclick={() => (imageOuverte = m.id)}>
                <span class="cadre">
                  <img
                    src={adresseMedia(fiche.id, m.id, largeurUtile(m, pixelsEcran()))}
                    alt={libelleMedia(m)}
                    loading="lazy"
                    decoding="async"
                    class:pixels={grosPixels(m, pixelsEcran())}
                    onerror={() => (ratees = [...ratees, m.id])}
                  />
                </span>
                <span class="muted">{libelleMedia(m)}</span>
              </button>
            {/if}
          {/each}
        </div>
      </section>
    {/if}

    {#if fiche.annexes?.length || documents.length}
      <section class="cx-block">
        <h3>📚 Documents du jeu</h3>
        <div class="annexes">
          {#each documents as d (d.id)}
            {@const l = libelleDocument(d)}
            <button class="btn" bind:this={boutonsDocuments[d.id]} disabled={ouverture !== null} onclick={() => ouvrir(d)}>
              {l.titre}
              <span class="muted">{ouverture === d.id ? texteChargement(suiviDocument.courant) : l.detail}</span>
            </button>
            {#if ouverture === d.id && !d.garde}
              <button class="btn" bind:this={boutonArret} onclick={() => fiche && arreterDoc(fiche.id, d.id)}>⏹ Arrêter</button>
            {/if}
          {/each}
          {#each fiche.annexes ?? [] as a (a.i)}
            <button class="btn" onclick={() => ouvrirAnnexe(a)}>
              {ICONES_ANNEXES[a.type] ?? '📎'} {a.titre}
              <span class="muted">{a.texte ? 'à lire' : locale ? 'ouvrir' : taille(a.taille)}</span>
            </button>
          {/each}
        </div>
        {#if annexe}
          <article class="lecture panel">
            <header>{annexe.titre}</header>
            <div class="texte">
              {#each paragraphes(annexe.texte) as p, i (i)}
                <p>{#each p as m, k (k)}{#if m.gras}<strong>{m.t}</strong>{:else}{m.t}{/if}{/each}</p>
              {/each}
            </div>
            <button class="btn" onclick={() => (annexe = null)}>Fermer</button>
          </article>
        {/if}
      </section>
    {/if}

    <section class="cx-block" id="versions">
      <h3 tabindex="-1">📦 Versions disponibles</h3>
      {#if ajoutPossible === false && !horsLigne}<p class="muted">Aucune autre version ne peut s’ajouter à ce jeu (seulement pour des ROM ou des images disque).</p>{/if}
      {#if !fiche.versions?.length}
        <p class="muted">Aucune version n’est encore rangée dans Firehouse pour ce jeu.</p>
      {:else}
        <div class="versions">
          {#each fiche.versions as v (v.telechargement_id)}
            {@const n = nature(v.qualite)}
            <article class="version cx-card">
              <header>
                <span class="tag {n.ton}">{n.libelle}</span>
                <h4>{v.nom}</h4>
              </header>
              <ul class="fichiers">
                {#each v.fichiers as f (f.nom)}
                  <li><span>{f.nom}</span><span class="muted">{taille(f.taille)}</span></li>
                {/each}
              </ul>
              <p class="muted">
                Total : {taille(v.fichiers.reduce((s, f) => s + f.taille, 0))}
                {#if v.range_le}· rangée le {new Date(v.range_le).toLocaleDateString('fr-FR')}{/if}
              </p>
              {#if v.notes}
                <details>
                  <summary>Notes de cette version</summary>
                  <p class="notes">{v.notes}</p>
                  {#if v.notes_a_traduire}<p class="muted">Ces notes ne sont pas encore traduites.</p>{/if}
                </details>
              {/if}
              {#if aCartes(fiche.plateforme)}
                <!-- Un jeu à cartes (e-Reader) : chaque carte s'ajoute ou se retire à part. -->
                {#if cartesSurPc(jeuSurPc(fiche.id)).includes(v.telechargement_id)}
                  <p class="muted">✅ Cette carte est sur ce PC.</p>
                  {#if jeuSurPc(fiche.id)?.installation}
                    <button class="btn primary" onclick={() => fiche && jouer(fiche.id, undefined, undefined, v.telechargement_id)} disabled={partie.enJeu !== null}>▶ Lire cette carte</button>
                  {/if}
                  {#if cartesSurPc(jeuSurPc(fiche.id)).length > 1}
                    <button class="btn" onclick={() => fiche && retirerUneCarte(fiche.id, v.telechargement_id, v.nom)}>🗑 Retirer cette carte</button>
                  {/if}
                {:else if !horsLigne}
                  <button class="btn" onclick={() => fiche && mettreDansLaLudotheque(fiche.id, v.telechargement_id)}>➕ Ajouter cette carte</button>
                {/if}
              {:else if jeuSurPc(fiche.id)?.id === fiche.id}
                <!-- Déjà sur ce PC : chaque version (région) s'ajoute ou se retire à part ; le jeu reste jouable. -->
                {@const surPc = jeuSurPc(fiche.id)!}
                {@const enRoute = tele.versions[fiche.id]?.[v.telechargement_id]}
                {@const ajoutee = enRoute ?? ajoutees.find((a) => a.version === v.telechargement_id)}
                {#if surPc.etat !== 'telecharge'}
                  <!-- Le jeu lui-même arrive encore : rien à ajouter ni à retirer version par version. -->
                {:else if ajoutee && ajoutee.etat !== 'telecharge'}
                  <p class="muted">
                    {ETATS_VERSION[ajoutee.etat]}{enRoute && enRoute.total ? ` — ${Math.floor((enRoute.recus / enRoute.total) * 100)} %` : ''}{ajoutee.message ? ` : ${ajoutee.message}` : ''}
                  </p>
                  {#if ajoutee.etat !== 'en_cours'}
                    <button class="btn primary" onclick={() => fiche && reprendre(fiche.id)}>▶ Reprendre</button>
                  {/if}
                  <button class="btn" onclick={() => fiche && retirerUneVersion(fiche.id, v.telechargement_id, v.nom, true)}>✖ Annuler l’arrivée</button>
                {:else if cartesSurPc(surPc).includes(v.telechargement_id)}
                  <p class="muted">✅ Cette version est sur ce PC{v.telechargement_id === surPc.version ? ' (celle d’origine)' : ''}.</p>
                  {#if v.telechargement_id !== surPc.version}
                    <button class="btn" onclick={() => fiche && retirerUneVersion(fiche.id, v.telechargement_id, v.nom, false)}>🗑 Retirer cette version</button>
                  {/if}
                {:else if !horsLigne && !surPc.installation?.lanceur && !surPc.installation?.boutique && avis.find((a) => a.telechargement_id === v.telechargement_id)?.cochable && avis.find((a) => a.telechargement_id === surPc.version)?.cochable}
                  <button class="btn" data-ajout onclick={() => fiche && ajouterUneVersion(fiche.id, v, fiche.titre)}>➕ Ajouter cette version</button>
                {/if}
              {:else if !locale && !jeuSurPc(fiche.id)}
                <button class="btn" onclick={() => fiche && mettreDansLaLudotheque(fiche.id)}>➕ Mettre dans ma ludothèque</button>
              {/if}
            </article>
          {/each}
        </div>
      {/if}
    </section>
  {/if}
</div>

{#if imageOuverte !== null && fiche}
  <VisionneuseImages
    jeu={fiche.id}
    medias={medias.filter((m) => !ratees.includes(m.id))}
    depart={imageOuverte}
    titre={fiche.titre}
    sousLaBarre
    onfermer={fermerImage}
  />
{:else if ouvert?.livret && fiche}
  <LecteurLivret jeu={fiche.id} doc={ouvert.doc} pages={ouvert.pages} titre={fiche.titre} sousLaBarre onfermer={fermer} />
{:else if ouvert && !ouvert.livret && fiche}
  <LecteurMedia jeu={fiche.id} doc={ouvert.doc} video={ouvert.video} titre={fiche.titre} sousLaBarre onfermer={fermer} />
{/if}

<style>
  .galerie {
    display: flex;
    flex-wrap: wrap;
    gap: calc(10 * var(--u));
  }
  /* Une cellule fixe (grille régulière, la croix de la manette s'y déplace bien) ; l'image s'y loge en entier. Le
     contour de sélection reste sur le bouton (l'« overflow » est sur le cadre intérieur). */
  .vignette {
    display: grid;
    gap: calc(4 * var(--u));
    width: calc(160 * var(--u));
    padding: calc(4 * var(--u));
    border: 1px solid var(--line);
    border-radius: calc(10 * var(--u));
    background: var(--bg2);
    color: var(--ink);
    cursor: pointer;
    text-align: center;
  }
  .vignette:hover {
    border-color: var(--accent);
  }
  .vignette .cadre {
    height: calc(120 * var(--u));
    overflow: hidden;
    border-radius: calc(6 * var(--u));
    display: grid;
    place-items: center;
    background: repeating-conic-gradient(var(--damier-clair) 0% 25%, var(--damier-fonce) 0% 50%) 50% / 12px 12px;
  }
  .vignette img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }
  .vignette img.pixels {
    image-rendering: pixelated;
  }
  .vignette .muted {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .page {
    padding: calc(16 * var(--u)) calc(24 * var(--u)) calc(32 * var(--u));
    display: grid;
    gap: calc(16 * var(--u));
    max-width: calc(1200 * var(--u));
    margin: 0 auto;
  }
  .actions-jeu {
    display: flex;
    gap: calc(8 * var(--u));
    flex-wrap: wrap;
  }
  .retour {
    justify-self: start;
    text-decoration: none;
  }
  .message {
    padding: calc(16 * var(--u));
  }
  .jeu {
    display: grid;
    grid-template-columns: calc(260 * var(--u)) 1fr;
    gap: calc(24 * var(--u));
  }
  h1 {
    margin: 0;
    font-size: calc(30 * var(--u));
  }
  .sous {
    margin: calc(4 * var(--u)) 0 calc(10 * var(--u));
    color: var(--dim);
  }
  .genres {
    display: flex;
    flex-wrap: wrap;
    gap: calc(6 * var(--u));
  }
  .resume {
    white-space: pre-line;
    line-height: 1.6;
  }
  .infos .cx-kv {
    max-width: calc(520 * var(--u));
  }
  .annexes {
    display: flex;
    flex-wrap: wrap;
    gap: calc(8 * var(--u));
  }
  .lecture {
    margin-top: calc(12 * var(--u));
    display: grid;
    gap: calc(10 * var(--u));
    padding-bottom: calc(12 * var(--u));
  }
  .lecture .texte {
    padding: 0 calc(16 * var(--u));
    line-height: 1.6;
  }
  .lecture > .btn {
    justify-self: end;
    margin-right: calc(12 * var(--u));
  }
  .versions {
    display: grid;
    gap: calc(12 * var(--u));
  }
  .version {
    padding: calc(14 * var(--u));
    display: grid;
    gap: calc(8 * var(--u));
    background: var(--panel);
  }
  .version:hover {
    transform: none;
  }
  .version header {
    display: grid;
    gap: calc(6 * var(--u));
    justify-items: start;
  }
  h4 {
    margin: 0;
    font-size: calc(15 * var(--u));
  }
  .fichiers {
    list-style: none;
    margin: 0;
    padding: 0;
    font-family: var(--mono);
    font-size: calc(12.5 * var(--u));
  }
  .fichiers li {
    display: flex;
    justify-content: space-between;
    gap: calc(12 * var(--u));
    padding: calc(4 * var(--u)) 0;
    border-bottom: 1px solid var(--line);
  }
  .notes {
    white-space: pre-line;
  }
  .version > .btn {
    justify-self: start;
  }
  @media (max-width: 720px) {
    .jeu {
      grid-template-columns: 1fr;
    }
    .jaquette {
      max-width: calc(220 * var(--u));
    }
  }
</style>
