<script lang="ts">
  // La sauvegarde du profil : ce qu'elle contient, où elle va, quand elle se fait.
  import { onMount } from 'svelte';
  import { taille } from '$lib/api';
  import { etat, reglerProfil } from '$lib/etat.svelte';
  import { depuis } from '$lib/ludotheque/ludotheque.svelte';
  import { lireDerniereSauvegarde, sauvegarde, sauvegarder } from '$lib/sauvegarde.svelte';

  onMount(lireDerniereSauvegarde);
  const d = $derived(sauvegarde.derniere);
</script>

<div class="sauvegarde">
  <p class="muted">
    Tout ce qui ne se retélécharge pas : tes réglages, la liste de tes jeux, et les parties et codes de triche de
    tous tes jeux (jeux PC comme émulateurs). Elle part chez Firehouse, dans ton dossier, par Internet : ça marche à
    la maison comme ailleurs. Firehouse garde les versions précédentes. Après un reformatage, tu retrouves tout.
  </p>

  <dl class="cx-kv">
    <dt>Dernière sauvegarde</dt>
    <dd>
      {#if d}
        {depuis(Number(d.date))}, depuis {d.pc} · {d.fichiers} fichier(s) de parties ({taille(d.taille)})
      {:else}
        jamais depuis ce PC
      {/if}
    </dd>
  </dl>

  <div class="cx-form-section">
    <label class="champ">
      <span>Sauvegarde automatique</span>
      <select value={etat.profil.sauvegarde.auto} onchange={(e) => reglerProfil('sauvegarde.auto', e.currentTarget.value)}>
        <option value="apres_partie">Après chaque partie (conseillé)</option>
        <option value="quotidienne">Une fois par jour</option>
        <option value="manuelle">Jamais : seulement quand je le demande</option>
      </select>
    </label>
  </div>

  <div class="actions">
    <button class="btn primary" onclick={() => sauvegarder()} disabled={sauvegarde.enCours || etat.pc.firehouse.simule}>
      {sauvegarde.enCours ? 'Sauvegarde en cours…' : '💾 Sauvegarder maintenant'}
    </button>
  </div>
  {#if etat.pc.firehouse.simule}<p class="muted">En mode simulé, rien n’est sauvegardé.</p>{/if}
</div>

<style>
  .sauvegarde {
    display: grid;
    gap: calc(12 * var(--u));
  }
  .muted {
    margin: 0;
  }
  .actions {
    display: flex;
    gap: calc(8 * var(--u));
  }
</style>
