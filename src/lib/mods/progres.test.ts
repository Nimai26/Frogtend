// La progression d'une opération sur les mods (`avecProgres`, événement `mod-progres` du cœur) : l'écoute posée AVANT
// le travail, seuls les événements de CE suivi comptent, l'étape, la barre et « N fichiers sur 2 015 » suivent, le
// départ connu se montre tout de suite (0.227.1, défaut vu par Seb le 25/11), et le message se ferme quoi qu'il arrive.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render } from '@testing-library/svelte';
import { tick } from 'svelte';
import type { ProgresMod } from './mods';

type Rappel = (e: { payload: ProgresMod }) => void;
const ecoutes: { nom: string; rappel: Rappel }[] = [];
const arrets = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (nom: string, rappel: Rappel) => {
    ecoutes.push({ nom, rappel });
    return () => arrets(nom);
  }),
}));

import Dialogues from '$lib/dialogues/Dialogues.svelte';
import { pile } from '$lib/dialogues/fenetres.svelte';
import { avecProgres, etatProgres } from './progres.svelte';

const fr = (n: number) => n.toLocaleString('fr-FR');
const emettre = (p: ProgresMod) => ecoutes.filter((e) => e.nom === 'mod-progres').forEach((e) => e.rappel({ payload: p }));
const toasts = () => JSON.parse(JSON.stringify(pile.toasts));

describe('avecProgres', () => {
  beforeEach(() => {
    ecoutes.length = 0;
    arrets.mockClear();
    pile.toasts = [];
  });
  afterEach(() => cleanup());

  it('écoute avant le travail, montre l’étape, la barre et « 504 fichiers sur 2 015 » à l’écran, puis ferme le message', async () => {
    render(Dialogues);
    let vu: unknown = null;
    let texte = '';
    let largeur = '';
    const r = await avecProgres('🧹 Retrait', false, async (suivi) => {
      expect(ecoutes.map((e) => e.nom)).toEqual(['mod-progres']);
      emettre({ suivi: suivi + 1000, etape: 'retrait', faits: 9, total: 10, debit: 0 });
      emettre({ suivi, etape: 'retrait', faits: 504, total: 2015, debit: 0 });
      vu = toasts();
      await tick();
      texte = document.querySelector('.toast.progres')?.textContent ?? '';
      largeur = document.querySelector<HTMLElement>('.toast.progres .bar > span')?.style.width ?? '';
      return 'fini';
    });
    expect(r).toBe('fini');
    expect(vu).toEqual([expect.objectContaining({ message: '🧹 Retrait', progres: { etape: 'Rangement des mods hors du jeu', chiffres: `504 fichiers sur ${fr(2015)}`, pourcent: 25 } })]);
    expect(texte).toContain(`504 fichiers sur ${fr(2015)}`);
    expect(largeur).toBe('25%');
    expect(pile.toasts).toEqual([]);
    expect(arrets).toHaveBeenCalledWith('mod-progres');
  });

  it('le départ connu se montre AVANT le premier événement du cœur (jamais une barre muette)', async () => {
    let vu: unknown = null;
    await avecProgres(
      '🧹 Retrait',
      false,
      async () => {
        vu = toasts();
      },
      { etape: 'verification', total: 2014 },
    );
    expect(vu).toEqual([expect.objectContaining({ progres: { etape: 'Vérification des fichiers des mods', chiffres: `0 fichier sur ${fr(2014)}`, pourcent: 0 } })]);
    // Une reprise : la barre part du vrai compte.
    await avecProgres(
      '▶ Reprise',
      false,
      async () => {
        vu = toasts();
      },
      { etape: 'retrait', total: 2014, faits: 811 },
    );
    expect(vu).toEqual([expect.objectContaining({ progres: { etape: 'Rangement des mods hors du jeu', chiffres: `811 fichiers sur ${fr(2014)}`, pourcent: 40 } })]);
    // Sans départ (ou un total inconnu) : rien d'inventé.
    await avecProgres('x', false, async () => {
      vu = toasts();
    });
    expect(vu).toEqual([expect.objectContaining({ progres: { etape: '', chiffres: '', pourcent: 0 } })]);
  });

  it('un échec du travail ferme aussi le message et l’écoute', async () => {
    await expect(
      avecProgres('x', false, async () => {
        throw new Error('non');
      }),
    ).rejects.toThrow('non');
    expect(pile.toasts).toEqual([]);
    expect(arrets).toHaveBeenCalledTimes(1);
  });

  it('lot C4c-2 : l’appelant suit les étapes de SON suivi seulement ; l’installation se dit sans « 0 o »', async () => {
    const vus: string[] = [];
    let vu: unknown = null;
    await avecProgres(
      '⬇ Installation',
      true,
      async (suivi) => {
        emettre({ suivi: suivi + 1, etape: 'installation', faits: 0, total: 0, debit: 0 });
        emettre({ suivi, etape: 'installation', faits: 0, total: 0, debit: 0 });
        vu = toasts();
      },
      undefined,
      (p) => vus.push(p.etape),
    );
    expect(vus).toEqual(['installation']);
    expect(vu).toEqual([expect.objectContaining({ progres: { etape: 'Installation', chiffres: 'ça peut prendre quelques minutes', pourcent: 0 } })]);
  });

  it('etatProgres : « environ » seulement pour un téléchargement approximatif', () => {
    expect(etatProgres({ etape: 'remise', faits: 2, total: 12, debit: 0 }, true)).toEqual({ etape: 'Remise dans le jeu', chiffres: '2 fichiers sur 12', pourcent: 17 });
    expect(etatProgres({ etape: 'telechargement', faits: 0, total: 1024, debit: 0 }, true).chiffres).toContain('environ');
  });
});
