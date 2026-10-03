<script lang="ts">
  // 📦 Les contenus additionnels d'un jeu du PC (PS3 d'abord) : trouvés sur le disque, cochés par la personne,
  // installés seulement après son oui (taille annoncée).
  import { api, taille } from '$lib/api';
  import { confirmer, toast } from '$lib/dialogues/fenetres.svelte';
  import { motifDuRefus } from '$lib/dialogues/messages';
  import { libelleGenre, nomCourt, resume, type ContenusDuJeu } from './contenus';

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

  async function installer() {
    if (!choisis.length) return;
    const oui = await confirmer(`📦 Installer ${choisis.length} contenu(s) pour « ${titre} » ?`, {
      message: [
        `${taille(total)} à installer par RPCS3 (il travaille sans fenêtre, puis se ferme).`,
        ...choisis.slice(0, 8).map((c) => `• ${nomCourt(c.nom, titre)} (${libelleGenre(c.genre)})${c.licence ? ' — avec sa licence' : ''}`),
        choisis.length > 8 ? `… et ${choisis.length - 8} autre(s).` : '',
        'Les licences vont dans le compte RPCS3 de TON profil : un autre profil devra les installer pour lui.',
      ]
        .filter(Boolean)
        .join('\n'),
      libelleValider: 'Installer',
    });
    if (!oui) return;
    enCours = true;
    try {
      const b = await api.contenusInstaller(id, choisis.map((c) => c.id));
      toast(
        b.refuses.length
          ? `📦 ${b.installes} installé(s), ${b.refuses.length} refusé(s) : ${b.refuses.map(([n, m]) => `${nomCourt(n, titre)} (${m})`).join(', ')}`
          : `✅ ${b.installes} contenu(s) installé(s) pour « ${titre} ».`,
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
  <p class="muted">📦 Règle RPCS3 (⚙ Options ▸ Émulateurs) pour gérer les DLC de ce jeu.</p>
{:else if donnees?.etat === 'ok' && donnees.contenus.length}
  <div class="contenus cx-block">
    <button class="btn" onclick={() => (ouvert = !ouvert)}>📦 Contenus additionnels : {resume(donnees.contenus)} {ouvert ? '▴' : '▾'}</button>
    {#if ouvert}
      <ul>
        {#each donnees.contenus as c (c.id)}
          <li>
            {#if c.installe}
              <span title="Déjà installé pour ton profil">✅</span>
            {:else}
              <input type="checkbox" bind:group={coches} value={c.id} aria-label={`Choisir ${c.nom}`} />
            {/if}
            <span class="nom">{nomCourt(c.nom, titre)}</span>
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
  .actions {
    display: flex;
    gap: calc(6 * var(--u));
    flex-wrap: wrap;
  }
  p {
    margin: 0;
  }
</style>
