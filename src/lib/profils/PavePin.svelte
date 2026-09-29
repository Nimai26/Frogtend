<script lang="ts">
  // Un pavé de code PIN : à la souris, au doigt (grandes touches), au clavier (chiffres, Retour, Entrée).
  // Les chiffres tapés ne s'affichent jamais : seulement des points.

  let {
    longueurMax = 8,
    libelleValider = 'Valider',
    onvalider,
    onannuler,
    erreur = '',
  }: {
    longueurMax?: number;
    libelleValider?: string;
    onvalider: (pin: string) => void;
    onannuler?: () => void;
    erreur?: string;
  } = $props();

  let pin = $state('');

  function taper(chiffre: string) {
    if (pin.length < longueurMax) pin += chiffre;
  }
  function effacer() {
    pin = pin.slice(0, -1);
  }
  function valider() {
    if (pin.length >= 4) {
      const p = pin;
      pin = '';
      onvalider(p);
    }
  }

  function surTouche(e: KeyboardEvent) {
    if (/^\d$/.test(e.key)) {
      e.preventDefault();
      taper(e.key);
    } else if (e.key === 'Backspace') {
      e.preventDefault();
      effacer();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      valider();
    } else if (e.key === 'Escape' && onannuler) {
      e.preventDefault();
      onannuler();
    }
  }

  const touches = ['1', '2', '3', '4', '5', '6', '7', '8', '9'];
</script>

<svelte:window onkeydown={surTouche} />

<div class="pave">
  <div class="points" aria-live="polite" aria-label="{pin.length} chiffre(s) tapé(s)">
    {#each { length: Math.max(4, pin.length) } as _, i (i)}
      <span class="point" class:plein={i < pin.length}></span>
    {/each}
  </div>
  {#if erreur}<p class="erreur" role="alert">{erreur}</p>{/if}
  <div class="touches">
    {#each touches as t (t)}
      <button class="btn touche" onclick={() => taper(t)}>{t}</button>
    {/each}
    <button class="btn touche" onclick={effacer} aria-label="Effacer le dernier chiffre">⌫</button>
    <button class="btn touche" onclick={() => taper('0')}>0</button>
    <button class="btn touche primary" onclick={valider} disabled={pin.length < 4} aria-label={libelleValider}>✔</button>
  </div>
  {#if onannuler}
    <button class="btn annuler" onclick={onannuler}>Annuler</button>
  {/if}
</div>

<style>
  .pave {
    display: grid;
    justify-items: center;
    gap: calc(14 * var(--u));
  }
  .points {
    display: flex;
    gap: calc(10 * var(--u));
    min-height: calc(18 * var(--u));
  }
  .point {
    width: calc(14 * var(--u));
    height: calc(14 * var(--u));
    border-radius: 50%;
    border: 2px solid var(--line);
  }
  .point.plein {
    background: var(--accent);
    border-color: var(--accent);
  }
  .erreur {
    margin: 0;
    color: var(--err-texte);
  }
  .touches {
    display: grid;
    grid-template-columns: repeat(3, calc(64 * var(--u)));
    gap: calc(10 * var(--u));
  }
  .touche {
    height: calc(56 * var(--u));
    font-size: calc(20 * var(--u));
  }
</style>
