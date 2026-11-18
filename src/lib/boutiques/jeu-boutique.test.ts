import { describe, expect, it } from 'vitest';
import type { JeuResume } from '$lib/api';
import { messageLancementBoutique, boutiqueDesinstallee, estBattlenet, MESSAGE_BATTLENET, toujoursLeMeme, BOUTIQUES, estJeuDeBoutique, libelleOuverture, ligneEtat, messageNouveaux, moyenDOuverture, nomBoutique } from './jeu-boutique';

const jeu = (o: Partial<JeuResume>): JeuResume =>
  ({ id: -5, titre: 'X', annee: null, plateforme: 'Windows', genres: [], developpeur: null, editeur: null, statut: 'boutique', ...o }) as JeuResume;

describe('jeux de boutique', () => {
  it('se reconnaissent à leur id négatif', () => {
    expect(estJeuDeBoutique({ id: -3 })).toBe(true);
    expect(estJeuDeBoutique({ id: 110 })).toBe(false);
  });

  it('s’ouvrent par Steam, par GOG Galaxy, ou par le lanceur de leur boutique', () => {
    expect(moyenDOuverture(jeu({ source: 'steam', cle_boutique: '620' }))).toEqual({ par: 'steam', appid: '620' });
    expect(moyenDOuverture(jeu({ source: 'galaxy', cle_boutique: 'epic_Fortnite' }))).toEqual({ par: 'galaxy', cle: 'epic_Fortnite' });
    expect(moyenDOuverture(jeu({ source: 'galaxy' }))).toBeNull();
    expect(moyenDOuverture(jeu({ source: 'uplay', boutique: 'uplay', cle_boutique: 'uplay_46' }))).toEqual({ par: 'boutique', boutique: 'uplay', cle: 'uplay_46' });
    expect(moyenDOuverture(jeu({ source: 'gog', boutique: 'gog', cle_boutique: 'gog_1' }))).toEqual({ par: 'boutique', boutique: 'gog', cle: 'gog_1' });
  });

  it('nomment le lanceur d’après leur boutique, jamais un nom écrit en dur', () => {
    expect(libelleOuverture(jeu({ source: 'uplay', boutique: 'uplay', cle_boutique: 'uplay_46' }))).toBe('Ubisoft Connect');
    expect(libelleOuverture(jeu({ source: 'amazon', boutique: 'amazon', cle_boutique: 'amazon_x' }))).toBe('Amazon Games');
    expect(libelleOuverture(jeu({ source: 'galaxy', boutique: 'epic', cle_boutique: 'epic_F' }))).toBe('GOG Galaxy');
    expect(libelleOuverture(jeu({ source: 'steam', boutique: 'steam', cle_boutique: '620' }))).toBe('Steam');
    expect(nomBoutique('battlenet')).toBe('Battle.net');
    expect(nomBoutique('inconnue')).toBe('sa boutique');
    expect(BOUTIQUES.map((b) => b.valeur)).toEqual(expect.arrayContaining(['amazon', 'battlenet']));
  });

  it('annoncent en UN toast les jeux nouveaux, et rien s’il n’y en a pas', () => {
    expect(messageNouveaux([['epic', 2], ['uplay', 1]])).toBe('🎮 3 nouveaux jeux trouvés sur ce PC (Epic 2, Ubisoft 1)');
    expect(messageNouveaux([['gog', 1]])).toBe('🎮 1 nouveau jeu trouvé sur ce PC (GOG 1)');
    expect(messageNouveaux([])).toBeNull();
    expect(messageNouveaux([['steam', 0]])).toBeNull();
  });

  it('disent l’état de chaque boutique sans mot technique', () => {
    expect(ligneEtat({ nom: 'Epic Games', lanceur: true, lu: true, installes: 12 })).toBe('Epic Games : 12 jeux sur ce PC, relus au démarrage');
    expect(ligneEtat({ nom: 'Amazon Games', lanceur: false, lu: false, installes: 0 })).toBe('Amazon Games : lanceur absent');
    expect(ligneEtat({ nom: 'Epic Games', lanceur: true, lu: false, installes: 0 })).toBe('Epic Games : jeux illisibles pour l’instant');
    expect(ligneEtat({ nom: 'Ubisoft Connect', lanceur: true, lu: true, installes: 0 })).toContain('aucun jeu installé');
    for (const l of [ligneEtat({ nom: 'Steam', lanceur: true, lu: true, installes: 1 })]) expect(l).not.toMatch(/manifeste|registre|product\.db/);
  });

  it('ne font rien pour un autre profil ouvert pendant l’attente', () => {
    expect(toujoursLeMeme('seb', 'seb')).toBe(true);
    expect(toujoursLeMeme('seb', 'lou')).toBe(false);
    expect(toujoursLeMeme('seb', undefined)).toBe(false);
    expect(toujoursLeMeme(undefined, undefined)).toBe(false);
  });

  it('Battle.net : jamais un ▶ Jouer qui échoue, lanceur présent ou non', () => {
    expect(estBattlenet({ boutique: 'battlenet', source: 'battlenet' })).toBe(true);
    expect(estBattlenet({ installation: { boutique: { source: 'battlenet' } } })).toBe(true);
    expect(estBattlenet({ boutique: 'battlenet', source: 'galaxy' })).toBe(false);
    expect(estBattlenet({ boutique: 'uplay', source: 'uplay' })).toBe(false);
    expect(MESSAGE_BATTLENET).toContain('ouvre-le depuis Battle.net');
  });

  it('reconnaissent une ligne du PC désinstallée hors de Frogtend (Taodbox ne la montre pas)', () => {
    expect(boutiqueDesinstallee({ id: -9, installation: null, ajoute_par: 'uplay' })).toBe(true);
    expect(boutiqueDesinstallee({ id: -9, installation: { dossier: 'x' }, ajoute_par: 'uplay' })).toBe(false);
    expect(boutiqueDesinstallee({ id: -9, installation: null, ajoute_par: 'local' })).toBe(false);
    expect(boutiqueDesinstallee({ id: 110, installation: null, ajoute_par: 'steam' })).toBe(false);
  });

  it('n’annoncent le lancement qu’en UN toast, avec le retour par ⊞/Guide au canapé', () => {
    expect(messageLancementBoutique('Epic Games', 'TerraScape', false)).toBe('▶ Epic Games lance « TerraScape »… S’il fait une mise à jour, ça peut prendre quelques minutes.');
    expect(messageLancementBoutique('Epic Games', 'TerraScape', true)).toContain('appuie sur ⊞/Guide pour revenir à Frogtend');
  });
});
