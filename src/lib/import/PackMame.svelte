<script lang="ts">
  // 🕹 Tout MAME depuis Firehouse (contrat 1.33 ; ✅ Seb 27/10 : dossier neuf proposé d'office, ouvert à tous après
  // l'annonce et l'accord, les CHD à part). Rien n'est reçu avant le oui ; rien n'est jamais effacé ni remplacé.
  import { onMount } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { api, duree, taille } from '$lib/api';
  import { confirmer, fermerToast, informer, majProgres, toast, toastProgres } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { parcourir } from '$lib/emulateurs/assistant.svelte';
  import { etat, reglerPc } from '$lib/etat.svelte';
  import { debitConnu, rechargerJeuxDuPc } from '$lib/ludotheque/telechargements.svelte';
  import { rechargerListe, rechargerPlateformes } from '$lib/ludotheque/ludotheque.svelte';
  import { aFaire, avisVersionMame, placeSuffisante, texteAnnoncePack, type AnnoncePack } from './packMame';


  let dossier = $state('');
  let annonce = $state<AnnoncePack | null>(null);
  let refus = $state('');
  let avecChd = $state(false);
  let enCours = $state(false);
  let fini = $state(false);
  let avisMame = $state<string | null>(null);

  async function charger() {
    try {
      [dossier, annonce] = await api.packMameAnnonce(etat.pc.dossierPackMame || null, avecChd);
      refus = '';
      // MAME doit être à la version du romset (étape E) : dit avant la réception.
      const mame = (await api.emulateursInstalles().catch(() => [])).find((e) => e.id === 'mame');
      avisMame = avisVersionMame(mame, annonce.version);
    } catch (e) {
      annonce = null;
      refus = motifDuRefus(e);
    }
  }
  onMount(charger);

  /** Étape B : un jeu par fiche de Firehouse, relié à elle (jaquette, résumé), ses clones en versions. */
  async function versLaLudotheque() {
    try {
      const b = await api.packMameLudotheque(dossier);
      await rechargerJeuxDuPc();
      await rechargerListe();
      await rechargerPlateformes();
      toast(b.jeux || b.versions ? `✅ ${b.jeux} jeu(x) MAME ajouté(s) à ta ludothèque, avec leur fiche (${b.versions} version(s) de plus).` : `Les jeux du pack sont déjà dans ta ludothèque.`);
    } catch (e) {
      toast(`Impossible : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  async function autreDossier() {
    const d = await parcourir({ dossier: true, titre: 'Dossier du pack MAME (vide ou nouveau)' });
    if (!d) return;
    await reglerPc('dossierPackMame', d);
    await charger();
  }

  async function recevoir() {
    if (!annonce || enCours) return;
    await charger();
    if (!annonce) return;
    if (!placeSuffisante(annonce)) {
      await informer('🕹 Pas assez de place', `${texteAnnoncePack(annonce, debitConnu(), duree)}\n\nLibère de la place, ou choisis un dossier sur un autre disque.`);
      return;
    }
    // L'accord porte sur ce qui est annoncé : la mise à jour seulement si elle l'a été.
    const maj = annonce.anciens > 0;
    const titre = !maj ? '🕹 Recevoir le pack MAME ?' : annonce.a_recevoir > 0 ? '🕹 Recevoir et mettre à jour le pack MAME ?' : '🕹 Mettre à jour le pack MAME ?';
    const oui = await confirmer(titre, {
      message: `${texteAnnoncePack(annonce, debitConnu(), duree)}\n\nDans : ${dossier}\n\nTu peux arrêter quand tu veux : la suite reprendra où elle en était. Rien n’est jamais effacé${maj ? '.\nFerme MAME avant : un fichier en cours d’utilisation, ou que tu as modifié, est laissé tel quel.' : ' ni remplacé.'}`,
      libelleValider: maj ? 'Mettre à jour' : 'Recevoir',
    });
    if (!oui) return;
    await reglerPc('dossierPackMame', dossier);
    enCours = true;
    const t = toastProgres('Pack MAME : réception (tu peux continuer à utiliser Frogtend)');
    const arreter = isTauri()
      ? await listen<{ fait: number; total: number; octets: number }>('pack-mame', (e) => {
          majProgres(t, {
            etape: 'Réception du pack MAME',
            chiffres: `${e.payload.fait} fichier(s) sur ${e.payload.total} · ${taille(e.payload.octets)}`,
            pourcent: e.payload.total ? Math.round((e.payload.fait / e.payload.total) * 100) : 0,
          });
        }).catch(() => () => {})
      : () => {};
    try {
      const b = await api.packMameLancer(dossier, avecChd, maj ? annonce.version : null);
      const laisses = b.laisses.length ? ` ${b.laisses.length} fichier(s) laissé(s) (voir le journal).` : '';
      toast(
        b.arret ? `🕹 Pack MAME : ${b.recus} fichier(s) reçu(s), arrêté (${b.arret}) : relance pour continuer.${laisses}` : `✅ Pack MAME : ${b.recus} fichier(s) reçu(s).${laisses}`,
        b.arret || b.laisses.length ? 'alerte' : 'ok',
      );
      fini = !b.arret;
    } catch (e) {
      toast(`Pack MAME impossible : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      arreter();
      fermerToast(t);
      enCours = false;
      await charger();
    }
  }
</script>

<section class="cx-block">
  <h3>🕹 Tout MAME depuis Firehouse</h3>
  {#if refus}
    <p class="muted">{refus}</p>
    <button class="btn" onclick={autreDossier}>📂 Choisir un dossier</button>
  {:else if annonce && annonce.en_cours && !enCours}
    <p>🕹 Le pack MAME est déjà en train d’arriver sur ce PC (lancé plus tôt). Tu peux l’arrêter tout de suite : il reprendra où il en était.</p>
    <div class="actions">
      <button class="btn" onclick={() => api.packMameArreter()}>⏸ Arrêter</button>
      <button class="btn" onclick={charger}>↻ Voir où il en est</button>
    </div>
  {:else if annonce}
    <p class="annonce">{texteAnnoncePack(annonce, debitConnu(), duree)}</p>
    {#if avisMame}<p class="avis">⚠ {avisMame}</p>{/if}
    <p class="muted">Dossier : {dossier} <button class="btn petit" onclick={autreDossier} disabled={enCours}>Changer</button></p>
    {#if annonce.chd_fichiers > 0 || avecChd}
      <label class="case">
        <input type="checkbox" bind:checked={avecChd} onchange={charger} disabled={enCours} />
        Ajouter aussi les disques (CHD) de quelques jeux : {annonce.chd_fichiers} fichier(s), {taille(annonce.chd_octets)}
      </label>
    {/if}
    <div class="actions">
      {#if aFaire(annonce)}
        <button class="btn primary" onclick={recevoir} disabled={enCours}>
          {enCours ? 'Réception…' : annonce.anciens > 0 ? '⬆ Mettre à jour le pack' : '🕹 Recevoir le pack'}
        </button>
      {/if}
      {#if enCours}<button class="btn" onclick={() => api.packMameArreter()}>⏸ Arrêter</button>{/if}
      {#if (annonce.deja_la > 0 || fini) && !enCours}
        <button class="btn" onclick={versLaLudotheque}>📥 Mettre ces jeux dans ma ludothèque</button>
      {/if}
    </div>
  {/if}
</section>

<style>
  .annonce {
    white-space: pre-line;
  }
  .avis {
    font-weight: 600;
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
</style>
