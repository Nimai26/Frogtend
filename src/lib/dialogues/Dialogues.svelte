<script lang="ts">
  // Affiche la pile des fenêtres maison et les toasts. À poser une seule fois, dans la mise en page.
  import { annulerDessus, fermerToast, pile, type Dialogue } from './fenetres.svelte';
  import { retenirFocus } from './focus';

  // Texte en cours de saisie de la fenêtre « Demander », par identifiant de fenêtre.
  let saisies = $state<Record<number, string>>({});

  const dessus = $derived(pile.dialogues.at(-1));

  function valider(d: Dialogue) {
    if (d.sorte === 'confirmer') d.fermer(true);
    else if (d.sorte === 'demander') d.fermer(saisies[d.id] ?? d.valeur);
    else if (d.sorte === 'informer') d.fermer();
  }

  // Seule la fenêtre du dessus écoute le clavier.
  function surTouche(e: KeyboardEvent) {
    if (!dessus) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      annulerDessus();
    } else if (e.key === 'Enter' && dessus.sorte !== 'choisir') {
      const cible = e.target as HTMLElement;
      if (cible.tagName === 'TEXTAREA' || cible.tagName === 'BUTTON') return;
      e.preventDefault();
      valider(dessus);
    }
  }
</script>

<svelte:window onkeydown={surTouche} />

{#each pile.dialogues as d (d.id)}
  <div class="voile" role="presentation" onclick={(e) => e.target === e.currentTarget && d === dessus && annulerDessus()}>
    <div
      class="boite"
      role={d.sorte === 'confirmer' || d.sorte === 'informer' ? 'alertdialog' : 'dialog'}
      aria-modal="true"
      aria-labelledby="dlg-titre-{d.id}"
      inert={d !== dessus}
      use:retenirFocus
    >
      <h2 id="dlg-titre-{d.id}">{d.titre}</h2>
      {#if d.message}<p class="message">{d.message}</p>{/if}

      {#if d.sorte === 'demander'}
        {#if d.multiligne}
          <textarea rows="5" value={d.valeur} oninput={(e) => (saisies[d.id] = e.currentTarget.value)}></textarea>
        {:else}
          <input
            type={d.masque ? 'password' : 'text'}
            autocomplete="off"
            spellcheck="false"
            value={d.valeur}
            oninput={(e) => (saisies[d.id] = e.currentTarget.value)}
          />
        {/if}
      {/if}

      {#if d.sorte === 'choisir'}
        {#if d.options.length === 0}
          <p class="muted">Aucun choix possible pour l'instant.</p>
        {:else}
          <ul class="choix">
            {#each d.options as o, i (i)}
              <li>
                <button class="btn option" onclick={() => d.fermer(o.valeur)}>
                  <span>{o.libelle}</span>
                  {#if o.detail}<span class="muted">{o.detail}</span>{/if}
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      {/if}

      <div class="actions">
        {#if d.sorte === 'informer'}
          <button class="btn primary" data-valider onclick={() => d.fermer()}>OK</button>
        {:else}
          <button class="btn" data-premier={d.sorte === 'confirmer' && d.danger ? '' : undefined} onclick={annulerDessus}>Annuler</button>
          {#if d.sorte === 'confirmer'}
            <button class="btn {d.danger ? 'danger' : 'primary'}" data-valider onclick={() => d.fermer(true)}>
              {d.libelleValider}
            </button>
          {:else if d.sorte === 'demander'}
            <button class="btn primary" data-valider onclick={() => valider(d)}>{d.libelleValider}</button>
          {/if}
        {/if}
      </div>
    </div>
  </div>
{/each}

<div class="toasts" aria-live="polite">
  {#each pile.toasts as t (t.id)}
    <button class="toast {t.type}" onclick={() => fermerToast(t.id)}>{t.message}</button>
  {/each}
</div>

<style>
  .voile {
    position: fixed;
    inset: 0;
    background: var(--voile);
    display: grid;
    place-items: center;
    z-index: 100;
  }
  .boite {
    background: var(--panel);
    border: 1px solid color-mix(in srgb, var(--ink) 16%, transparent);
    border-radius: calc(14 * var(--u));
    padding: calc(18 * var(--u));
    width: min(calc(440 * var(--u)), 94vw);
    max-height: 86vh;
    overflow: auto;
    box-shadow: 0 calc(20 * var(--u)) calc(60 * var(--u)) var(--ombre-noire);
    backdrop-filter: blur(16px);
  }
  h2 {
    margin: 0 0 calc(10 * var(--u));
    font-size: calc(16 * var(--u));
  }
  .message {
    margin: 0 0 calc(14 * var(--u));
    white-space: pre-line;
  }
  textarea,
  input {
    margin-bottom: calc(14 * var(--u));
  }
  .choix {
    list-style: none;
    margin: 0 0 calc(14 * var(--u));
    padding: 0;
    display: grid;
    gap: calc(6 * var(--u));
  }
  .option {
    width: 100%;
    justify-content: space-between;
    text-align: left;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: calc(8 * var(--u));
    flex-wrap: wrap;
  }
  .toasts {
    position: fixed;
    left: 50%;
    bottom: calc(20 * var(--u));
    transform: translateX(-50%);
    display: grid;
    gap: calc(8 * var(--u));
    z-index: 200;
    width: min(calc(520 * var(--u)), 94vw);
  }
  .toast {
    font: inherit;
    text-align: left;
    background: var(--panel);
    color: var(--ink);
    border: 1px solid var(--line);
    border-left: calc(3 * var(--u)) solid var(--dim);
    border-radius: calc(10 * var(--u));
    padding: calc(10 * var(--u)) calc(14 * var(--u));
    min-height: var(--cible-min);
    box-shadow: 0 calc(8 * var(--u)) calc(24 * var(--u)) var(--ombre-noire);
    backdrop-filter: blur(16px);
    cursor: pointer;
  }
  .toast.ok { border-left-color: var(--ok); }
  .toast.alerte { border-left-color: var(--warn); }
  .toast.erreur { border-left-color: var(--err); }
</style>
