<script lang="ts">
  import { invoke, isTauri } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { etat } from '$lib/etat.svelte';
  import { verifierMiseAJour } from '$lib/mises-a-jour';

  let infos = $state<{ nom: string; version: string; identifiant: string } | null>(null);
  let verification = $state(false);

  onMount(async () => {
    if (isTauri()) infos = await invoke('infos_application');
  });

  async function verifier() {
    verification = true;
    try {
      await verifierMiseAJour(false);
    } finally {
      verification = false;
    }
  }
</script>

<section class="panel">
  <header>ℹ À propos de Frogtend</header>
  <div class="corps">
    <dl class="cx-kv">
      <dt>Version</dt>
      <dd>{infos?.version ?? 'inconnue (hors de l’application)'}</dd>
      <dt>Identifiant</dt>
      <dd>{infos?.identifiant ?? '—'}</dd>
      <dt>Skin affiché</dt>
      <dd>{etat.skinApplique ?? '—'}</dd>
    </dl>

    <div class="actions">
      <button class="btn primary" onclick={verifier} disabled={verification || !isTauri()}>
        {verification ? 'Vérification…' : '⬆ Chercher une mise à jour'}
      </button>
    </div>

    <div class="cx-block">
      <h3>Crédits</h3>
      <p>{etat.catalogue?.credit}</p>
      <p class="muted">Frogtend est distribué sous licence Apache 2.0.</p>
    </div>
  </div>
</section>

<style>
  .corps {
    padding: calc(16 * var(--u));
    display: grid;
    gap: calc(14 * var(--u));
  }
  .actions {
    display: flex;
    justify-content: flex-end;
  }
</style>
