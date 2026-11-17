<script lang="ts">
  // 📦 Les contenus additionnels d'un jeu du PC : trouvés sur le disque. PS3 : cochés par la personne, installés
  // seulement après son oui (taille annoncée). Switch, Ryubing et Eden (Seb 16/11 : « pour tout ce qui est update, DLC,
  // on peut passer en automatique ») : utilisés tout seuls à chaque partie ; depuis le 17/11, Frogtend est la source de
  // vérité — un interrupteur par contenu, allumé d'office, pour ce jeu et toute la maison. Wii U : inclus dans le .wua.
  import { api, taille } from '$lib/api';
  import { confirmer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
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

  let { id, titre }: { id: number; titre: string } = $props();

  let donnees = $state<ContenusDuJeu | null>(null);
  let ouvert = $state(false);
  let coches = $state<string[]>([]);
  let enCours = $state(false);

  async function charger(jeu: number) {
    donnees = await api.contenusDuJeu(jeu).catch(() => null);
    coches = [];
  }

  $effect(() => {
    const jeu = id;
    donnees = null;
    ouvert = false;
    charger(jeu);
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
    {#if ouvert && donnees.contenus.length}
      <p class="muted">{PHRASE_CHOIX}</p>
      {#if dlcEnPartieEden(donnees.systeme, donnees.contenus)}<p class="muted">{REMARQUE_EDEN}</p>{/if}
      {#if bloque}<p class="muted">{motifInterrupteursBloques(attenteEmulateurSeul())}</p>{/if}
      <ul>
        {#each donnees.contenus as c (c.id)}
          <li>
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
              <span title={raisonNonPose(c)}>—</span>
            {/if}
            <span class="nom">{nomCourt(c.nom, titre)}</span>
            <span class="muted">{libelleGenre(c.genre)} · {taille(c.taille)}{c.raison ? ` · ${raisonNonPose(c)}` : ''}</span>
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
  p {
    margin: 0;
  }
</style>
