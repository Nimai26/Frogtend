<script lang="ts">
  // 🧩 Les mods Switch d'un jeu (contrat 1.44, ✅ Seb 18/11) : servis par Firehouse (validés par un admin), téléchargés
  // après l'accord chiffré de la personne, un interrupteur par mod pour toute la maison (Eden comme Ryubing), posés
  // dans l'émulateur au lancement seulement. Le même bloc sert au bureau (panneau du jeu) et au canapé (Taodbox,
  // « Réglages du jeu ») : tout se fait au clavier, à la souris ou à la manette, avec les fenêtres maison.
  import { api, estErreurCoeur, taille, type ModSwitchVu, type ModsSwitch, type SuiviModsSwitch } from '$lib/api';
  import { choisir, cocher, confirmer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { etat, reglerProfil } from '$lib/etat.svelte';
  import { attenteEmulateurSeul, lancementImpossible } from '$lib/ludotheque/jeu.svelte';
  import { debitConnu } from '$lib/ludotheque/telechargements.svelte';
  import { auCanape } from '$lib/taodbox/canape';
  import { selectionner } from '$lib/taodbox/manette.svelte';
  import { tick } from 'svelte';
  import { lignesRemarque, motifInterrupteursBloques } from './contenus';
  import {
    aMettreAJour,
    aTelecharger,
    aUnInterrupteur,
    blocModsVisible,
    cibleDuFocus,
    clesVues,
    conflitAvec,
    descriptionLongue,
    dureeEstimee,
    etapeModsAnnoncee,
    ligneAuteur,
    ligneEtatListe,
    ligneProgresMods,
    lignesDernierePose,
    lignesFinMods,
    mentions,
    messageFinMods,
    messageTelechargement,
    raisonGrise,
    pasEncoreEssaye,
    pourcentEnCours,
    resumeMods,
    texteInterrupteurMod,
    texteSimple,
    titreTelechargement,
    totalOctets,
  } from './mods';

  // `etatBloc` (lu par l'écran « Réglages du jeu ») : `attente` tant que la liste n'est pas lue, puis `vide` ou
  // `visible` — pour dire « Rien à régler » plutôt qu'un écran muet.
  let {
    id,
    titre,
    deplie = false,
    etatBloc = $bindable('attente'),
  }: { id: number; titre: string; deplie?: boolean; etatBloc?: 'attente' | 'vide' | 'visible' } = $props();

  let liste = $state<ModsSwitch | null>(null);
  let ouvert = $state(false);
  let suivi = $state<SuiviModsSwitch | null>(null);
  /** Les mods demandés à l'instant (en attente dans la file du cœur). */
  let enFile = $state<string[]>([]);
  let accordEnCours = $state(false);
  let basculement = $state<string | null>(null);
  let retrait = $state<string | null>(null);
  /** Les descriptions dépliées (« Lire plus »). */
  let lus = $state<string[]>([]);
  /** Ce que le lecteur d'écran entend à la fin d'un téléchargement (« Mods téléchargés »). */
  let annonceFin = $state('');
  let charge = $state(false);
  let bloc = $state<HTMLElement | null>(null);
  let entete = $state<HTMLButtonElement | null>(null);

  const mods = $derived(liste?.mods ?? []);
  const nouveaux = $derived(aTelecharger(mods));
  // Pendant une partie ou un émulateur ouvert seul : rien ne change (il réécrirait ses réglages par-dessus).
  const bloque = $derived(lancementImpossible());
  const occupe = $derived(!!suivi?.actif || accordEnCours);
  const horsLigne = $derived(liste?.etat !== 'ok');
  const actif = $derived(!!suivi?.actif);
  const pose = $derived(liste?.derniere_pose ?? null);

  $effect(() => {
    etatBloc = !charge ? 'attente' : blocModsVisible(liste) ? 'visible' : 'vide';
  });

  $effect(() => {
    const jeu = id;
    liste = null;
    suivi = null;
    enFile = [];
    lus = [];
    annonceFin = '';
    charge = false;
    ouvert = deplie;
    charger(jeu);
  });

  /** En tâche de fond : le panneau est déjà affiché, la liste vient ensuite (ou la dernière vue, hors ligne). */
  async function charger(jeu: number) {
    const l = await api.modsSwitchLister(jeu).catch(() => null);
    if (jeu !== id) return;
    liste = l;
    charge = true;
    const su = await api.modsSwitchSuivi(jeu).catch(() => null);
    if (jeu !== id) return;
    suivi = su;
    // « À l'avenir, mettre à jour tout seul » (choisi par la personne, désactivable dans les options) : les nouvelles
    // révisions partent seules — jamais pendant un lancement, jamais deux fois.
    const majs = aMettreAJour(l?.mods ?? []);
    if (l?.etat === 'ok' && majs.length && etat.profil.modsSwitch?.auto && !su?.actif && !lancementImpossible() && !accordEnCours) {
      accordEnCours = true;
      try {
        await telecharger(jeu, majs, true);
      } finally {
        accordEnCours = false;
      }
    }
  }

  /** Relit la liste (après un choix, un téléchargement) ; en cas d'échec, l'ancienne reste (jamais un bloc vide). */
  async function relire() {
    const jeu = id;
    const l = await api.modsSwitchLister(jeu).catch(() => null);
    if (l && jeu === id) liste = l;
  }

  /** L'ordre des mods quand l'action a commencé (pour trouver « la suivante » d'une ligne retirée). */
  let ordre: string[] = [];
  const noterOrdre = () => (ordre = mods.map((m) => m.cle));

  /** Après une action (Télécharger, Retirer…), l'élément qui avait le focus a pu disparaître : le focus revient à la
   * ligne du mod (ou à la suivante, sinon à l'en-tête), jamais en haut de la page. */
  async function rendreFocus(cle: string) {
    await tick();
    const a = document.activeElement;
    if (a && a !== document.body && a.isConnected) return;
    // La première commande active de la ligne (ou d'une suivante), que la manette sait viser — jamais la ligne
    // `tabindex="-1"` elle-même : `deplacer` repartirait alors du haut de la page.
    const lignes = [...(bloc?.querySelectorAll<HTMLElement>('li[data-cle]') ?? [])];
    const cible = cibleDuFocus(lignes, cle, ordre, entete);
    if (cible) selectionner(cible);
  }

  /** Met en file la liste que la personne a VUE : si elle a changé entre-temps, le cœur refuse et on la relit. */
  async function telecharger(jeu: number, l: ModSwitchVu[], auto: boolean) {
    try {
      const n = await api.modsSwitchTelecharger(jeu, clesVues(l), totalOctets(l), auto);
      if (auto && n) toast(`🧩 ${n} mod(s) de « ${titre} » se mettent à jour tout seuls (${taille(totalOctets(l))}), en arrière-plan.`);
      if (jeu !== id) return;
      enFile = l.map((m) => m.cle);
      const su = await api.modsSwitchSuivi(jeu).catch(() => null);
      if (jeu !== id || !su) return;
      suivi = su;
      // Un mod très léger a pu finir avant cette première lecture : la minuterie ne démarrera jamais, la fin se dit ici.
      if (!su.actif && enFile.length) await finir(su);
    } catch (e) {
      if (estErreurCoeur(e) && e.sorte === 'conflit') {
        toast(`🧩 ${texteSimple(motifDuRefus(e), false)}`, 'alerte');
        if (jeu === id) await relire();
        return;
      }
      toast(`Téléchargement impossible : ${texteSimple(motifDuRefus(e), false)}`, 'erreur');
    }
  }

  /** La fin d'un téléchargement : un toast (combien sont prêts), « Mods téléchargés » pour le lecteur d'écran, la
   * liste relue (nouveaux états). */
  async function finir(su: SuiviModsSwitch) {
    enFile = [];
    const fin = messageFinMods(su);
    if (fin) toast(fin, 'ok');
    annonceFin = su.faits ? 'Mods téléchargés.' : 'Téléchargement des mods terminé.';
    await relire();
  }

  /** « ⬇ Télécharger (X Mo) » : l'accord chiffré (taille, durée si connue, toute la maison, abri des parties). */
  async function proposer(m: ModSwitchVu) {
    if (occupe || bloque) return;
    accordEnCours = true;
    try {
      noterOrdre();
      const oui = await confirmer(titreTelechargement([m], debitConnu()), { message: messageTelechargement([m]), libelleValider: '⬇ Télécharger' });
      if (oui) await telecharger(id, [m], false);
    } finally {
      accordEnCours = false;
    }
    await rendreFocus(m.cle);
  }

  /** « Tout télécharger » : tout coché d'office, le total recalculé à chaque case. */
  async function proposerTout() {
    if (occupe || bloque || !nouveaux.length) return;
    accordEnCours = true;
    const vus = nouveaux;
    noterOrdre();
    try {
      const debit = debitConnu();
      const choisis = await cocher<string>(
        `🧩 Télécharger des mods pour « ${titre} » ?`,
        vus.map((m) => ({ valeur: m.cle, libelle: m.nom, detail: taille(m.taille) })),
        {
          coches: vus.map(() => true),
          message: messageTelechargement(vus),
          resume: (v) => {
            const t = totalOctets(vus.filter((m) => v.includes(m.cle)));
            const d = dureeEstimee(t, debit);
            return `${v.length} mod(s), ${taille(t)}${d ? `, ${d}` : ''}`;
          },
          refus: (v) => (v.length ? null : 'Coche au moins un mod.'),
          libelleValider: '⬇ Télécharger',
        },
      );
      if (choisis?.length) await telecharger(id, vus.filter((m) => choisis.includes(m.cle)), false);
    } finally {
      accordEnCours = false;
    }
    await rendreFocus(vus[0].cle);
  }

  /** Une nouvelle révision : le même accord, avec « à l'avenir, mettre à jour tout seul ». */
  async function proposerMiseAJour(m: ModSwitchVu) {
    if (occupe || bloque) return;
    accordEnCours = true;
    try {
      const c = await choisir<'oui' | 'auto' | 'non'>(
        titreTelechargement([m], debitConnu(), true),
        [
          { valeur: 'oui', libelle: '⬇ Mettre à jour' },
          {
            valeur: 'auto',
            libelle: '⬇ Mettre à jour, et à l’avenir mettre à jour tout seul',
            // Au canapé, l'interrupteur est sur l'écran « Réglages du jeu » ; au bureau, dans les options.
            detail: auCanape() ? 'Sans redemander (désactivable ici, en haut de « Réglages du jeu »)' : 'Sans redemander (désactivable dans ⚙ Options ▸ 💾 Sauvegarde)',
          },
          { valeur: 'non', libelle: 'Pas maintenant' },
        ],
        messageTelechargement([m], true),
      );
      if (c !== 'oui' && c !== 'auto') return;
      noterOrdre();
      if (c === 'auto') await reglerProfil('modsSwitch.auto', true);
      await telecharger(id, [m], c === 'auto');
    } finally {
      accordEnCours = false;
    }
    await rendreFocus(m.cle);
  }

  /** Demande d'éteindre l'autre mod (conflit), puis enregistre ; rend faux si la personne refuse. */
  async function accordConflit(m: ModSwitchVu, b: ModSwitchVu): Promise<boolean> {
    return confirmer(`Ne va pas avec « ${b.nom} » : l’allumer éteint l’autre`, {
      message: [
        `« ${m.nom} » et « ${b.nom} » ne peuvent pas marcher ensemble.`,
        `Allumer « ${m.nom} » éteint « ${b.nom} », pour toute la maison.`,
        'Frogtend met tes parties de ce jeu à l’abri avant la prochaine partie.',
      ].join('\n'),
      libelleValider: 'Allumer',
    });
  }

  async function basculer(m: ModSwitchVu) {
    // Pris AVANT toute fenêtre : un double appui n'ouvre jamais deux fenêtres ni deux enregistrements.
    if (bloque || basculement || !liste) return;
    basculement = m.cle;
    const allume = !m.allume;
    try {
      let eteindre: string | null = null;
      if (allume) {
        const b = conflitAvec(mods, m.cle);
        if (b) {
          if (!(await accordConflit(m, b))) return;
          eteindre = b.cle;
        }
      } else if (m.touche_sauvegardes) {
        const oui = await confirmer(`Éteindre « ${m.nom} » ?`, {
          message: 'Ce mod touche à tes parties : le jeu peut parfois refuser une partie faite avec lui.\nFrogtend met tes parties de ce jeu à l’abri avant la prochaine partie.\nCe choix vaut pour toute la maison.',
          libelleValider: 'Éteindre',
        });
        if (!oui) return;
      }
      await enregistrer(m, allume, eteindre);
      await relire();
    } finally {
      basculement = null;
    }
  }

  async function enregistrer(m: ModSwitchVu, allume: boolean, eteindre: string | null) {
    try {
      const change = await api.modsSwitchChoisir(id, m.cle, allume, eteindre);
      if (change) toast(`✅ « ${m.nom} » ${allume ? 'allumé' : 'éteint'} : tes parties de ce jeu seront mises à l’abri avant la prochaine partie.`);
    } catch (e) {
      // Un conflit que la liste affichée ne montrait pas (elle a changé) : relue, puis la même question.
      if (allume && !eteindre && estErreurCoeur(e) && e.sorte === 'refus') {
        await relire();
        const b = conflitAvec(liste?.mods ?? [], m.cle);
        if (b) {
          if (await accordConflit(m, b)) await enregistrer(m, true, b.cle);
          return;
        }
      }
      toast(`Impossible d’enregistrer : ${texteSimple(motifDuRefus(e), false)}`, 'erreur');
    }
  }

  async function retirer(m: ModSwitchVu) {
    if (bloque || retrait || occupe) return;
    retrait = m.cle;
    noterOrdre();
    try {
      const oui = await confirmer(`Retirer « ${m.nom} » de ce PC ?`, {
        message: 'Firehouse ne le propose plus. Il quitte les émulateurs à la prochaine partie.\nFrogtend met tes parties de ce jeu à l’abri avant.',
        libelleValider: 'Retirer',
        danger: true,
      });
      if (!oui) return;
      try {
        await api.modsSwitchRetirer(id, m.cle);
        toast(`✅ « ${m.nom} » retiré de ce PC.`);
      } catch (e) {
        toast(`Impossible de retirer : ${texteSimple(motifDuRefus(e), false)}`, 'erreur');
      }
      await relire();
    } finally {
      retrait = null;
    }
    await rendreFocus(m.cle);
  }

  // Pendant un téléchargement : relu toutes les 2 s (rien n'est demandé à Firehouse, seulement au cœur) ; à la fin, la
  // liste est relue (nouveaux états).
  // `actif` (dérivé) : la minuterie n'est recréée qu'au début et à la fin, jamais à chaque relecture du suivi.
  $effect(() => {
    if (!actif) return;
    const jeu = id;
    annonceFin = '';
    const minuterie = setInterval(async () => {
      const su = await api.modsSwitchSuivi(jeu).catch(() => null);
      if (!su || jeu !== id) return;
      suivi = su;
      // La fin se dit : un toast (combien sont prêts) et, pour le lecteur d'écran, « Mods téléchargés ».
      if (!su.actif) await finir(su);
    }, 2000);
    return () => clearInterval(minuterie);
  });

  const dansLaFile = (m: ModSwitchVu) => !!suivi?.actif && (enFile.includes(m.cle) || suivi.en_cours === m.nom);
  const enCours = (m: ModSwitchVu) => !!suivi?.actif && suivi.en_cours === m.nom;
  const lu = (cle: string) => lus.includes(cle);
  const basculerLecture = (cle: string) => (lus = lu(cle) ? lus.filter((c) => c !== cle) : [...lus, cle]);
  // Les identifiants HTML d'un mod (sa clé est « source-12 hex » : déjà sûre).
  const ident = (quoi: string, m: ModSwitchVu) => `mod-${quoi}-${m.cle}`;
</script>

{#if blocModsVisible(liste) && liste}
  <div class="mods cx-block" bind:this={bloc}>
    <button class="btn entete" bind:this={entete} aria-expanded={ouvert} onclick={() => (ouvert = !ouvert)}>
      🧩 Mods : <span aria-live="polite">{resumeMods(mods)}</span> {ouvert ? '▴' : '▾'}
    </button>
    {#if ligneEtatListe(liste)}<p class="muted">{ligneEtatListe(liste)}</p>{/if}
    {#each lignesRemarque(liste.remarque) as l (l)}<p class="muted">{l}</p>{/each}
    <!-- Ce que la dernière partie a dit (un mod allumé pas mis en place, un abri impossible…) : toujours dit. -->
    {#each lignesDernierePose(pose) as l, i (i)}<p class={i ? 'muted' : ''}>{l}</p>{/each}
    <!-- Le lecteur d'écran n'annonce que l'ÉTAPE (le mod en cours) puis la fin, jamais le pourcentage qui bouge. -->
    <span class="annonce" aria-live="polite">{etapeModsAnnoncee(suivi) || annonceFin}</span>
    {#if ligneProgresMods(suivi)}<p>{ligneProgresMods(suivi)}</p>{/if}
    {#each lignesFinMods(suivi) as l, i (i)}<p class="muted">{l}</p>{/each}
    {#if ouvert}
      <p class="muted">Ces mods viennent de Firehouse. Chacun vaut pour ce jeu, avec Eden comme avec Ryubing, pour toute la maison.</p>
      {#if bloque}<p class="muted">{motifInterrupteursBloques(attenteEmulateurSeul())}</p>{/if}
      {#if nouveaux.length >= 2 && !horsLigne}
        <div class="actions">
          <button class="btn petit" onclick={proposerTout} disabled={occupe || bloque}>
            ⬇ Tout télécharger ({nouveaux.length}, {taille(totalOctets(nouveaux))})
          </button>
        </div>
      {/if}
      <ul>
        {#each mods as m (m.cle)}
          <!-- `tabindex="-1"` : le focus y revient après une action qui fait disparaître le bouton choisi. -->
          <li class:grise={m.etat === 'incompatible'} data-cle={m.cle} tabindex="-1">
            <span class="nom">{m.nom}</span>
            <span class="action">
              {#if aUnInterrupteur(m)}
                <!-- Un interrupteur : son texte dit l'état (jamais la couleur seule). Celui qui s'enregistre garde le
                     focus (aria-disabled) ; les autres sont désactivés pendant ce temps. -->
                <button
                  class="btn petit"
                  role="switch"
                  aria-checked={m.allume}
                  aria-label={m.nom}
                  aria-describedby={ident('etat', m)}
                  aria-disabled={bloque || basculement !== null}
                  disabled={bloque || (basculement !== null && basculement !== m.cle)}
                  onclick={() => basculer(m)}
                >
                  <span id={ident('etat', m)}>{texteInterrupteurMod(m, pose)}</span>
                </button>
              {:else if enCours(m)}
                <span>⬇ {pourcentEnCours(suivi)} %</span>
              {:else if dansLaFile(m)}
                <span class="muted">⏳ En attente</span>
              {/if}
            </span>
            <div class="details">
              <p class="muted">{ligneAuteur(m)}</p>
              {#if m.description.trim()}
                <p class="description" class:replie={descriptionLongue(m.description) && !lu(m.cle)} id={ident('desc', m)}>{m.description}</p>
                {#if descriptionLongue(m.description)}
                  <button class="lien" aria-expanded={lu(m.cle)} aria-controls={ident('desc', m)} onclick={() => basculerLecture(m.cle)}>
                    {lu(m.cle) ? 'Lire moins' : 'Lire plus'}
                  </button>
                {/if}
              {/if}
              {#each mentions(m) as l (l)}<p class="muted">{l}</p>{/each}
              {#if pasEncoreEssaye(m)}<p class="muted discret">pas encore essayé dans un émulateur</p>{/if}
              {#if m.etat === 'incompatible'}
                <!-- Jamais posé (l'émulateur l'ignorerait en silence) : un bouton focalisable qui dit pourquoi. -->
                <p id={ident('raison', m)}>{raisonGrise(m)}</p>
                <div class="actions">
                  <button class="btn petit" aria-disabled="true" aria-describedby={ident('raison', m)}>⛔ Ne va pas avec ta version</button>
                </div>
              {:else if m.etat === 'pas_telecharge' && !dansLaFile(m) && m.servi}
                <div class="actions">
                  <button class="btn petit" onclick={() => proposer(m)} disabled={occupe || bloque || horsLigne}>⬇ Télécharger ({taille(m.taille)})</button>
                </div>
              {:else if m.etat === 'revision_dispo' && !dansLaFile(m) && m.servi}
                <div class="actions">
                  <button class="btn petit" onclick={() => proposerMiseAJour(m)} disabled={occupe || bloque || horsLigne}>
                    ⬇ Mettre à jour ({taille(m.taille)})
                  </button>
                </div>
              {/if}
              {#if !m.servi}
                <div class="actions">
                  <button class="btn petit danger" onclick={() => retirer(m)} disabled={bloque || occupe || retrait !== null}>🗑 Retirer</button>
                </div>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<style>
  .mods {
    display: grid;
    gap: calc(6 * var(--u));
  }
  .entete {
    justify-content: flex-start;
    text-align: left;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: calc(10 * var(--u));
  }
  /* Panneau étroit (~300 u) : le nom prend la place, l'interrupteur à droite, les détails dessous sur toute la largeur. */
  li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: calc(4 * var(--u)) calc(8 * var(--u));
    align-items: center;
    padding-bottom: calc(8 * var(--u));
    border-bottom: 1px solid var(--line);
  }
  li:last-child {
    border-bottom: none;
  }
  .nom {
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .grise .nom {
    color: var(--dim);
  }
  .details {
    grid-column: 1 / -1;
    display: grid;
    gap: calc(4 * var(--u));
    min-width: 0;
  }
  .description {
    overflow-wrap: anywhere;
    white-space: pre-line;
  }
  .replie {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
  }
  .lien {
    justify-self: start;
    background: none;
    border: none;
    padding: calc(4 * var(--u)) 0;
    min-height: var(--cible-min);
    color: var(--accent2-texte);
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
  }
  .discret {
    font-style: italic;
  }
  .actions {
    display: flex;
    gap: calc(6 * var(--u));
    flex-wrap: wrap;
  }
  /* Les petits boutons du bloc : la cible garde 40 px au moins (charte § 4). */
  .petit {
    min-height: var(--cible-min);
    padding: calc(5 * var(--u)) calc(11 * var(--u));
    font-size: calc(12 * var(--u));
  }
  li:focus {
    outline: calc(2 * var(--u)) solid var(--accent);
    outline-offset: calc(2 * var(--u));
  }
  /* Un bouton « aria-disabled » reste focalisable (il dit pourquoi) mais se montre éteint. */
  button[aria-disabled='true']:not([role='switch']) {
    opacity: 0.6;
    cursor: default;
  }
  /* Lu par un lecteur d'écran, jamais affiché. */
  .annonce {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  p {
    margin: 0;
  }
</style>
