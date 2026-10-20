import { describe, expect, it } from 'vitest';
import type { JeuPc } from '$lib/api';
import { rangerParFiche } from './telechargements.svelte';

const jeu = (id: number, fiche: number | null, d_un_autre: boolean, ajoute_le: string): JeuPc =>
  ({ id, fiche, d_un_autre, ajoute_le, version: -1, titre: 't', plateforme: 'p', dossier: 'd', etat: 'telecharge', total: 0, fichiers: [], message: null, ajoute_par: 'x', installation: null, temps_jeu: 0, derniere_partie: null, recus: 0 }) as JeuPc;

describe('jeux importés reliés à une fiche (contrat 1.21, ✅ Seb 20/10)', () => {
  it('prend celui de mon profil d’abord, sinon le plus ancien ; jamais un jeu de Firehouse ni un import seul', () => {
    const r = rangerParFiche([
      jeu(-1, 110, true, '2026-10-01'),
      jeu(-2, 110, false, '2026-10-05'),
      jeu(-3, 120, true, '2026-10-09'),
      jeu(-4, 120, true, '2026-10-02'),
      jeu(-5, null, false, '2026-10-01'),
      jeu(130, 130, false, '2026-10-01'),
    ]);
    expect(r[110].id).toBe(-2);
    expect(r[120].id).toBe(-4);
    expect(Object.keys(r).sort()).toEqual(['110', '120']);
  });
});
