<script lang="ts">
  // Les téléchargements : ce qui arrive sur le PC, où ça en est, et les boutons pour agir.
  import { goto } from '$app/navigation';
  import { duree, taille } from '$lib/api';
  import { annuler, mettreEnPause, rechargerJeuxDuPc, reprendre, tele } from '$lib/ludotheque/telechargements.svelte';
  import { onMount } from 'svelte';

  onMount(rechargerJeuxDuPc);

  const liste = $derived(
    Object.values(tele.jeux).sort((a, b) => {
      const rang = { en_cours: 0, attente: 1, pause: 2, erreur: 3, telecharge: 4 };
      return rang[a.etat] - rang[b.etat] || a.ajoute_le.localeCompare(b.ajoute_le);
    }),
  );
  const enCours = $derived(liste.filter((j) => j.etat !== 'telecharge'));
  const finis = $derived(liste.filter((j) => j.etat === 'telecharge'));
  const pourcent = (r: number, t: number) => (t > 0 ? Math.floor((r / t) * 100) : 0);
  const ETATS: Record<string, string> = {
    en_cours: '⬇ En cours',
    attente: '⏳ En attente',
    pause: '⏸ En pause',
    erreur: '⛔ Échec',
    telecharge: '✅ Téléchargé',
  };
</script>

<div class="page">
  <section class="panel">
    <header>⬇ Téléchargements</header>
    <div class="corps">
      {#if enCours.length === 0}
        <p class="muted">
          Aucun téléchargement en cours. Pour en lancer un : « 🛒 Catalogue Firehouse », choisis un jeu, puis
          « ➕ Mettre dans ma ludothèque ».
        </p>
      {:else}
        <ul class="liste">
          {#each enCours as j (j.id)}
            <li class="cx-card element">
              <div class="haut">
                <button class="lien-titre" onclick={() => goto(`/jeu/${j.id}`)}>{j.titre}</button>
                <span class="tag {j.etat === 'erreur' ? 'err' : j.etat === 'pause' ? 'warn' : ''}">{ETATS[j.etat]}</span>
              </div>
              <div class="bar"><span style="width: {pourcent(j.recus, j.total)}%"></span></div>
              <p class="muted">
                {pourcent(j.recus, j.total)} % · {taille(j.recus)} sur {taille(j.total)} · {j.plateforme}
                {#if j.etat === 'en_cours' && j.debit}
                  · {taille(j.debit)}/s · encore {duree((j.total - j.recus) / j.debit)}
                {/if}
              </p>
              {#if j.message}<p class="muted">{j.message}</p>{/if}
              <p class="muted chemin">{j.dossier}</p>
              <div class="boutons">
                {#if j.etat === 'en_cours' || j.etat === 'attente'}
                  <button class="btn" onclick={() => mettreEnPause(j.id)}>⏸ Pause</button>
                {:else}
                  <button class="btn primary" onclick={() => reprendre(j.id)}>▶ Reprendre</button>
                {/if}
                <button class="btn danger" onclick={() => annuler(j.id)}>Annuler</button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </section>

  {#if finis.length}
    <section class="panel">
      <header>✅ Sur ce PC</header>
      <div class="corps">
        <ul class="finis">
          {#each finis as j (j.id)}
            <li>
              <button class="lien-titre" onclick={() => goto(`/jeu/${j.id}`)}>{j.titre}</button>
              <span class="muted">{j.plateforme} · {taille(j.total)} · {j.dossier}</span>
            </li>
          {/each}
        </ul>
      </div>
    </section>
  {/if}
</div>

<style>
  .page {
    padding: calc(16 * var(--u)) calc(24 * var(--u));
    display: grid;
    gap: calc(16 * var(--u));
    max-width: calc(1000 * var(--u));
    margin: 0 auto;
  }
  .corps {
    padding: calc(16 * var(--u));
  }
  .liste,
  .finis {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: calc(12 * var(--u));
  }
  .element {
    padding: calc(14 * var(--u));
    display: grid;
    gap: calc(8 * var(--u));
  }
  .element:hover {
    transform: none;
  }
  .element p {
    margin: 0;
  }
  .haut {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: calc(10 * var(--u));
  }
  .lien-titre {
    font: inherit;
    font-weight: 600;
    color: var(--ink);
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    text-align: left;
  }
  .lien-titre:hover {
    color: var(--accent-texte);
  }
  .chemin {
    font-family: var(--mono);
    font-size: calc(11.5 * var(--u));
  }
  .boutons {
    display: flex;
    gap: calc(8 * var(--u));
  }
  .finis li {
    display: grid;
    gap: calc(2 * var(--u));
  }
</style>
