<script lang="ts">
  // Créer un profil : un nom, un code PIN facultatif, et le jeton Firehouse de la personne.
  import { api, libelleCompte } from '$lib/api';
  import { toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { etat } from '$lib/etat.svelte';

  let {
    premier = false,
    oncree,
    onannuler,
  }: { premier?: boolean; oncree: () => void; onannuler?: () => void } = $props();

  let nom = $state('');
  let proteger = $state(true);
  let pin = $state('');
  let pin2 = $state('');
  let jeton = $state('');
  let envoi = $state(false);

  const simule = $derived(etat.pc.firehouse.simule);
  const pinValide = $derived(/^\d{4,8}$/.test(pin));
  const probleme = $derived.by(() => {
    if (nom.trim() === '') return 'Donne un nom au profil.';
    if (proteger && !pinValide) return 'Le code PIN doit faire de 4 à 8 chiffres.';
    if (proteger && pin !== pin2) return 'Les deux codes PIN ne sont pas identiques.';
    if (!simule && jeton.trim() === '') return 'Colle le jeton Firehouse de cette personne.';
    return '';
  });

  async function creer(e: SubmitEvent) {
    e.preventDefault();
    if (probleme) return;
    envoi = true;
    try {
      const p = await api.creerProfil(nom.trim(), proteger ? pin : null, jeton.trim() || null);
      toast(
        p.compte
          ? `✅ Profil « ${p.nom} » créé. Jeton vérifié : ${libelleCompte(p.compte)}.`
          : `✅ Profil « ${p.nom} » créé.`,
      );
      oncree();
    } catch (err) {
      toast(`Refusé : ${motifDuRefus(err)}`, 'erreur');
    } finally {
      envoi = false;
      pin = pin2 = jeton = '';
    }
  }
</script>

<form class="formulaire" onsubmit={creer}>
  <h2>{premier ? '👋 Bienvenue ! Crée ton profil' : '➕ Nouveau profil'}</h2>
  <p class="muted">
    Un profil par personne : chacun voit sa ludothèque, avec ses réglages. Ce que chacun voit est décidé par Firehouse.
  </p>

  <label class="champ">
    <span>Nom (celui affiché sur l'écran d'accueil)</span>
    <!-- svelte-ignore a11y_autofocus -->
    <input type="text" bind:value={nom} maxlength="40" autofocus autocomplete="off" />
  </label>

  <label class="case">
    <input type="checkbox" bind:checked={proteger} />
    <span>Protéger ce profil par un code PIN (conseillé si d'autres personnes utilisent ce PC)</span>
  </label>
  {#if proteger}
    <div class="deux">
      <label class="champ">
        <span>Code PIN (4 à 8 chiffres)</span>
        <input type="password" inputmode="numeric" bind:value={pin} maxlength="8" autocomplete="new-password" />
      </label>
      <label class="champ">
        <span>Le même, encore une fois</span>
        <input type="password" inputmode="numeric" bind:value={pin2} maxlength="8" autocomplete="new-password" />
      </label>
    </div>
  {/if}

  <label class="champ">
    <span>Jeton Firehouse {simule ? '(facultatif en mode simulé)' : ''}</span>
    <input type="password" bind:value={jeton} autocomplete="off" spellcheck="false" />
  </label>
  <p class="muted">
    Le jeton est créé par un admin dans le cockpit de Firehouse, un par PC. Il est rangé dans le coffre de Windows :
    Frogtend ne l'affiche jamais.
  </p>

  {#if probleme}<p class="muted">ℹ {probleme}</p>{/if}

  <div class="actions">
    {#if onannuler}<button type="button" class="btn" onclick={onannuler}>Annuler</button>{/if}
    <button type="submit" class="btn primary" disabled={!!probleme || envoi}>
      {envoi ? 'Création…' : 'Créer le profil'}
    </button>
  </div>
</form>

<style>
  .formulaire {
    width: 100%;
    display: grid;
    gap: calc(12 * var(--u));
  }
  h2 {
    margin: 0;
    font-size: calc(20 * var(--u));
    text-align: center;
  }
  .muted {
    margin: 0;
  }
  .deux {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(calc(200 * var(--u)), 1fr));
    gap: calc(12 * var(--u));
  }
  .case {
    display: flex;
    align-items: center;
    gap: calc(10 * var(--u));
  }
  .case input {
    width: calc(20 * var(--u));
    min-height: 0;
    height: calc(20 * var(--u));
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: calc(8 * var(--u));
  }
</style>
