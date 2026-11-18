<script lang="ts">
  // 📦 Les contenus additionnels d'un jeu du PC : trouvés sur le disque. PS3 : cochés par la personne, installés
  // seulement après son oui (taille annoncée). Switch, Ryubing et Eden (Seb 16/11 : « pour tout ce qui est update, DLC,
  // on peut passer en automatique ») : utilisés tout seuls à chaque partie ; depuis le 17/11, Frogtend est la source de
  // vérité — un interrupteur par contenu, allumé d'office, pour ce jeu et toute la maison. Wii U : inclus dans le .wua.
  import { api, estErreurCoeur, taille } from '$lib/api';
  import { choisir, confirmer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { etat, reglerProfil } from '$lib/etat.svelte';
  import {
    envoiArretable,
    envoisEnRoute,
    etapeAnnoncee,
    ligneEcarte,
    ligneEnvoi,
    ligneProgres,
    ligneProposition,
    messageAccord,
    titreAccord,
    type ExamenEnvois,
    type SuiviEnvois,
  } from './envois';
  import { attenteEmulateurSeul, lancementImpossible } from '$lib/ludotheque/jeu.svelte';
  import {
    cochesCoherentes,
    dlcEnPartieEden,
    emulateurAutomatique,
    motifInterrupteursBloques,
    PHRASE_CHOIX,
    quelquesNoms,
    libelleGenre,
    lignesRemarque,
    nomCourt,
    raisonNonPose,
    REMARQUE_EDEN,
    resume,
    resumeChoix,
    texteInterrupteur,
    type ContenusDuJeu,
    type ContenuVu,
  } from './contenus';
  import BlocMods from './BlocMods.svelte';

  // `plateforme` : un jeu Switch montre aussi ses mods (🧩, ✅ Seb 18/11). `deplie` : la liste ouverte d'office (Taodbox,
  // « Réglages du jeu » : rien à déplier à la manette avant de régler).
  // `vide` (lu par l'écran « Réglages du jeu ») : tout est lu, et ni contenus ni mods ne se montrent.
  let {
    id,
    titre,
    plateforme,
    deplie = false,
    vide = $bindable(false),
  }: { id: number; titre: string; plateforme?: string; deplie?: boolean; vide?: boolean } = $props();
  let contenusLus = $state(false);
  let etatMods = $state<'attente' | 'vide' | 'visible'>('attente');

  let donnees = $state<ContenusDuJeu | null>(null);
  let ouvert = $state(false);
  let coches = $state<string[]>([]);
  let enCours = $state(false);
  // Lot 7 (Seb 17/11) : ce que Firehouse n'a pas (examiné en tâche de fond, jamais au lancement) et les envois en route.
  let envois = $state<ExamenEnvois | null>(null);
  let suivi = $state<SuiviEnvois | null>(null);
  let accordEnCours = $state(false);

  async function charger(jeu: number) {
    donnees = await api.contenusDuJeu(jeu).catch(() => null);
    if (jeu === id) contenusLus = true;
    coches = [];
    if (jeu === id && emulateurAutomatique(donnees?.systeme)) examinerEnvois(jeu);
  }

  /** En tâche de fond : le panneau est déjà affiché, la réponse de Firehouse vient ensuite (ou jamais, hors ligne). */
  async function examinerEnvois(jeu: number) {
    const e = await api.envoisSwitchExaminer(jeu).catch(() => null);
    if (jeu !== id) return;
    envois = e;
    suivi = e?.suivi ?? null;
    // « À l'avenir, envoyer tout seul » (choisi par la personne, désactivable dans les options) : pendant ce temps,
    // le bouton attend (jamais deux accords).
    if (e?.etat === 'ok' && e.a_envoyer.length && etat.profil.envoisFirehouse?.auto && !accordEnCours) {
      accordEnCours = true;
      try {
        await accepter(jeu, true, e);
      } finally {
        accordEnCours = false;
      }
    }
  }

  /** Met en file la liste que la personne a VUE (`vu`) : si elle a changé entre-temps, le cœur refuse et on la relit. */
  async function accepter(jeu: number, auto: boolean, vu: ExamenEnvois) {
    try {
      const n = await api.envoisSwitchAccepter(jeu, vu.a_envoyer.map((c) => c.nom), vu.total, auto);
      if (auto && n) toast(`📤 ${n} contenu(s) (${taille(vu.total)}) partent tout seuls vers Firehouse, en arrière-plan.`);
      if (jeu === id) {
        envois = envois ? { ...envois, a_envoyer: [] } : envois;
        suivi = await api.envoisSwitchSuivi(jeu).catch(() => suivi);
      }
    } catch (e) {
      if (estErreurCoeur(e) && e.sorte === 'conflit') {
        toast(`📤 ${motifDuRefus(e)}`, 'alerte');
        if (jeu === id && !auto) {
          const relu = await api.envoisSwitchExaminer(jeu).catch(() => null);
          if (jeu === id && relu) {
            envois = relu;
            suivi = relu.suivi;
          }
        }
        return;
      }
      // La proposition reste : la personne pourra réessayer.
      toast(`Envoi impossible : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  /** L'accord, chiffré (combien, combien de Go, combien de temps), avec « à l'avenir, tout seul ». */
  async function proposerEnvoi() {
    if (!envois || accordEnCours) return;
    accordEnCours = true;
    const vu = envois;
    try {
      const choix = await choisir<'oui' | 'auto' | 'non'>(
        titreAccord(vu),
        [
          { valeur: 'oui', libelle: '📤 Les envoyer' },
          { valeur: 'auto', libelle: '📤 Les envoyer, et à l’avenir envoyer tout seul', detail: 'Sans redemander (désactivable dans ⚙ Options ▸ 💾 Sauvegarde)' },
          { valeur: 'non', libelle: 'Pas maintenant' },
        ],
        messageAccord(vu),
      );
      if (choix !== 'oui' && choix !== 'auto') return;
      if (choix === 'auto') await reglerProfil('envoisFirehouse.auto', true);
      await accepter(id, choix === 'auto', vu);
    } finally {
      accordEnCours = false;
    }
  }

  /** « ✋ Arrêter l'envoi » : les envois pas finis de ce jeu sont retirés (ils pourront être reproposés). */
  let arretEnCours = $state(false);
  async function arreterEnvoi() {
    if (arretEnCours) return;
    arretEnCours = true;
    const jeu = id;
    try {
      await api.envoisSwitchAnnuler(jeu);
      toast('✋ Envoi arrêté : rien d’autre ne part pour ce jeu.');
      const relu = await api.envoisSwitchExaminer(jeu).catch(() => null);
      if (jeu === id) {
        envois = relu;
        suivi = relu?.suivi ?? (await api.envoisSwitchSuivi(jeu).catch(() => null));
      }
    } catch (e) {
      toast(`Impossible d’arrêter l’envoi : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      arretEnCours = false;
    }
  }

  $effect(() => {
    const jeu = id;
    donnees = null;
    contenusLus = false;
    ouvert = deplie;
    envois = null;
    suivi = null;
    charger(jeu);
  });

  // Pendant un envoi : relu toutes les 2 s (rien n'est demandé à Firehouse, seulement au cœur).
  $effect(() => {
    if (!envoisEnRoute(suivi)) return;
    const jeu = id;
    const minuterie = setInterval(async () => {
      const s = await api.envoisSwitchSuivi(jeu).catch(() => null);
      if (s && jeu === id) suivi = s;
    }, 2000);
    return () => clearInterval(minuterie);
  });

  // Ce que montre le bloc des contenus (mêmes conditions que le gabarit, plus bas).
  const contenusVisibles = $derived(
    donnees?.etat === 'emulateur' ||
      (donnees?.etat === 'ok' &&
        (emulateurAutomatique(donnees.systeme)
          ? !!(donnees.contenus.length || donnees.ignores?.length || donnees.remarque)
          : !!(donnees.contenus.length || donnees.ignores?.length))),
  );
  $effect(() => {
    vide = contenusLus && !contenusVisibles && (plateforme !== 'Nintendo Switch' || etatMods === 'vide');
  });

  const choisis = $derived(donnees?.contenus.filter((c) => coches.includes(c.id)) ?? []);
  const total = $derived(choisis.reduce((n, c) => n + c.taille, 0));
  // Ryubing ou Eden : donnés tout seuls à chaque partie, d'après les interrupteurs de la maison.
  const automatique = $derived(emulateurAutomatique(donnees?.systeme));
  // Pendant une partie ou un émulateur ouvert seul, les choix attendent (il réécrirait ses réglages par-dessus).
  const bloque = $derived(lancementImpossible());
  let basculement = $state<string | null>(null);

  async function basculer(c: ContenuVu) {
    // Pris AVANT la confirmation : un double appui n'ouvre jamais deux fenêtres ni deux enregistrements.
    if (!c.cle || bloque || basculement) return;
    basculement = c.cle;
    const actif = !c.actif;
    try {
      if (!actif) {
        const oui = await confirmer(`Éteindre « ${nomCourt(c.nom, titre)} » ?`, {
          message: [
            'Le jeu peut parfois refuser une partie faite avec ce contenu.',
            'Frogtend met tes parties de ce jeu à l’abri avant la prochaine partie.',
            'Ce choix vaut pour toute la maison, avec Eden comme avec Ryubing.',
          ].join('\n'),
          libelleValider: 'Éteindre',
        });
        if (!oui) return;
      }
      try {
        await api.contenusSwitchChoisir(id, c.cle, actif);
      } catch (e) {
        toast(`Impossible d'enregistrer : ${motifDuRefus(e)}`, 'erreur');
      }
      await relire();
    } finally {
      basculement = null;
    }
  }

  /** Relit la fiche après un choix ; en cas d'échec, l'ancienne liste reste (jamais un panneau vide). */
  async function relire() {
    try {
      donnees = await api.contenusDuJeu(id);
    } catch (e) {
      toast(`Impossible de relire les mises à jour et DLC : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  async function installer() {
    if (!choisis.length) return;
    const oui = await confirmer(`📦 Installer ${choisis.length} contenu(s) pour « ${titre} » ?`, {
      message: [
        `${taille(total)} à installer par RPCS3 (il travaille sans fenêtre, puis se ferme).`,
        ...choisis.slice(0, 8).map((c) => `• ${nomCourt(c.nom, titre)} (${libelleGenre(c.genre)})${c.licence && c.genre !== 'licence' ? ' — avec sa licence' : ''}`),
        choisis.length > 8 ? `… et ${choisis.length - 8} autre(s).` : '',
        'Les licences vont dans le compte RPCS3 de TON profil : un autre profil devra les installer pour lui.',
        'Ferme RPCS3 avant : Frogtend refuse d’installer pendant qu’il est ouvert.',
      ]
        .filter(Boolean)
        .join('\n'),
      libelleValider: 'Installer',
    });
    if (!oui) return;
    enCours = true;
    try {
      const b = await api.contenusInstaller(id, choisis.map((c) => c.id));
      const deja = b.deja ? ` (${b.deja} mise(s) à jour déjà dépassée(s) par celle installée : rien à refaire)` : '';
      toast(
        b.refuses.length
          ? `📦 ${b.installes} installé(s)${deja}, ${b.refuses.length} refusé(s) : ${b.refuses.map(([n, m]) => `${nomCourt(n, titre)} (${m})`).join(', ')}`
          : `✅ ${b.installes} contenu(s) installé(s) pour « ${titre} »${deja}.`,
        b.refuses.length ? 'alerte' : 'ok',
      );
      await charger(id);
    } catch (e) {
      toast(`Installation impossible : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      enCours = false;
    }
  }
</script>

{#if donnees?.etat === 'emulateur'}
  <p class="muted">📦 Règle {donnees.systeme === 'switch' ? 'Eden ou Ryubing' : 'RPCS3'} (⚙ Options ▸ Émulateurs) pour gérer les mises à jour et DLC de ce jeu.</p>
{:else if donnees?.etat === 'ok' && automatique && (donnees.contenus.length || donnees.ignores?.length || donnees.remarque)}
  <!-- Ryubing et Eden (Seb 16-17/11) : utilisés seuls au lancement, d'après un interrupteur par contenu (toute la maison). -->
  <div class="contenus cx-block">
    {#if donnees.contenus.length}
      <button class="btn" aria-expanded={ouvert} onclick={() => (ouvert = !ouvert)}>
        📦 Mises à jour et DLC : <span aria-live="polite">{resumeChoix(donnees.contenus, donnees.systeme)}</span> {ouvert ? '▴' : '▾'}
      </button>
    {/if}
    {#each lignesRemarque(donnees.remarque) as l (l)}<p class="muted">{l}</p>{/each}
    {#if donnees.ignores?.length}
      <p class="muted">
        ⚠ {donnees.ignores.length} fichier(s) illisible(s), peut-être abîmé(s) au téléchargement : {quelquesNoms(
          donnees.ignores.map((n) => nomCourt(n, titre)),
        )}. Pour les réparer : retire le jeu de ta ludothèque puis remets-le (ses parties sont gardées).
      </p>
    {/if}
    <!-- Lot 7 (Seb 17/11) : ce que Firehouse n'a pas, l'accord, puis où en est l'envoi. -->
    {#if ligneProposition(envois)}
      <div class="actions envoi">
        <span>{ligneProposition(envois)}</span>
        <button class="btn petit" onclick={proposerEnvoi} disabled={accordEnCours}>Les envoyer…</button>
      </div>
    {/if}
    {#each (envois?.etat === 'ok' ? envois.ecartes : []).slice(0, 5) as e, i (i)}<p class="muted">{ligneEcarte(e)}</p>{/each}
    {#if envois?.etat === 'ok' && envois.ecartes.length > 5}<p class="muted">… et {envois.ecartes.length - 5} autre(s) pas envoyé(s).</p>{/if}
    <!-- Le lecteur d'écran n'annonce que les changements d'ÉTAPE, jamais le pourcentage qui bouge toutes les 2 s. -->
    <span class="annonce" aria-live="polite">{etapeAnnoncee(suivi?.en_cours)}</span>
    {#if suivi?.en_cours}<p>{ligneProgres(suivi.en_cours)}</p>{/if}
    {#each (suivi?.envois ?? []).filter((e) => !(suivi?.en_cours && e.nom === suivi.en_cours.nom)) as e, i (i)}
      <p class="muted">{ligneEnvoi(e, !!suivi?.arret)}</p>
    {/each}
    {#if suivi?.arret}<p class="muted">⚠ {suivi.arret}</p>{/if}
    {#if envoiArretable(suivi)}
      <div class="actions">
        <button class="btn petit" onclick={arreterEnvoi} disabled={arretEnCours}>✋ Arrêter l’envoi</button>
      </div>
    {/if}
    {#if ouvert && donnees.contenus.length}
      <p class="muted">{PHRASE_CHOIX}</p>
      {#if dlcEnPartieEden(donnees.systeme, donnees.contenus)}<p class="muted">{REMARQUE_EDEN}</p>{/if}
      {#if bloque}<p class="muted">{motifInterrupteursBloques(attenteEmulateurSeul())}</p>{/if}
      <ul class="grille">
        {#each donnees.contenus as c (c.id)}
          <li>
            <!-- Panneau étroit : le nom prend la place, l'interrupteur à droite, le détail dessous sur toute la largeur
                 (capture de Seb, 18/11 : « Fire Emblem Three Houses » coupé sur 3 lignes à côté de l'interrupteur). -->
            <span class="nom">{nomCourt(c.nom, titre)}</span>
            {#if c.cle}
              <!-- Un interrupteur : son texte dit l'état (jamais la couleur seule). -->
              <!-- Celui qui s'enregistre garde le focus (aria-disabled) ; les AUTRES sont désactivés pendant ce temps. -->
              <button
                class="btn petit"
                role="switch"
                aria-checked={!!c.actif}
                aria-label={nomCourt(c.nom, titre)}
                aria-describedby={`etat-${c.cle}`}
                aria-disabled={bloque || basculement !== null}
                disabled={bloque || (basculement !== null && basculement !== c.cle)}
                onclick={() => basculer(c)}
              >
                <!-- Le texte de l'état, lu aussi par un lecteur d'écran (aria-label ne porte que le nom). -->
                <span id={`etat-${c.cle}`}>{texteInterrupteur(c, donnees.systeme, donnees.contenus)}</span>
              </button>
            {:else}
              <!-- Pas d'interrupteur : la raison est écrite dans le détail, dessous (jamais un title seul). -->
              <span aria-hidden="true">—</span>
            {/if}
            <span class="muted detail">{libelleGenre(c.genre)} · {taille(c.taille)}{c.raison ? ` · ${raisonNonPose(c)}` : ''}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{:else if donnees?.etat === 'ok' && (donnees.contenus.length || donnees.ignores?.length)}
  <div class="contenus cx-block">
    {#if donnees.ignores?.length}
      <p class="muted">
        ⚠ {donnees.ignores.length} fichier(s) de contenus illisible(s), peut-être abîmé(s) au téléchargement : {quelquesNoms(
          donnees.ignores,
        )}. Pour les réparer : retire le jeu de ta ludothèque puis remets-le (ses parties sont gardées).
      </p>
    {/if}
    <button class="btn" onclick={() => (ouvert = !ouvert)}>📦 Contenus additionnels : {resume(donnees.contenus)} {ouvert ? '▴' : '▾'}</button>
    {#if ouvert && donnees.systeme === 'wiiu'}
      <p class="muted">Inclus dans le fichier .wua du jeu : Cemu les charge tout seul, rien à installer.</p>
      <ul>
        {#each donnees.contenus as c (c.id)}<li><span>✅</span><span class="nom">{c.nom}</span></li>{/each}
      </ul>
    {:else if ouvert}
      <ul>
        {#each donnees.contenus as c (c.id)}
          <li>
            {#if c.installe}
              <span title="Déjà installé pour ton profil">✅</span>
              <span class="nom">{nomCourt(c.nom, titre)}</span>
            {:else}
              <!-- La case ET son nom : toute la ligne se clique (cible assez grande, souris comme manette). -->
              <label class="choix">
                <input
                  type="checkbox"
                  bind:group={coches}
                  value={c.id}
                  onchange={() => donnees && (coches = cochesCoherentes(donnees.contenus, coches, c.id))}
                />
                <span class="nom">{nomCourt(c.nom, titre)}</span>
              </label>
            {/if}
            <span class="muted">{libelleGenre(c.genre)} · {taille(c.taille)}{c.licence ? ' · licence' : ''}</span>
          </li>
        {/each}
      </ul>
      <div class="actions">
        <button class="btn petit" onclick={() => (coches = donnees!.contenus.filter((c) => !c.installe).map((c) => c.id))}>Tout cocher</button>
        <button class="btn petit primary" onclick={installer} disabled={enCours || !choisis.length}>
          {enCours ? 'Installation…' : `📦 Installer (${choisis.length}, ${taille(total)})`}
        </button>
      </div>
    {/if}
  </div>
{/if}
{#if plateforme === 'Nintendo Switch'}<BlocMods {id} {titre} {deplie} bind:etatBloc={etatMods} />{/if}

<style>
  .contenus {
    display: grid;
    gap: calc(6 * var(--u));
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: calc(4 * var(--u));
    max-height: calc(260 * var(--u));
    overflow: auto;
  }
  li {
    display: flex;
    gap: calc(6 * var(--u));
    align-items: center;
    flex-wrap: wrap;
  }
  .nom {
    flex: 1;
    overflow-wrap: anywhere;
  }
  /* Switch, panneau étroit (~300 u utiles) : nom | interrupteur, puis le détail sur toute la largeur. */
  .grille li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: calc(2 * var(--u)) calc(8 * var(--u));
  }
  .grille .detail {
    grid-column: 1 / -1;
  }
  .choix {
    display: flex;
    flex: 1;
    gap: calc(6 * var(--u));
    align-items: center;
    min-height: var(--cible-min);
    cursor: pointer;
  }
  .actions {
    display: flex;
    gap: calc(6 * var(--u));
    flex-wrap: wrap;
  }
  .envoi {
    align-items: center;
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
