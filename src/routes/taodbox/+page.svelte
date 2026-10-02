<script lang="ts">
  // Taodbox (lot 5) : le mode canapé. Les jeux de CE PC en grandes jaquettes, par console ; tout se fait à la manette
  // (croix, A, B) ou aux flèches. L'échelle ×2, le focus fort et la marge de la télé viennent de la charte (§ 7).
  import { goto } from '$app/navigation';
  import { onMount, tick } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import { api, type JeuPc } from '$lib/api';
  import { confirmer } from '$lib/dialogues/fenetres.svelte';
  import { etat, sortirDuProfil } from '$lib/etat.svelte';
  import { installer, jouer, partie } from '$lib/ludotheque/jeu.svelte';
  import Jaquette from '$lib/ludotheque/Jaquette.svelte';
  import { tele } from '$lib/ludotheque/telechargements.svelte';
  import { taodbox } from '$lib/taodbox/manette.svelte';
  import { adresseImageBoutique } from '$lib/api';
  import { lireSteam, ouvrirDansSteam, steam } from '$lib/boutiques/steam.svelte';

  let lanceParSunshine = $state(false);
  let console_ = $state<string | null>(null);

  const jeux = $derived(
    Object.values(tele.jeux)
      .filter((j: JeuPc) => j.etat === 'telecharge')
      .sort((a, b) => (b.derniere_partie ?? '').localeCompare(a.derniere_partie ?? '') || a.titre.localeCompare(b.titre)),
  );
  const consoles = $derived([...new Set(jeux.map((j) => j.plateforme))].sort());
  const affiches = $derived(console_ ? jeux.filter((j) => j.plateforme === console_) : jeux);

  /** Les jeux Steam installés sur ce PC, jouables depuis le canapé (Steam les lance). */
  const steamInstalles = $derived(steam.jeux.filter((j) => j.installe));

  onMount(async () => {
    lireSteam();
    lanceParSunshine = isTauri() ? await api.taodboxLance().catch(() => false) : false;
    await tick();
    document.querySelector<HTMLElement>('.taodbox .carte')?.focus();
  });

  async function jeu(j: JeuPc) {
    if (partie.enJeu !== null) return;
    if (!j.installation) {
      if (!(await installer(j.id))) return;
    }
    await jouer(j.id);
  }

  async function bureau() {
    await goto('/');
  }

  async function quitter() {
    if (!(await confirmer('⏻ Quitter Taodbox ?', { libelleValider: 'Quitter' }))) return;
    const { exit } = await import('@tauri-apps/plugin-process');
    await exit(0);
  }
</script>

<div class="taodbox">
  <header>
    <img class="logo" src="/grenouille.png" alt="" />
    <h1>Taodbox</h1>
    <span class="qui">{etat.profilOuvert?.nom}</span>
    <span class="espace"></span>
    <button class="btn" onclick={() => sortirDuProfil()}>👥 Changer de joueur</button>
    {#if lanceParSunshine}
      <button class="btn" onclick={quitter}>⏻ Quitter</button>
    {:else}
      <button class="btn" onclick={bureau}>🖥 Bureau</button>
    {/if}
  </header>

  {#if consoles.length > 1}
    <nav class="consoles" aria-label="Consoles">
      <button class="btn" class:choisie={console_ === null} onclick={() => (console_ = null)}>Tout</button>
      {#each consoles as c (c)}
        <button class="btn" class:choisie={console_ === c} onclick={() => (console_ = c)}>{c}</button>
      {/each}
    </nav>
  {/if}

  {#if affiches.length === 0}
    <p class="vide">Aucun jeu sur ce PC pour l’instant. Ajoute-en depuis le bureau (Catalogue Firehouse).</p>
  {:else}
    <div class="grille">
      {#each affiches as j (j.id)}
        <button class="carte" onclick={() => jeu(j)} disabled={partie.enJeu !== null && partie.enJeu !== j.id} title={j.titre}>
          <Jaquette id={j.id} titre={j.titre} plateforme={j.plateforme} largeur={180} />
          <span class="titre">{j.titre}</span>
          <span class="etat">{partie.enJeu === j.id ? '▶ En cours' : j.installation ? '' : '📦 À installer'}</span>
        </button>
      {/each}
    </div>
  {/if}

  {#if steamInstalles.length && (console_ === null || console_ === 'Steam')}
    <h2 class="section">Steam</h2>
    <div class="grille">
      {#each steamInstalles as j (j.id)}
        <button class="carte" onclick={() => ouvrirDansSteam(j, 'jouer')} title={j.nom}>
          <span class="image-steam"><img src={adresseImageBoutique('steam', j.id)} alt="" loading="lazy" /></span>
          <span class="titre">{j.nom}</span>
          <span class="etat">Steam</span>
        </button>
      {/each}
    </div>
  {/if}

  <footer class="aide">
    {#if taodbox.manettes}🎮 Croix pour choisir · A pour jouer · B pour revenir{:else}↑ ↓ ← → pour choisir · Entrée pour jouer · Échap pour revenir{/if}
    · Pendant le jeu : {etat.pc.menuJeu.touche === 'Pause' ? 'Pause/Attn' : etat.pc.menuJeu.touche} pour le menu
  </footer>
</div>

<style>
  /* Marge de sécurité de la télé : environ 5 % (charte § 7). */
  .taodbox {
    box-sizing: border-box;
    height: 100vh;
    padding: 5vh 5vw;
    display: flex;
    flex-direction: column;
    gap: calc(16 * var(--u));
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    gap: calc(14 * var(--u));
  }
  .logo {
    height: calc(36 * var(--u));
  }
  h1 {
    margin: 0;
    font-size: calc(26 * var(--u));
  }
  .qui {
    color: var(--dim);
  }
  .espace {
    flex: 1;
  }
  .consoles {
    display: flex;
    gap: calc(8 * var(--u));
    flex-wrap: wrap;
  }
  .consoles .choisie {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    border-color: var(--accent);
  }
  .grille {
    flex: 1;
    overflow: auto;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(calc(180 * var(--u)), 1fr));
    gap: calc(18 * var(--u));
    align-content: start;
    padding: calc(8 * var(--u));
  }
  .carte {
    display: grid;
    gap: calc(6 * var(--u));
    padding: calc(6 * var(--u));
    background: none;
    border: calc(2 * var(--u)) solid transparent;
    border-radius: calc(10 * var(--u));
    color: var(--ink);
    text-align: left;
    cursor: pointer;
    transition: transform 0.08s;
  }
  /* Focus fort et toujours visible (charte § 7). */
  .carte:focus-visible,
  .carte:focus {
    outline: calc(4 * var(--u)) solid var(--accent);
    outline-offset: calc(4 * var(--u));
    border-color: var(--accent);
    transform: scale(1.05);
  }
  .titre {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .etat {
    color: var(--dim);
    min-height: 1.2em;
  }
  .section {
    margin: 0;
    font-size: calc(18 * var(--u));
  }
  .image-steam {
    aspect-ratio: 2 / 3;
    border-radius: calc(6 * var(--u));
    overflow: hidden;
    background: color-mix(in srgb, var(--ink) 8%, transparent);
  }
  .image-steam img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .vide {
    color: var(--dim);
    font-size: calc(18 * var(--u));
  }
  .aide {
    color: var(--dim);
    text-align: center;
  }
</style>
