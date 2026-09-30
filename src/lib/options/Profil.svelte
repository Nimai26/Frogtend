<script lang="ts">
  // « Mon profil » : renommer, changer ou retirer le code PIN, remplacer le jeton, supprimer le profil.
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { api, libelleCompte, type Compte } from '$lib/api';
  import { confirmer, demander, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { etat, sortirDuProfil } from '$lib/etat.svelte';
  import { synchroniser, viderLudotheque } from '$lib/ludotheque/ludotheque.svelte';

  const p = $derived(etat.profilOuvert!);

  let compte = $state<Compte | null>(null);
  let compteErreur = $state('');
  async function lireCompte() {
    compteErreur = '';
    try {
      compte = await api.compte();
    } catch (e) {
      compte = null;
      compteErreur = motifDuRefus(e);
    }
  }
  onMount(lireCompte);

  async function renommer() {
    const nom = await demander('✏ Nouveau nom du profil', { valeur: p.nom });
    if (!nom || nom.trim() === p.nom) return;
    try {
      etat.profilOuvert = await api.renommerProfil(nom);
      toast('Enregistré.');
    } catch (e) {
      toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  /** Demande un PIN dans un champ masqué (fenêtre maison « Demander »). */
  const demanderPin = (titre: string) => demander(titre, { message: '4 à 8 chiffres.', masque: true });

  async function changerPin(retirer: boolean) {
    const ancien = p.protege ? await demanderPin('🔒 Ton code PIN actuel') : null;
    if (p.protege && ancien === null) return;
    let nouveau: string | null = null;
    if (!retirer) {
      nouveau = await demanderPin('🔒 Nouveau code PIN');
      if (nouveau === null) return;
      const encore = await demanderPin('🔒 Le même, encore une fois');
      if (encore !== nouveau) {
        toast('Refusé : les deux codes PIN ne sont pas identiques.', 'erreur');
        return;
      }
    }
    try {
      await api.changerPin(ancien, nouveau);
      etat.profilOuvert = { ...p, protege: !retirer };
      toast(retirer ? '✅ Code PIN retiré.' : '✅ Code PIN changé.');
    } catch (e) {
      toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  async function changerJeton() {
    const jeton = await demander('🔑 Nouveau jeton Firehouse', {
      message: 'Colle le jeton créé par un admin dans le cockpit de Firehouse. Il ira dans le coffre de Windows.',
      masque: true,
    });
    if (!jeton) return;
    try {
      const c = await api.changerJeton(jeton);
      toast(c ? `✅ Jeton vérifié (${libelleCompte(c)}) et enregistré. Synchronisation…` : '✅ Jeton enregistré. Synchronisation…');
      await lireCompte();
      await synchroniser();
    } catch (e) {
      toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
    }
  }

  async function supprimer() {
    const oui = await confirmer(`🗑 Supprimer le profil « ${p.nom} » de ce PC ?`, {
      message:
        'Son jeton, sa ludothèque téléchargée et ses réglages seront effacés de ce PC. Rien ne change dans Firehouse.',
      danger: true,
    });
    if (!oui) return;
    const pin = p.protege ? await demanderPin('🔒 Ton code PIN, pour confirmer') : null;
    if (p.protege && pin === null) return;
    try {
      await api.supprimerProfil(p.id, pin);
      viderLudotheque();
      await sortirDuProfil();
      toast('✅ Profil supprimé de ce PC.');
      goto('/');
    } catch (e) {
      toast(`Refusé : ${motifDuRefus(e)}`, 'erreur');
    }
  }
</script>

<div class="page">
  <section class="panel">
    <header>👤 Mon profil</header>
    <div class="corps">
      <dl class="cx-kv">
        <dt>Nom</dt>
        <dd>{p.nom}</dd>
        <dt>Code PIN</dt>
        <dd>{p.protege ? '🔒 oui' : 'non'}</dd>
        <dt>Compte Firehouse</dt>
        <dd>{compte ? libelleCompte(compte) : compteErreur ? `⚠ ${compteErreur}` : 'vérification…'}</dd>
      </dl>
      <div class="actions">
        <button class="btn" onclick={renommer}>✏ Renommer</button>
        <button class="btn" onclick={() => changerPin(false)}>🔒 {p.protege ? 'Changer' : 'Ajouter'} le code PIN</button>
        {#if p.protege}<button class="btn" onclick={() => changerPin(true)}>Retirer le code PIN</button>{/if}
        <button class="btn" onclick={changerJeton}>🔑 Remplacer le jeton Firehouse</button>
      </div>
    </div>
  </section>

  <section class="panel">
    <header>⚠ Zone sensible</header>
    <div class="corps">
      <p class="muted">Supprimer ce profil efface de ce PC son jeton, sa ludothèque téléchargée et ses réglages.</p>
      <div class="actions">
        <button class="btn danger" onclick={supprimer}>🗑 Supprimer ce profil</button>
      </div>
    </div>
  </section>
</div>

<style>
  .page {
    display: grid;
    gap: calc(16 * var(--u));
  }
  .corps {
    padding: calc(16 * var(--u));
    display: grid;
    gap: calc(14 * var(--u));
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: calc(8 * var(--u));
  }
</style>
