<script lang="ts">
  // 🧩 Les mods posés par VORTEX dans un jeu PC (lot M1, ✅ Seb 25/11) : ce qui ne va plus (diagnostic), les retirer
  // (rangés dans le dossier du jeu, jamais effacés ; parties à l'abri avant), en remettre un, ou tout remettre. Le même
  // bloc sert au bureau (panneau du jeu, fermé d'office) et au canapé (Taodbox, « Réglages du jeu », déplié) : tout se
  // fait au clavier, à la souris ou à la manette, avec les fenêtres maison. Jamais un chemin ni un mot technique.
  import { api, taille, type EtatVortex, type ModVortex } from '$lib/api';
  import { cibleDuFocus } from '$lib/contenus/mods';
  import { confirmer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { lancementImpossible } from '$lib/ludotheque/jeu.svelte';
  import { avecAbriAvant } from '$lib/parties/abriAvant';
  import { auCanape } from '$lib/taodbox/canape';
  import { selectionner } from '$lib/taodbox/manette.svelte';
  import { tick, untrack } from 'svelte';
  import { avecProgres, type DepartProgres } from './progres.svelte';
  import {
    autresMods,
    blocVortexVisible,
    chercher,
    compteRecherche,
    detailMod,
    encoreOuvert,
    enteteVortex,
    libelleRetrait,
    ligneInterrompu,
    lignesDiagnostic,
    lignesRetrait,
    ligneVortexRevenu,
    messageApresRemise,
    messageApresRetraitMod,
    messageApresToutRanger,
    messageRemise,
    messageRetrait,
    messageRetraitMod,
    messageToutRanger,
    retraitModComplet,
    messageToutRemettre,
    modsEnCause,
    modsEnPlace,
    motifVerrou,
    PAQUET,
    titreRetrait,
    titreRetraitMod,
    toastRetrait,
    trier,
  } from './vortex';

  let { id, titre, deplie = false }: { id: number; titre: string; deplie?: boolean } = $props();
  /** L'identifiant du motif d'un verrou : chaque bouton grisé le désigne (`aria-describedby`). */
  const uid = $props.id();
  const idMotif = `vortex-motif-${uid}`;

  let e = $state<EtatVortex | null>(null);
  let charge = $state(false);
  let ouvert = $state(false);
  /** Une opération (retrait, remise…) est en cours depuis ce bloc (posé APRÈS le oui de la personne). */
  let occupe = $state(false);
  /** Une fenêtre de confirmation est ouverte (un second appui n'en ouvre jamais une autre). */
  let question = false;
  /** « Voir les N autres mods » (avant le retrait). */
  let autresVus = $state(false);
  let limiteAutres = $state(PAQUET);
  let limiteRetires = $state(PAQUET);
  let limiteRemis = $state(PAQUET);
  let recherche = $state('');
  let rechercheRetires = $state('');
  let rechercheRemis = $state('');
  /** Ce que le lecteur d'écran entend : l'étape, jamais le pourcentage. */
  let annonce = $state('');
  let bloc = $state<HTMLElement | null>(null);
  let entete = $state<HTMLButtonElement | null>(null);
  let boutonReprendre = $state<HTMLButtonElement | null>(null);
  let boutonRetirer = $state<HTMLButtonElement | null>(null);
  let boutonVerifier = $state<HTMLButtonElement | null>(null);

  const canape = auCanape();
  // Pendant une partie ou un émulateur ouvert seul, pendant une opération, ou quand le cœur le dit (Galaxy ou Vortex
  // ouvert, jeu lancé hors de Frogtend).
  const partieEnCours = $derived(lancementImpossible());
  const motif = $derived(motifVerrou(e?.verrou ?? (partieEnCours ? 'partie' : null), occupe));
  const verrouille = $derived(motif !== null);
  const enCours = $derived(!!e?.retrait?.en_cours);
  const version = $derived(e?.diagnostic.version ?? null);
  // B1 (relecture UI) : le diagnostic ne se dit que s'il reste des mods de Vortex dans le jeu.
  const enPlace = $derived(!!e && modsEnPlace(e));
  const enCause = $derived(e && enPlace ? modsEnCause(e, e.mods) : []);
  const autres = $derived(e ? (enPlace ? autresMods(e, e.mods) : trier(e.mods)) : []);
  // La recherche ne porte que sur les AUTRES mods (un mod en cause, déjà montré, n'est jamais en double).
  const trouves = $derived(recherche.trim() ? chercher(autres, recherche) : []);
  const retires = $derived(e ? trier(e.retires) : []);
  const retiresMontres = $derived(rechercheRetires.trim() ? chercher(retires, rechercheRetires) : retires);
  // 0.227.1 (défaut vu par Seb le 25/11) : les mods REMIS par Frogtend, « 🧩 Dans le jeu », chacun retirable de nouveau.
  const remis = $derived(e ? trier(e.remis ?? []) : []);
  const remisMontres = $derived(rechercheRemis.trim() ? chercher(remis, rechercheRemis) : remis);
  // Sans manifeste de Vortex dans le jeu (son dépôt effacé, ou tout remis), plus de retrait global : « Tout ranger ».
  const peutToutRanger = $derived(!!e && !e.present && !enCours && remis.length > 0);
  // Le motif d'un verrou ne se dit qu'UNE fois, là où sont les boutons grisés : collé aux listes (« Dans le jeu »,
  // « Mods retirés ») quand il n'y a plus qu'elles à faire ; sinon en tête de la partie dépliée (retrait, reprise).
  const motifAuxListes = $derived(!!motif && !!e && !enCours && !(e.present && e.mods.length > 0) && (retires.length > 0 || remis.length > 0));
  const decrit = $derived(verrouille ? idMotif : undefined);
  const peutRetirer = $derived(!!e && e.present && !e.refus && !enCours && e.mods.length > 0);
  // Au canapé, les lignes se visent à la manette pour faire défiler la liste.
  const tabLigne = $derived(canape ? 0 : -1);

  $effect(() => {
    const jeu = id;
    e = null;
    charge = false;
    ouvert = deplie;
    autresVus = false;
    limiteAutres = PAQUET;
    limiteRetires = PAQUET;
    limiteRemis = PAQUET;
    recherche = '';
    rechercheRetires = '';
    rechercheRemis = '';
    annonce = '';
    charger(jeu);
  });

  // Une partie finie : l'état est relu (le jeu a pu écrire, Galaxy a pu s'ouvrir). Seulement au passage « en cours » →
  // « finie » (une valeur simple, jamais un objet dérivé : leçon du 28/10).
  let partieAvant = false;
  $effect(() => {
    const p = partieEnCours;
    if (partieAvant && !p) untrack(() => relire());
    partieAvant = p;
  });

  /** L'état lu au cœur ; un échec (même immédiat) ne casse jamais le panneau : `fautif` est rendu. */
  async function lireEtat<T>(jeu: number, fautif: T): Promise<EtatVortex | null | T> {
    try {
      return await api.modsVortexEtat(jeu);
    } catch {
      return fautif;
    }
  }

  async function charger(jeu: number) {
    const x = await lireEtat(jeu, null);
    if (jeu !== id) return;
    e = x;
    charge = true;
    // Au canapé (déplié), un retrait interrompu : « ▶ Reprendre » reçoit le focus d'office.
    if (deplie && x?.retrait?.en_cours) {
      await tick();
      if (boutonReprendre) selectionner(boutonReprendre);
    }
  }

  /** Relit l'état (après chaque action) ; en cas d'échec, l'ancien reste (jamais un bloc vide). */
  async function relire() {
    const jeu = id;
    const x = await lireEtat(jeu, undefined);
    if (x !== undefined && jeu === id) e = x;
  }

  async function basculer() {
    ouvert = !ouvert;
    if (ouvert && enCours) {
      await tick();
      if (boutonReprendre) selectionner(boutonReprendre);
    }
  }

  /** « 🔁 C'est fait, revérifier » : l'état relu ; s'il bloque encore, la personne le sait ; sinon le focus va à ce
   * qu'elle voulait faire (le bouton de retrait, sinon l'en-tête). */
  async function reverifier() {
    // Le motif était collé aux listes : le focus ira au premier bouton actif des listes (sinon à l'en-tête).
    const auxListes = motifAuxListes;
    await relire();
    await tick();
    const reste = encoreOuvert(e?.verrou ?? null);
    if (reste) {
      toast(reste, 'alerte');
      if (boutonVerifier?.isConnected) selectionner(boutonVerifier);
      return;
    }
    const premierDesListes = bloc?.querySelector<HTMLButtonElement>('.listes button:not(:disabled)') ?? null;
    const cible = auxListes ? (premierDesListes ?? entete) : (boutonReprendre ?? boutonRetirer ?? entete);
    if (cible?.isConnected) selectionner(cible);
  }

  /** L'ordre des lignes quand l'action a commencé (pour trouver « la suivante » d'une ligne disparue). */
  let ordre: string[] = [];

  /** Après une action, l'élément qui avait le focus a pu disparaître : il revient à la ligne (ou à la suivante, sinon à
   * l'en-tête), jamais en haut de la page. */
  async function rendreFocus(cle: string) {
    await tick();
    const a = document.activeElement;
    if (a && a !== document.body && a.isConnected) return;
    const lignes = [...(bloc?.querySelectorAll<HTMLElement>('li[data-cle]') ?? [])];
    const cible = cibleDuFocus(lignes, cle, ordre, entete);
    if (cible) selectionner(cible);
  }

  /** Une opération : les parties à l'abri d'abord (la personne choisit si c'est impossible), puis la progression. */
  async function operer<T>(message: string, faire: (suivi: number) => Promise<T>, depart?: DepartProgres): Promise<T | undefined> {
    return avecAbriAvant(id, 'avant_mod', () => avecProgres(message, false, faire, depart), { jeuPc: true });
  }

  const fichiersDe = (l: ModVortex[]) => l.reduce((n, m) => n + m.fichiers, 0);

  /** Le chemin commun : la question (rien n'est verrouillé avant le oui), puis l'opération, l'état relu, le focus
   * rendu — aussi après « Annuler » ou un abri abandonné. */
  async function avecAccord(cle: string, demander: () => Promise<boolean>, faire: () => Promise<void>) {
    if (question || occupe || verrouille) return;
    question = true;
    try {
      if (!(await demander())) return;
      occupe = true;
      try {
        await faire();
      } catch (x) {
        annonce = '';
        toast(`Impossible : ${motifDuRefus(x)}`, 'erreur');
      } finally {
        occupe = false;
        await relire();
      }
    } finally {
      question = false;
      await rendreFocus(cle);
    }
  }

  function retirer() {
    if (!e || !peutRetirer) return;
    const vu = e;
    ordre = [];
    return avecAccord(
      '',
      () => confirmer(titreRetrait(vu), { message: messageRetrait(vu, taille), danger: true, libelleValider: '🧹 Retirer les mods' }),
      async () => {
        annonce = 'Retrait des mods de Vortex en cours.';
        const b = await operer('🧹 Retrait des mods de Vortex (ne ferme pas Frogtend)', (s) => api.modsVortexRetirer(id, s), { etape: 'verification', total: fichiersDe(vu.mods) });
        if (b === undefined) {
          annonce = '';
          return;
        }
        toast(toastRetrait(b), 'ok');
        annonce = 'Mods retirés.';
      },
    );
  }

  function reprendre() {
    if (!enCours || !e?.retrait) return;
    ordre = [];
    // La barre part du vrai compte : les fichiers déjà rangés avant la coupure.
    const depart: DepartProgres = { etape: 'retrait', total: e.retrait.total - e.retrait.laisses, faits: Math.max(0, e.retrait.faits - e.retrait.laisses) };
    return avecAccord(
      '',
      async () => true,
      async () => {
        annonce = 'Reprise du retrait des mods.';
        const b = await operer('🧹 Retrait des mods de Vortex (ne ferme pas Frogtend)', (s) => api.modsVortexReprendre(id, s), depart);
        if (b === undefined) {
          annonce = '';
          return;
        }
        toast(toastRetrait(b), 'ok');
        annonce = 'Mods retirés.';
      },
    );
  }

  function remettre(m: ModVortex) {
    if (enCours) return;
    ordre = retiresMontres.map((x) => `retire:${x.source}`);
    return avecAccord(
      `retire:${m.source}`,
      () => confirmer(`↩ Remettre « ${m.nom} » ?`, { message: messageRemise(m, version), libelleValider: '↩ Remettre' }),
      async () => {
        annonce = `Remise de ${m.nom}.`;
        const b = await operer(`↩ Remise de « ${m.nom} » (ne ferme pas Frogtend)`, (s) => api.modsVortexRemettre(id, m.source, s), { etape: 'remise', total: m.fichiers });
        if (b === undefined) {
          annonce = '';
          return;
        }
        toast(messageApresRemise(b, m.nom), b.places_prises || b.changes ? 'alerte' : 'ok');
        annonce = 'Mod remis.';
      },
    );
  }

  function toutRemettre() {
    if (!e) return;
    const n = e.retires.length;
    const interrompu = enCours;
    const depart: DepartProgres = { etape: 'remise', total: fichiersDe(e.retires) };
    ordre = [];
    return avecAccord(
      '',
      () => confirmer('↩ Tout remettre comme avant ?', { message: messageToutRemettre(interrompu, n, version), libelleValider: '↩ Tout remettre' }),
      async () => {
        annonce = 'Remise des mods en cours.';
        const b = await operer('↩ Remise des mods dans le jeu (ne ferme pas Frogtend)', (s) => api.modsVortexToutRemettre(id, s), depart);
        if (b === undefined) {
          annonce = '';
          return;
        }
        toast(messageApresRemise(b, null), b.places_prises || b.changes ? 'alerte' : 'ok');
        annonce = 'Mods remis.';
      },
    );
  }

  /** 🧹 Retire de nouveau UN mod remis (partie « 🧩 Dans le jeu ») : le même enchaînement que les autres actions. */
  function retirerMod(m: ModVortex) {
    if (enCours) return;
    ordre = remisMontres.map((x) => `jeu:${x.source}`);
    return avecAccord(
      `jeu:${m.source}`,
      () => confirmer(titreRetraitMod(m), { message: messageRetraitMod(m), danger: true, libelleValider: '🧹 Retirer' }),
      async () => {
        annonce = `Retrait de ${m.nom}.`;
        const b = await operer(`🧹 Retrait de « ${m.nom} » (ne ferme pas Frogtend)`, (s) => api.modsVortexRetirerMod(id, m.source, s), { etape: 'retrait', total: m.fichiers });
        if (b === undefined) {
          annonce = '';
          return;
        }
        toast(messageApresRetraitMod(b, m.nom), retraitModComplet(b) ? 'ok' : 'alerte');
        annonce = 'Mod retiré.';
      },
    );
  }

  /** 🧹 « Tout ranger de nouveau » : chaque mod remis (sans le dépôt de Vortex, plus de retrait global possible). */
  function toutRanger() {
    if (!peutToutRanger) return;
    const n = remis.length;
    const total = fichiersDe(remis);
    ordre = [];
    return avecAccord(
      '',
      () => confirmer(n === 1 ? '🧹 Ranger de nouveau le mod remis ?' : `🧹 Ranger de nouveau les ${n.toLocaleString('fr-FR')} mods remis ?`, { message: messageToutRanger(n), danger: true, libelleValider: '🧹 Tout ranger' }),
      async () => {
        annonce = 'Rangement des mods remis en cours.';
        const b = await operer('🧹 Rangement des mods remis (ne ferme pas Frogtend)', (s) => api.modsVortexToutRanger(id, s), { etape: 'retrait', total });
        if (b === undefined) {
          annonce = '';
          return;
        }
        toast(messageApresToutRanger(b), retraitModComplet(b) ? 'ok' : 'alerte');
        annonce = 'Mods rangés.';
      },
    );
  }

  const plus = (montres: number, total: number) => Math.min(PAQUET, total - montres);
</script>

{#snippet verrou()}
  <!-- Le motif d'un verrou (une seule fois à l'écran) : les boutons grisés le désignent. -->
  <p class="bord err" id={idMotif}>{motif}</p>
  {#if e?.verrou}
    <div class="actions"><button class="btn petit" bind:this={boutonVerifier} onclick={reverifier} disabled={occupe}>🔁 C’est fait, revérifier</button></div>
  {/if}
{/snippet}

{#snippet ligneMod(m: ModVortex)}
  <!-- Au bureau `tabindex="-1"` (le focus y revient après une action) ; au canapé `0`, pour défiler à la manette. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <li data-cle={m.source} tabindex={tabLigne}>
    <span class="nom">{m.nom}</span>
    <span class="muted detail">{detailMod(m)}</span>
  </li>
{/snippet}

{#if charge && blocVortexVisible(e) && e}
  <div class="vortex cx-block" bind:this={bloc} aria-busy={occupe}>
    <button class="btn entete" bind:this={entete} aria-expanded={ouvert} onclick={basculer}>
      🧩 Mods : {enteteVortex(e)} {ouvert ? '▴' : '▾'}
    </button>
    <!-- Le lecteur d'écran n'annonce que l'ÉTAPE puis la fin, jamais le pourcentage qui bouge. -->
    <span class="annonce" aria-live="polite">{annonce}</span>
    {#if ouvert}
      {#if motif && !motifAuxListes}{@render verrou()}{/if}
      {#if e.refus}<p class="bord warn">{e.refus}</p>{/if}
      {#if ligneVortexRevenu(e)}<p class="bord warn">{ligneVortexRevenu(e)}</p>{/if}

      {#if enCours && e.retrait}
        <p class="bord warn">{ligneInterrompu(e.retrait)}</p>
        <div class="actions">
          <button class="btn petit" bind:this={boutonReprendre} onclick={reprendre} disabled={verrouille} aria-describedby={decrit}>▶ Reprendre le retrait</button>
          <button class="btn petit" onclick={toutRemettre} disabled={verrouille} aria-describedby={decrit}>↩ Tout remettre comme avant</button>
        </div>
      {:else}
        {#if enPlace}
          {@const diag = lignesDiagnostic(e.diagnostic, titre)}
          {#if diag.length}
            <div class="bord warn diagnostic">
              {#each diag as l, i (i)}<p class:alerte={i === 0 && l.startsWith('⚠')}>{l}</p>{/each}
            </div>
          {/if}
          {#if enCause.length}
            <p class="sous-titre">Les mods en cause</p>
            <ul>
              {#each enCause as m (m.source)}{@render ligneMod(m)}{/each}
            </ul>
          {/if}
        {/if}

        {#if e.present && e.mods.length}
          {#if peutRetirer}
            <div class="actions">
              <button class="btn petit danger" bind:this={boutonRetirer} onclick={retirer} disabled={verrouille} aria-describedby={decrit}>{libelleRetrait(e)}</button>
            </div>
          {/if}
          {#if autres.length}
            <button class="btn petit lien-liste" aria-expanded={autresVus} onclick={() => (autresVus = !autresVus)}>
              {autresVus ? 'Masquer les autres mods' : enCause.length ? `Voir les ${autres.length.toLocaleString('fr-FR')} autres mods` : `Voir les ${autres.length.toLocaleString('fr-FR')} mods`}
            </button>
          {/if}
          {#if autresVus}
            {#if !canape}
              <input class="filtre" type="search" placeholder="🔎 Chercher…" aria-label="Chercher un mod" bind:value={recherche} />
              {#if recherche.trim()}<p class="muted">{compteRecherche(trouves.length, autres.length)}</p>{/if}
            {/if}
            {@const montres = recherche.trim() ? trouves : autres}
            <ul>
              {#each montres.slice(0, limiteAutres) as m (m.source)}{@render ligneMod(m)}{/each}
            </ul>
            {#if montres.length > limiteAutres}
              <button class="btn petit" onclick={() => (limiteAutres += PAQUET)}>Voir {plus(limiteAutres, montres.length)} de plus</button>
            {/if}
          {/if}
        {/if}

        {#if retires.length && e.retrait}
          {#each lignesRetrait({ mods: retires.length, laisses: e.retrait.laisses, laisses_rompus: e.retrait.laisses_rompus, sauvegardes_vortex: e.retrait.sauvegardes_vortex }) as l, i (i)}
            <p class:muted={i > 0 && !l.startsWith('⚠') && !l.startsWith('Désinstalle')}>{l}</p>
          {/each}
        {/if}
        {#if motifAuxListes}{@render verrou()}{/if}
        <div class="listes">
        {#if remis.length}
          <p class="sous-titre">🧩 Dans le jeu ({remis.length.toLocaleString('fr-FR')})</p>
          {#if !canape && remis.length > PAQUET}
            <input class="filtre" type="search" placeholder="🔎 Chercher…" aria-label="Chercher un mod remis" bind:value={rechercheRemis} />
            {#if rechercheRemis.trim()}<p class="muted">{compteRecherche(remisMontres.length, remis.length)}</p>{/if}
          {/if}
          <ul>
            {#each remisMontres.slice(0, limiteRemis) as m (m.source)}
              <!-- Au canapé, la ligne se vise à la manette (même tout grisé) ; au bureau, le focus y revient. -->
              <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
              <li data-cle={`jeu:${m.source}`} tabindex={tabLigne}>
                <span class="nom">{m.nom}</span>
                <span class="action">
                  <button class="btn petit" aria-label={`Retirer ${m.nom}`} onclick={() => retirerMod(m)} disabled={verrouille} aria-describedby={decrit}>🧹 Retirer</button>
                </span>
                <span class="muted detail">{detailMod(m)}</span>
              </li>
            {/each}
          </ul>
          {#if remisMontres.length > limiteRemis}
            <button class="btn petit" onclick={() => (limiteRemis += PAQUET)}>Voir {plus(limiteRemis, remisMontres.length)} de plus</button>
          {/if}
          {#if peutToutRanger}
            <div class="actions">
              <button class="btn petit" onclick={toutRanger} disabled={verrouille} aria-describedby={decrit}>🧹 Tout ranger de nouveau</button>
            </div>
          {/if}
        {/if}
        {#if retires.length}
          <p class="sous-titre">🧩 Mods retirés ({retires.length.toLocaleString('fr-FR')})</p>
          {#if !canape && retires.length > PAQUET}
            <input class="filtre" type="search" placeholder="🔎 Chercher…" aria-label="Chercher un mod retiré" bind:value={rechercheRetires} />
            {#if rechercheRetires.trim()}<p class="muted">{compteRecherche(retiresMontres.length, retires.length)}</p>{/if}
          {/if}
          <ul>
            {#each retiresMontres.slice(0, limiteRetires) as m (m.source)}
              <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
              <li data-cle={`retire:${m.source}`} tabindex={tabLigne}>
                <span class="nom">{m.nom}</span>
                <span class="action">
                  <button class="btn petit" aria-label={`Remettre ${m.nom}`} onclick={() => remettre(m)} disabled={verrouille} aria-describedby={decrit}>↩ Remettre</button>
                </span>
                <span class="muted detail">{detailMod(m)}</span>
              </li>
            {/each}
          </ul>
          {#if retiresMontres.length > limiteRetires}
            <button class="btn petit" onclick={() => (limiteRetires += PAQUET)}>Voir {plus(limiteRetires, retiresMontres.length)} de plus</button>
          {/if}
          <div class="actions">
            <button class="btn petit" onclick={toutRemettre} disabled={verrouille} aria-describedby={decrit}>↩ Tout remettre comme avant</button>
          </div>
        {/if}
        </div>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .vortex {
    display: grid;
    gap: calc(6 * var(--u));
  }
  .entete {
    justify-content: flex-start;
    text-align: left;
    overflow-wrap: anywhere;
  }
  p {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .alerte {
    font-weight: 600;
  }
  /* Un paragraphe d'alerte : une bordure à gauche, aux couleurs du thème (jamais une couleur en dur). */
  .bord {
    padding: calc(6 * var(--u)) calc(10 * var(--u));
    border-left: calc(3 * var(--u)) solid var(--line);
    border-radius: calc(4 * var(--u));
  }
  .bord.warn {
    border-left-color: var(--warn);
    background: color-mix(in srgb, var(--warn) 10%, transparent);
  }
  .bord.err {
    border-left-color: var(--err);
    background: color-mix(in srgb, var(--err) 10%, transparent);
  }
  .diagnostic {
    display: grid;
    gap: calc(4 * var(--u));
  }
  .sous-titre {
    font-weight: 600;
    margin-top: calc(4 * var(--u));
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: calc(6 * var(--u));
  }
  /* Panneau étroit (~300 u) : le nom prend la place, le bouton à droite, le nombre de fichiers dessous. */
  li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: calc(2 * var(--u)) calc(8 * var(--u));
    align-items: center;
    padding-bottom: calc(6 * var(--u));
    border-bottom: 1px solid var(--line);
    min-height: var(--cible-min);
  }
  li:last-child {
    border-bottom: none;
  }
  li:focus {
    outline: calc(2 * var(--u)) solid var(--accent);
    outline-offset: calc(2 * var(--u));
  }
  .nom {
    font-weight: 600;
    overflow-wrap: anywhere;
    min-width: 0;
  }
  .detail {
    grid-column: 1 / -1;
  }
  .actions {
    display: flex;
    gap: calc(6 * var(--u));
    flex-wrap: wrap;
  }
  .listes {
    display: grid;
    gap: calc(6 * var(--u));
  }
  .lien-liste {
    justify-self: start;
  }
  /* Les petits boutons du bloc : la cible garde 40 px au moins (charte § 4). */
  .petit {
    min-height: var(--cible-min);
    padding: calc(5 * var(--u)) calc(11 * var(--u));
    font-size: calc(12 * var(--u));
    overflow-wrap: anywhere;
  }
  .filtre {
    min-height: var(--cible-min);
    width: 100%;
    box-sizing: border-box;
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
</style>
