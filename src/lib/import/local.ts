// 📥 Importer des jeux déjà sur le disque (ROM, MS-DOS, Windows, ajout manuel). Rien n'est copié, déplacé ni
// renommé : Frogtend note où est le jeu. Retirer un jeu importé ne l'efface jamais du disque.

export interface RomTrouvee {
  chemin: string;
  titre: string;
  taille: number;
}

export interface JeuDosTrouve {
  dossier: string;
  titre: string;
  programmes: string[];
}

export interface JeuAImporter {
  titre: string;
  plateforme: string;
  dossier: string;
  fichier?: string | null;
  programme?: string | null;
}

export interface BilanImport {
  ajoutes: number;
  deja: number;
  refuses: [string, string][];
}

/** Le dossier et le nom d'un chemin Windows. */
export function couper(chemin: string): { dossier: string; nom: string } {
  const i = Math.max(chemin.lastIndexOf('\\'), chemin.lastIndexOf('/'));
  return i < 0 ? { dossier: '', nom: chemin } : { dossier: chemin.slice(0, i), nom: chemin.slice(i + 1) };
}

/** Les extensions que les émulateurs recommandés lisent (sans point, en minuscules, sans doublon). */
export function extensionsDe(emulateurs: { extensions?: string[] }[]): string[] {
  const l = emulateurs.flatMap((e) => e.extensions ?? []).map((x) => x.trim().replace(/^\./, '').toLowerCase()).filter(Boolean);
  return [...new Set(l)];
}

/** « sfc, smc .zip » → ['sfc', 'smc', 'zip']. */
export function lireExtensions(texte: string): string[] {
  return [...new Set(texte.split(/[\s,;]+/).map((x) => x.replace(/^\*?\./, '').toLowerCase()).filter(Boolean))];
}

export function depuisRoms(roms: RomTrouvee[], plateforme: string): JeuAImporter[] {
  return roms.map((r) => {
    const { dossier, nom } = couper(r.chemin);
    return { titre: r.titre, plateforme, dossier, fichier: nom };
  });
}

export function depuisDos(jeux: (JeuDosTrouve & { programme: string })[]): JeuAImporter[] {
  return jeux.map((j) => ({ titre: j.titre, plateforme: 'MS-DOS', dossier: j.dossier, fichier: j.programme }));
}

/** Un jeu Windows d'après son programme : le titre proposé est le nom de son dossier. */
export function depuisProgramme(programme: string, titre?: string): JeuAImporter {
  const { dossier } = couper(programme);
  return { titre: titre?.trim() || couper(dossier).nom || couper(programme).nom.replace(/\.exe$/i, ''), plateforme: 'Windows', dossier, programme };
}

/** Un ajout manuel : un jeu Windows lance son programme ; les autres donnent leur fichier à l'émulateur. */
export function depuisManuel(titre: string, plateforme: string, fichier: string): JeuAImporter {
  if (plateforme === 'Windows' || plateforme === 'Windows 3.X') return { ...depuisProgramme(fichier, titre), plateforme };
  const { dossier, nom } = couper(fichier);
  return { titre: titre.trim(), plateforme, dossier, fichier: nom };
}

export function messageBilan(b: BilanImport): string {
  const morceaux = [`✅ ${b.ajoutes} jeu(x) ajouté(s) à ta ludothèque`];
  if (b.deja) morceaux.push(`${b.deja} déjà dedans`);
  if (b.refuses.length) morceaux.push(`${b.refuses.length} refusé(s) : ${b.refuses.slice(0, 3).map(([t, m]) => `${t} (${m})`).join(', ')}`);
  return morceaux.join(' · ');
}
