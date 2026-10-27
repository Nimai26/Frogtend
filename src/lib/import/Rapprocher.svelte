<script lang="ts">
  // 🔗 Reconnaître mes jeux importés (contrat 1.21, besoin 28 ; ✅ Seb 20/10). Annoncé et chiffré d'abord (fichiers,
  // poids, durée) ; tes fichiers sont lus sans rien modifier ; seuls les jeux reconnus à coup sûr sont reliés tout
  // seuls ; le reste t'est montré. Relier un jeu ne change ni ses fichiers ni son temps de jeu, et se défait.
  import { nomAffiche } from '$lib/ludotheque/ludotheque.svelte';
  import { onMount, tick } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { api, duree, taille } from '$lib/api';
  import { choisir, confirmer, fermerToast, majProgres, toast, toastProgres } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { rechargerListe, rechargerPlateformes } from '$lib/ludotheque/ludotheque.svelte';
  import { rechargerJeuxDuPc } from '$lib/ludotheque/telechargements.svelte';
  import ChercherBase from './ChercherBase.svelte';
  import { dureeEstimee, ficheNommee, nomFiche, propositions, texteAnnonce, texteMotif, texteStatut, type Annonce, type JeuRapproche, type Rapprochement } from './rapprochement';

  let annonce = $state<Annonce | null>(null);
  let erreurAnnonce = $state('');
  let rapport = $state<Rapprochement | null>(null);
  let reseau = $state(false);
  let enCours = $state(false);
  let occupe = $state(false);
  let progres = $state<{ etape: string; faits: number; total: number } | null>(null);
  let plateforme = $state('');
  const PAS = 100;
  let montres = $state({ aVoir: PAS, reconnus: PAS, inconnus: PAS });
  let liste: HTMLElement | undefined = $state();

  async function lire() {
    try {
      annonce = await api.rapprocherAnnonce();
      erreurAnnonce = '';
    } catch (e) {
      annonce = null;
      erreurAnnonce = motifDuRefus(e);
    }
    rapport = await api.rapprochementDernier().catch(() => null);
  }
  onMount(lire);

  // Changer de système remet chaque liste à ses 100 premières lignes.
  $effect(() => {
    void plateforme;
    montres = { aVoir: PAS, reconnus: PAS, inconnus: PAS };
  });

  const plateformes = $derived([...new Set((rapport?.jeux ?? []).map((j) => j.plateforme))].sort());
  const filtre = (l: JeuRapproche[]) => (plateforme ? l.filter((j) => j.plateforme === plateforme) : l);
  const aVoir = $derived(filtre((rapport?.jeux ?? []).filter((j) => j.decision === 'a_voir')));
  const reconnus = $derived(filtre((rapport?.jeux ?? []).filter((j) => j.decision === 'rattache')));
  const inconnus = $derived(filtre((rapport?.jeux ?? []).filter((j) => j.decision === 'inconnu')));
  const pourcent = $derived(progres ? (progres.etape === 'firehouse' ? 100 : progres.total ? Math.round((progres.faits / progres.total) * 100) : 0) : 0);

  async function lancer() {
    enCours = true;
    progres = { etape: 'lecture', faits: 0, total: annonce?.fichiers ?? 0 };
    const arreter = isTauri()
      ? await listen<{ etape: string; faits: number; total: number }>('rapprochement', (e) => {
          progres = e.payload;
          majProgres(t, {
            etape: e.payload.etape === 'firehouse' ? 'Firehouse compare…' : 'Lecture de tes fichiers',
            chiffres: e.payload.etape === 'firehouse' ? '' : `${e.payload.faits} fichier(s) sur ${e.payload.total}`,
            pourcent: e.payload.etape === 'firehouse' ? 100 : e.payload.total ? Math.round((e.payload.faits / e.payload.total) * 100) : 0,
          });
        }).catch(() => () => {})
      : () => {};
    const t = toastProgres('Reconnaissance de tes jeux importés (tu peux continuer à utiliser Frogtend)');
    try {
      const r = await api.rapprocher(reseau);
      rapport = r;
      await rechargerJeuxDuPc();
      await rechargerListe();
      await rechargerPlateformes();
      const n = (d: string) => r.jeux.filter((j) => j.decision === d).length;
      toast(`🔗 ${n('rattache')} jeu(x) reconnu(s), ${n('a_voir')} à vérifier, ${n('inconnu')} inconnu(s) de Firehouse.`);
      annonce = await api.rapprocherAnnonce().catch(() => annonce);
    } catch (e) {
      toast(`Reconnaissance impossible : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      arreter();
      fermerToast(t);
      enCours = false;
      progres = null;
    }
  }

  /** Après une action la ligne disparaît : le focus va à la ligne qui prend sa place (ou à la précédente, ou au titre de
   * la liste), jamais perdu pour le clavier et la manette. */
  async function apres(section: string, indice: number) {
    rapport = await api.rapprochementDernier().catch(() => rapport);
    // La ludothèque montre la fiche d'un jeu relié (une seule ligne) : elle est relue.
    await rechargerJeuxDuPc();
    await rechargerListe();
    await rechargerPlateformes();
    await tick();
    const lignes = liste?.querySelectorAll<HTMLElement>(`[data-section="${section}"] li`) ?? [];
    const cible = lignes[Math.min(indice, lignes.length - 1)];
    (cible?.querySelector<HTMLElement>('button') ?? liste?.querySelector<HTMLElement>(`[data-titre="${section}"]`))?.focus();
  }

  async function agir(section: string, indice: number, action: () => Promise<boolean>) {
    if (occupe) return;
    occupe = true;
    try {
      if (await action()) await apres(section, indice);
    } catch (e) {
      toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
      await apres(section, indice);
    } finally {
      occupe = false;
    }
  }

  const relier = (j: JeuRapproche, i: number) =>
    agir('aVoir', i, async () => {
      const p = propositions(j).filter(ficheNommee);
      let choix: number | null = null;
      if (p.length === 1) {
        const ok = await confirmer(`🔗 « ${j.titre} » est-il bien « ${nomFiche(p[0])} » ?`, {
          message: 'Ton jeu reçoit la fiche, les images et les triches de Firehouse. Ses fichiers et son temps de jeu ne changent pas, et tu peux défaire ce lien.',
          libelleValider: '✅ Oui, c’est ce jeu',
        });
        choix = ok ? p[0].media_id : null;
      } else if (p.length > 1) {
        choix = await choisir<number>(`🔗 Quel jeu est « ${j.titre} » ?`, p.map((c) => ({ valeur: c.media_id, libelle: nomFiche(c) })), 'Firehouse hésite entre ces jeux. Ses fichiers et son temps de jeu ne changent pas.');
      }
      if (choix === null) return false;
      await api.rapprochementRattacher(j.jeu, choix);
      return true;
    });

  const ecarter = (j: JeuRapproche, i: number) =>
    agir('aVoir', i, async () => {
      const ok = await confirmer(`✖ « ${j.titre} » n’est pas « ${propositions(j).map(nomFiche).join(' » ni « ')} » ?`, {
        message: 'Firehouse ne te le proposera plus. Tu peux revenir sur ce choix depuis la liste des inconnus.',
        libelleValider: 'Ce n’est pas lui',
      });
      if (!ok) return false;
      for (const c of propositions(j)) await api.rapprochementEcarter(j.jeu, c.media_id);
      return true;
    });

  const reprendre = (j: JeuRapproche, i: number) =>
    agir('inconnus', i, async () => {
      await api.rapprochementReprendre(j.jeu);
      toast(`« ${j.titre} » : les jeux proposés par Firehouse sont de nouveau à vérifier.`);
      return true;
    });

  const defaire = (j: JeuRapproche, i: number) =>
    agir('reconnus', i, async () => {
      if (!(await confirmer(`Ne plus relier « ${j.titre} » à sa fiche ?`, { message: 'Ton jeu ne bouge pas : il perd seulement la fiche et les images de Firehouse.', libelleValider: 'Ne plus relier' }))) return false;
      await api.rapprochementDefaire(j.jeu);
      return true;
    });

  /** « Afficher plus » : le focus va à la première ligne nouvellement montrée. */
  async function plus(section: 'aVoir' | 'reconnus' | 'inconnus') {
    const avant = montres[section];
    montres = { ...montres, [section]: avant + PAS };
    await tick();
    liste?.querySelectorAll<HTMLElement>(`[data-section="${section}"] li`)[avant]?.querySelector<HTMLElement>('button, [tabindex]')?.focus();
  }

  const aEteEcarte = (j: JeuRapproche) => j.versions.some((v) => v.statut === 'ecarte');
  const nonVerifie = (j: JeuRapproche) => j.versions.every((v) => v.statut === 'non_envoye');
</script>

<section class="cx-block">
  <h2>Avant de lancer</h2>
  {#if annonce}
    {#if annonce.fichiers === 0}
      <p>Tu n’as pas encore de jeu importé de ton disque (📥 Importer ▸ Fichiers ROM, MAME…).</p>
    {:else}
      <p class="annonce">{texteAnnonce(annonce)}</p>
      {#if annonce.reseau}
        <label class="case">
          <input type="checkbox" bind:checked={reseau} disabled={enCours} />
          Lire aussi en entier les {annonce.reseau} fichier(s) d’un disque réseau ({taille(annonce.octets_reseau)}, environ {duree(dureeEstimee(annonce, true) - dureeEstimee(annonce, false))} de plus) ; sinon, seule la liste du contenu de leurs archives est lue.
        </label>
      {/if}
      <div class="actions">
        <button class="btn primary" onclick={lancer} disabled={enCours}>{enCours ? 'Reconnaissance…' : '🔗 Lancer la reconnaissance'}</button>
      </div>
      {#if progres}
        <p class="muted">{progres.etape === 'firehouse' ? 'Firehouse compare…' : `Lecture de tes fichiers : ${progres.faits} sur ${progres.total}`}</p>
        <div class="bar" role="progressbar" aria-label="Reconnaissance" aria-valuemin="0" aria-valuemax="100" aria-valuenow={pourcent}>
          <span style="width: {pourcent}%"></span>
        </div>
      {/if}
    {/if}
  {:else if erreurAnnonce}
    <p class="muted">{erreurAnnonce}</p>
  {:else}
    <p class="muted">Lecture de tes jeux importés…</p>
  {/if}
</section>

{#if rapport && rapport.jeux.length}
  <section class="cx-block" bind:this={liste}>
    <h2>Ce que Firehouse a reconnu</h2>
    <p class="muted">Dernière reconnaissance : {new Date(rapport.le).toLocaleString('fr-FR')}</p>
    {#if plateformes.length > 1}
      <label class="case">
        Système :
        <select bind:value={plateforme}>
          <option value="">tous</option>
          {#each plateformes as p (p)}<option value={p}>{nomAffiche(p)}</option>{/each}
        </select>
      </label>
    {/if}

    <h3 tabindex="-1" data-titre="aVoir">🤔 À vérifier ({aVoir.length})</h3>
    {#if aVoir.length === 0}
      <p class="muted">Rien à vérifier.</p>
    {:else}
      <ul class="lignes" data-section="aVoir">
        {#each aVoir.slice(0, montres.aVoir) as j, i (j.jeu)}
          {@const p = propositions(j)}
          <li>
            <div>
              <strong>{j.titre}</strong> <span class="muted">— {nomAffiche(j.plateforme)}</span>
              <p class="muted">
                Firehouse propose : {p.map(nomFiche).join(', ')}
                · {j.versions[0]?.motif ? texteMotif(j.versions[0].motif) : texteStatut(j.versions[0]?.statut ?? 'inconnu')}
              </p>
              {#if !p.some(ficheNommee)}
                <p class="muted">Firehouse a reconnu un jeu que ta ludothèque n’a pas encore reçu : mets ta ludothèque à jour, puis reviens ici.</p>
              {/if}
            </div>
            <div class="actions">
              {#if p.some(ficheNommee)}<button class="btn primary" onclick={() => relier(j, i)} disabled={occupe}>✅ C’est ce jeu</button>{/if}
              <button class="btn" onclick={() => ecarter(j, i)} disabled={occupe}>✖ Ce n’est pas lui</button>
            </div>
          </li>
        {/each}
      </ul>
      {#if aVoir.length > montres.aVoir}<button class="btn" onclick={() => plus('aVoir')}>Afficher 100 de plus ({aVoir.length - montres.aVoir} restant(s))</button>{/if}
    {/if}

    <details>
      <summary data-titre="reconnus">✅ Reconnus ({reconnus.length})</summary>
      <ul class="lignes" data-section="reconnus">
        {#each reconnus.slice(0, montres.reconnus) as j, i (j.jeu)}
          <li>
            <div>
              <strong>{j.titre}</strong> <span class="muted">— {nomAffiche(j.plateforme)} · {j.rattache_par === 'personne' ? 'relié par toi' : 'reconnu à coup sûr'}</span>
              {#if j.versions[0]?.motif}<p class="muted">⚠ {texteMotif(j.versions[0].motif)}</p>{/if}
            </div>
            <button class="btn" onclick={() => defaire(j, i)} disabled={occupe}>Ne plus relier</button>
          </li>
        {/each}
      </ul>
      {#if reconnus.length > montres.reconnus}<button class="btn" onclick={() => plus('reconnus')}>Afficher 100 de plus</button>{/if}
    </details>

    <details>
      <summary data-titre="inconnus">❓ Pas reconnus ({inconnus.length})</summary>
      <p class="muted">Ils restent dans ta ludothèque, comme avant.</p>
      <ul class="lignes" data-section="inconnus">
        {#each inconnus.slice(0, montres.inconnus) as j, i (j.jeu)}
          <li>
            <span>{j.titre} <span class="muted">— {nomAffiche(j.plateforme)} · {nonVerifie(j) ? texteStatut('non_envoye') : aEteEcarte(j) ? texteStatut('ecarte') : texteStatut('inconnu')}</span></span>
            {#if aEteEcarte(j)}<button class="btn" onclick={() => reprendre(j, i)} disabled={occupe}>↩ Revoir ce que Firehouse proposait</button>{/if}
          </li>
        {/each}
      </ul>
      {#if inconnus.length > montres.inconnus}<button class="btn" onclick={() => plus('inconnus')}>Afficher 100 de plus</button>{/if}
    </details>
    {#if rapport.non_envoyes}<p class="muted">{rapport.non_envoyes} fichier(s) n’ont pas pu être vérifiés (trop de fichiers d’un coup pour Firehouse).</p>{/if}
  </section>
{/if}

<!-- Contrat 1.32 : les jeux sans fiche, cherchés dans la base de Firehouse (après le rapprochement par le contenu). -->
<ChercherBase />

<style>
  .annonce {
    white-space: pre-line;
  }
  .case {
    display: flex;
    gap: calc(8 * var(--u));
    align-items: center;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: calc(8 * var(--u));
  }
  .lignes {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: calc(8 * var(--u));
  }
  .lignes li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: calc(12 * var(--u));
    flex-wrap: wrap;
    padding: calc(8 * var(--u)) 0;
    border-bottom: 1px solid var(--line);
  }
  .lignes p {
    margin: calc(4 * var(--u)) 0 0;
  }
  details summary {
    cursor: pointer;
    font-weight: 600;
    margin: calc(12 * var(--u)) 0 calc(6 * var(--u));
  }
</style>
