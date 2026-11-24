// Lot S5 : les mots de la restauration des parties PC (fonctions pures).
import { describe, expect, it } from 'vitest';
import { colonnesConflit, ECART_SUR_S, lignesBilan, messageConflit, pcDuProfil, plusRecente, texteApercu, titresCites } from './reprise';
import type { BilanReprise, ConflitEmplacement } from '$lib/api';

const taille = (o: number) => `${o} o`;
const BILAN: BilanReprise = { jeux: 0, posees: 0, fichiers: 0, deja_la: 0, non_installes: 0, non_resolues: 0, identifiants: 0, en_cours: 0, conflits: 0, partielles: 0 };
const CONFLIT: ConflitEmplacement = {
  jeu: 7,
  cle: '0a1b2c3d',
  titre: 'Cyberpunk 2077',
  ici: { date: 1_795_600_000, octets: 11, fichiers: 2 },
  firehouse: { date: 1_795_517_589, octets: 20, fichiers: 3 },
  partiel: false,
};

describe('la plus récente', () => {
  it('seulement si les deux dates sont connues et nettement différentes', () => {
    expect(plusRecente(2000, 1000)).toBe('ici');
    expect(plusRecente(1000, 2000)).toBe('firehouse');
    expect(plusRecente(null, 2000)).toBeNull();
    expect(plusRecente(2000, null)).toBeNull();
    expect(plusRecente(1000, 1000 + ECART_SUR_S - 1)).toBeNull();
  });
});

describe('la fenêtre de conflit', () => {
  it('deux colonnes, date et taille, le badge du bon côté, jamais un chemin ni une clé', () => {
    const { ici, firehouse } = colonnesConflit(CONFLIT, taille);
    expect(ici.titre).toBe('💻 Sur ce PC');
    expect(firehouse.titre).toBe('🔥 Chez Firehouse');
    expect(ici.recente && !firehouse.recente).toBe(true);
    expect(ici.lignes[1]).toBe('11 o · 2 fichiers');
    expect(firehouse.lignes[1]).toBe('20 o · 3 fichiers');
    const tout = JSON.stringify([ici, firehouse]) + messageConflit(CONFLIT);
    expect(tout).not.toContain('0a1b2c3d');
    expect(tout).not.toMatch(/[A-Z]:\\/);
  });

  it('une date inconnue : aucun badge, et c’est dit', () => {
    const { ici, firehouse } = colonnesConflit({ ...CONFLIT, firehouse: { ...CONFLIT.firehouse, date: null } }, taille);
    expect(ici.recente || firehouse.recente).toBe(false);
    expect(firehouse.lignes[0]).toBe('Sauvegardée le date inconnue');
  });

  it('une copie partielle est dite', () => {
    expect(messageConflit({ ...CONFLIT, partiel: true })).toContain('incomplète');
    expect(messageConflit(CONFLIT)).not.toContain('incomplète');
  });
});

describe('le bilan', () => {
  it('posées, en attente (par raison), partielles, conflits ; rien : aucune ligne', () => {
    expect(lignesBilan(BILAN)).toEqual([]);
    const l = lignesBilan({ ...BILAN, posees: 2, fichiers: 5, non_installes: 1, identifiants: 1, non_resolues: 1, en_cours: 1, partielles: 1, conflits: 3 });
    expect(l[0]).toBe('✅ 2 parties remises à sa place (5 fichiers).');
    expect(l.some((x) => x.includes('installé'))).toBe(true);
    expect(l.some((x) => x.startsWith('⏳ 2 parties n’ont pas encore'))).toBe(true);
    expect(l.some((x) => x.startsWith('⏸ 1 partie attend'))).toBe(true);
    expect(l.some((x) => x.includes('OneDrive'))).toBe(true);
    expect(l.at(-1)).toBe('❓ 3 parties sont différentes de celle de ce PC : rien n’est remplacé sans ton choix.');
  });
});

describe('revue du lot S5', () => {
  it('une copie incomplète (reprise coupée) est dite', () => {
    expect(lignesBilan({ ...BILAN, incompletes: 2 })[0]).toContain('2 parties ne sont pas arrivées en entier');
  });
  it('les titres cités : 5, puis « et N autres »', () => {
    expect(titresCites(['A', 'B'])).toBe('A, B');
    expect(titresCites(['A', 'B', 'C', 'D', 'E', 'F'])).toBe('A, B, C, D, E et 1 autre');
  });
});

describe('l’annonce et le choix du PC', () => {
  it('dit les jeux, les fichiers et la taille', () => {
    expect(texteApercu({ jeux: 3, fichiers: 12, octets: 2048 }, taille)).toBe('À télécharger : 3 jeux, 12 fichiers, 2048 o.');
    expect(texteApercu({ jeux: 0, fichiers: 1, octets: 6 }, taille)).toBe('À télécharger : 1 fichier, 6 o.');
    expect(texteApercu(null, taille)).toBe('');
  });

  it('les sauvegardes de ce profil seulement, la plus récente d’abord', () => {
    const s = (pc: string, date: string | null, a_moi = true) => ({ pc, a_moi, derniere: date ? { date } : null });
    const l = pcDuProfil([s('ANCIEN', '1000'), s('AUTRE', '9000', false), s('VENKMAN', '5000'), s('SANS', null)]);
    expect(l.map((x) => x.pc)).toEqual(['VENKMAN', 'ANCIEN', 'SANS']);
  });
});
