<script lang="ts">
  // Les emplacements des jeux sur CE PC : un ou plusieurs dossiers par défaut, et, si besoin, des dossiers propres
  // à un système (SNES, MS-DOS…), dans l'ordre de préférence. Retirer un emplacement n'efface AUCUN fichier.
  import { onMount } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import { api, taille, type Plateforme } from '$lib/api';
  import { choisir, demander, toast } from '$lib/dialogues/fenetres.svelte';
  import { etat, reglerPc } from '$lib/etat.svelte';

  const e = $derived(etat.pc.emplacements);
  let libres = $state<Record<string, number | null>>({});
  let plateformes = $state<Plateforme[]>([]);

  async function mesurer(chemins: string[]) {
    for (const c of chemins) {
      libres[c] = await api.espaceLibre(c).catch(() => null);
    }
  }
  $effect(() => {
    mesurer([...e.defaut, ...Object.values(e.systemes).flat()]);
  });
  onMount(async () => {
    plateformes = await api.plateformes(false).catch(() => []);
  });

  /** Demande un dossier : le sélecteur de Windows, ou un chemin tapé à la main. */
  async function demanderDossier(): Promise<string | null> {
    const facon = await choisir('📁 Ajouter un emplacement', [
      { valeur: 'parcourir', libelle: '📂 Choisir un dossier', detail: 'avec l’explorateur de Windows' },
      { valeur: 'taper', libelle: '✏ Taper le chemin', detail: 'par exemple E:\\Jeux ou \\\\NAS\\Jeux' },
    ]);
    if (facon === 'parcourir' && isTauri()) {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const r = await open({ directory: true, multiple: false, title: 'Où ranger les jeux ?' });
      return typeof r === 'string' ? r : null;
    }
    if (facon === 'taper') {
      const c = await demander('✏ Chemin du dossier', { valeur: 'D:\\Jeux' });
      return c?.trim() || null;
    }
    return null;
  }

  async function ajouter(systeme: string | null) {
    const c = await demanderDossier();
    if (!c) return;
    const libre = await api.espaceLibre(c).catch(() => null);
    if (libre === null) {
      toast(`Refusé : le dossier « ${c} » est introuvable. Crée-le d’abord, ou vérifie que le disque est branché.`, 'erreur');
      return;
    }
    if (systeme === null) {
      if (e.defaut.includes(c)) return;
      await reglerPc('emplacements.defaut', [...e.defaut, c]);
    } else {
      const liste = e.systemes[systeme] ?? [];
      if (liste.includes(c)) return;
      await reglerPc('emplacements.systemes', { ...e.systemes, [systeme]: [...liste, c] });
    }
    toast(`✅ Emplacement ajouté : ${c} (${taille(libre)} libres).`);
  }

  async function modifier(systeme: string | null, liste: string[]) {
    if (systeme === null) await reglerPc('emplacements.defaut', liste);
    else {
      const s = { ...e.systemes };
      if (liste.length === 0) delete s[systeme];
      else s[systeme] = liste;
      await reglerPc('emplacements.systemes', s);
    }
  }
  const monter = (l: string[], i: number) => (i > 0 ? [...l.slice(0, i - 1), l[i], l[i - 1], ...l.slice(i + 1)] : l);
  const retirer = (l: string[], i: number) => l.filter((_, k) => k !== i);

  async function ajouterSysteme() {
    const deja = new Set(Object.keys(e.systemes));
    const s = await choisir(
      '🎮 Pour quel système ?',
      plateformes.filter((p) => !deja.has(p.nom)).map((p) => ({ valeur: p.nom, libelle: p.nom, detail: `${p.jeux} jeu(x)` })),
      'Ses jeux iront dans ses propres dossiers au lieu des dossiers par défaut.',
    );
    if (s) await ajouter(s);
  }
</script>

{#snippet liste(systeme: string | null, chemins: string[])}
  {#if chemins.length === 0}
    <p class="muted">Aucun dossier.</p>
  {:else}
    <ol class="chemins">
      {#each chemins as c, i (c)}
        <li>
          <span class="chemin">{c}</span>
          <span class="muted">
            {libres[c] === undefined ? '…' : libres[c] === null ? '⚠ introuvable' : `${taille(libres[c] ?? 0)} libres`}
          </span>
          <span class="boutons">
            {#if i > 0}
              <button class="btn petit" title="Préférer ce dossier" onclick={() => modifier(systeme, monter(chemins, i))}>↑</button>
            {/if}
            <button class="btn petit" title="Retirer (aucun fichier n’est effacé)" onclick={() => modifier(systeme, retirer(chemins, i))}>✕</button>
          </span>
        </li>
      {/each}
    </ol>
  {/if}
{/snippet}

<div class="emplacements">
  <p class="muted">
    Où ranger les jeux sur ce PC. Frogtend prend le premier dossier qui a assez de place. Retirer un dossier d’ici
    n’efface aucun fichier.
  </p>

  <div class="cx-block">
    <h3>Par défaut (tous les systèmes)</h3>
    {@render liste(null, e.defaut)}
    <button class="btn" onclick={() => ajouter(null)}>➕ Ajouter un dossier</button>
  </div>

  {#each Object.entries(e.systemes) as [systeme, chemins] (systeme)}
    <div class="cx-block">
      <h3>{systeme}</h3>
      {@render liste(systeme, chemins)}
      <button class="btn" onclick={() => ajouter(systeme)}>➕ Ajouter un dossier pour {systeme}</button>
    </div>
  {/each}

  <button class="btn" onclick={ajouterSysteme} disabled={plateformes.length === 0}>
    🎮 Des dossiers propres à un système…
  </button>
</div>

<style>
  .emplacements {
    display: grid;
    gap: calc(12 * var(--u));
  }
  .emplacements > .btn,
  .cx-block > .btn {
    justify-self: start;
  }
  .cx-block {
    display: grid;
    gap: calc(8 * var(--u));
  }
  .cx-block > h3 {
    margin-bottom: 0;
  }
  .muted {
    margin: 0;
  }
  .chemins {
    margin: 0;
    padding-left: calc(20 * var(--u));
    display: grid;
    gap: calc(6 * var(--u));
  }
  .chemins li {
    display: grid;
    grid-template-columns: 1fr auto auto;
    align-items: center;
    gap: calc(10 * var(--u));
  }
  .chemin {
    font-family: var(--mono);
    font-size: calc(12.5 * var(--u));
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .boutons {
    display: flex;
    gap: calc(4 * var(--u));
  }
  .petit {
    min-height: calc(30 * var(--u));
    padding: calc(2 * var(--u)) calc(9 * var(--u));
  }
</style>
