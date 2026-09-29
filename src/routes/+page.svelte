<script lang="ts">
  // La ludothèque, en trois colonnes : plateformes, jaquettes, détails (inspirée de LaunchBox).
  import { onMount } from 'svelte';
  import { etat, reglerProfil } from '$lib/etat.svelte';
  import GrilleJeux from '$lib/ludotheque/GrilleJeux.svelte';
  import { ouvrirLudotheque, rechargerListe } from '$lib/ludotheque/ludotheque.svelte';
  import PanneauDetails from '$lib/ludotheque/PanneauDetails.svelte';
  import PanneauPlateformes from '$lib/ludotheque/PanneauPlateformes.svelte';
  import { TAILLE_JAQUETTE_MAX, TAILLE_JAQUETTE_MIN } from '$lib/reglages/reglages';

  const r = $derived(etat.profil.ludotheque);

  onMount(ouvrirLudotheque);
</script>

<div class="ludotheque" class:sans-gauche={!r.panneauPlateformes} class:sans-droite={!r.panneauDetails}>
  {#if r.panneauPlateformes}
    <div class="gauche"><PanneauPlateformes /></div>
  {/if}

  <div class="centre">
    <div class="outils">
      <button
        class="btn petit"
        onclick={() => reglerProfil('ludotheque.panneauPlateformes', !r.panneauPlateformes)}
        title={r.panneauPlateformes ? 'Masquer les plateformes' : 'Afficher les plateformes'}
      >
        {r.panneauPlateformes ? '◀ Plateformes' : '▶ Plateformes'}
      </button>
      <label class="tri">
        <span class="muted">Trier par</span>
        <select
          value={r.tri}
          onchange={async (e) => {
            await reglerProfil('ludotheque.tri', e.currentTarget.value);
            rechargerListe();
          }}
        >
          <option value="titre">Titre</option>
          <option value="annee">Année (ancien d’abord)</option>
          <option value="annee_desc">Année (récent d’abord)</option>
        </select>
      </label>
      <label class="taille">
        <span class="muted" aria-hidden="true">🖼</span>
        <input
          type="range"
          aria-label="Taille des jaquettes"
          min={TAILLE_JAQUETTE_MIN}
          max={TAILLE_JAQUETTE_MAX}
          step="10"
          value={r.tailleJaquette}
          onchange={(e) => reglerProfil('ludotheque.tailleJaquette', Number(e.currentTarget.value))}
        />
      </label>
      <button
        class="btn petit"
        onclick={() => reglerProfil('ludotheque.panneauDetails', !r.panneauDetails)}
        title={r.panneauDetails ? 'Masquer les détails' : 'Afficher les détails'}
      >
        {r.panneauDetails ? 'Détails ▶' : 'Détails ◀'}
      </button>
    </div>
    <GrilleJeux />
  </div>

  {#if r.panneauDetails}
    <div class="droite"><PanneauDetails /></div>
  {/if}
</div>

<style>
  .ludotheque {
    height: 100%;
    display: grid;
    grid-template-columns: calc(280 * var(--u)) 1fr calc(340 * var(--u));
    min-height: 0;
  }
  .ludotheque.sans-gauche {
    grid-template-columns: 1fr calc(340 * var(--u));
  }
  .ludotheque.sans-droite {
    grid-template-columns: calc(280 * var(--u)) 1fr;
  }
  .ludotheque.sans-gauche.sans-droite {
    grid-template-columns: 1fr;
  }
  .gauche,
  .droite {
    background: color-mix(in srgb, var(--panel) 85%, transparent);
    padding: calc(12 * var(--u));
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .gauche {
    border-right: 1px solid var(--line);
  }
  .droite {
    border-left: 1px solid var(--line);
  }
  .centre {
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
  }
  .outils {
    display: flex;
    align-items: center;
    gap: calc(10 * var(--u));
    padding: calc(8 * var(--u)) calc(12 * var(--u));
    flex-wrap: wrap;
  }
  .petit {
    min-height: calc(32 * var(--u));
    padding: calc(4 * var(--u)) calc(10 * var(--u));
    font-size: calc(12 * var(--u));
  }
  .tri,
  .taille {
    display: flex;
    align-items: center;
    gap: calc(6 * var(--u));
  }
  .tri select {
    width: auto;
    min-height: calc(32 * var(--u));
  }
  .taille {
    flex: 1;
    justify-content: flex-end;
  }
  .taille input {
    width: calc(140 * var(--u));
    min-height: 0;
    padding: 0;
  }
  @media (max-width: 1000px) {
    .ludotheque,
    .ludotheque.sans-gauche,
    .ludotheque.sans-droite {
      grid-template-columns: 1fr;
    }
    .gauche,
    .droite {
      max-height: 40vh;
    }
  }
</style>
