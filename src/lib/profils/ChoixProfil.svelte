<script lang="ts">
  // L'écran d'accueil : « Qui joue ? ». Choisir son profil (PIN s'il est protégé), ou en créer un.
  import { onMount } from 'svelte';
  import { api, estErreurCoeur, type Profil } from '$lib/api';
  import { choisir as choisirDans, demander, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { entrerDansProfil, etat, reglerPc } from '$lib/etat.svelte';
  import { verifierAdresse } from '$lib/reglages/reglages';
  import PavePin from './PavePin.svelte';
  import NouveauProfil from './NouveauProfil.svelte';

  let profils = $state<Profil[] | null>(null);
  let choisi = $state<Profil | null>(null);
  let erreurPin = $state('');
  let creation = $state(false);
  let ouverture = $state(false);

  async function charger() {
    try {
      profils = await api.profils();
      if (profils.length === 0) creation = true;
    } catch (e) {
      profils = [];
      toast(`Impossible de lire les profils : ${motifDuRefus(e)}`, 'erreur');
    }
  }
  onMount(charger);

  async function ouvrir(p: Profil, pin: string | null) {
    ouverture = true;
    erreurPin = '';
    try {
      const ouvert = await api.ouvrirProfil(p.id, pin);
      await entrerDansProfil(ouvert);
    } catch (e) {
      if (estErreurCoeur(e) && e.sorte === 'pin') erreurPin = e.motif;
      else toast(`Impossible d’ouvrir ce profil : ${motifDuRefus(e)}`, 'erreur');
    } finally {
      ouverture = false;
    }
  }

  function choisir(p: Profil) {
    if (p.protege) {
      choisi = p;
      erreurPin = '';
    } else {
      ouvrir(p, null);
    }
  }

  const initiale = (nom: string) => nom.trim().charAt(0).toUpperCase();

  /** Les réglages de connexion de ce PC, accessibles avant d'ouvrir un profil. */
  async function connexion() {
    const simule = etat.pc.firehouse.simule;
    const c = await choisirDans('🔌 Connexion à Firehouse', [
      { valeur: 'adresse', libelle: '✏ Changer l’adresse', detail: etat.pc.firehouse.adresse },
      {
        valeur: 'mode',
        libelle: simule ? '🌐 Utiliser le vrai Firehouse' : '🧪 Passer en mode simulé',
        detail: simule ? 'quitter les exemples' : 'des exemples, sans connexion',
      },
    ]);
    if (c === 'mode') {
      await reglerPc('firehouse.simule', !simule);
      toast(simule ? 'Vrai Firehouse : chaque profil a besoin de son jeton.' : 'Mode simulé : des exemples, sans connexion.');
    } else if (c === 'adresse') {
      const a = await demander('✏ Adresse de Firehouse', { valeur: etat.pc.firehouse.adresse });
      if (a === null) return;
      const r = verifierAdresse(a);
      if ('refus' in r) toast(`Refusé : ${r.refus}`, 'erreur');
      else {
        await reglerPc('firehouse.adresse', r.adresse);
        toast('Enregistré.');
      }
    }
  }
</script>

<div class="accueil">
  <div class="panel boite">
    <div class="titre">
      <img src="/grenouille.png" alt="" />
      <h1>Frogtend</h1>
    </div>

    {#if creation}
      <NouveauProfil
        premier={profils?.length === 0}
        onannuler={profils && profils.length > 0 ? () => (creation = false) : undefined}
        oncree={async () => {
          creation = false;
          await charger();
        }}
      />
    {:else if choisi}
      <h2>🔒 {choisi.nom}</h2>
      <p class="muted">Tape ton code PIN.</p>
      <PavePin
        erreur={erreurPin}
        onvalider={(pin) => choisi && ouvrir(choisi, pin)}
        onannuler={() => (choisi = null)}
      />
    {:else}
      <h2>Qui joue ?</h2>
      {#if profils === null}
        <p class="muted">Chargement des profils…</p>
      {:else}
        <div class="profils">
          {#each profils as p (p.id)}
            <button class="profil cx-card" onclick={() => choisir(p)} disabled={ouverture}>
              <span class="avatar" aria-hidden="true">{initiale(p.nom)}</span>
              <span class="nom">{p.nom}</span>
              <span class="muted">
                {p.protege ? '🔒 protégé' : 'sans code'}
                {#if !p.a_un_jeton && !etat.pc.firehouse.simule}· ⚠ sans jeton{/if}
              </span>
            </button>
          {/each}
          <button class="profil cx-card ajouter" onclick={() => (creation = true)}>
            <span class="avatar" aria-hidden="true">＋</span>
            <span class="nom">Nouveau profil</span>
          </button>
        </div>
      {/if}
      <p class="pied muted">
        {etat.pc.firehouse.simule ? '🧪 Mode simulé' : `Firehouse : ${etat.pc.firehouse.adresse}`}
        <button class="btn petit" onclick={connexion}>🔌 Connexion</button>
      </p>
    {/if}
  </div>
</div>

<style>
  .accueil {
    min-height: 100%;
    display: grid;
    place-items: center;
    padding: calc(24 * var(--u));
  }
  .boite {
    width: min(calc(760 * var(--u)), 100%);
    padding: calc(28 * var(--u));
    display: grid;
    justify-items: center;
    gap: calc(12 * var(--u));
  }
  .titre {
    display: flex;
    align-items: center;
    gap: calc(12 * var(--u));
  }
  .titre img {
    width: calc(64 * var(--u));
    height: calc(64 * var(--u));
  }
  h1 {
    margin: 0;
    font-size: calc(34 * var(--u));
  }
  h2 {
    margin: calc(8 * var(--u)) 0 0;
    font-size: calc(20 * var(--u));
  }
  .profils {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: calc(16 * var(--u));
    margin-top: calc(10 * var(--u));
  }
  .profil {
    font: inherit;
    color: var(--ink);
    cursor: pointer;
    width: calc(150 * var(--u));
    padding: calc(18 * var(--u)) calc(12 * var(--u));
    display: grid;
    justify-items: center;
    gap: calc(6 * var(--u));
  }
  .avatar {
    width: calc(72 * var(--u));
    height: calc(72 * var(--u));
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: calc(30 * var(--u));
    font-weight: 700;
    background: linear-gradient(135deg, var(--accent2), var(--accent));
    color: var(--on-accent);
  }
  .ajouter .avatar {
    background: var(--panel);
    color: var(--dim);
    border: 2px dashed var(--line);
  }
  .nom {
    font-weight: 600;
  }
  .pied {
    display: flex;
    align-items: center;
    gap: calc(10 * var(--u));
    margin-top: calc(14 * var(--u));
  }
  .petit {
    min-height: calc(32 * var(--u));
    padding: calc(4 * var(--u)) calc(10 * var(--u));
  }
</style>
