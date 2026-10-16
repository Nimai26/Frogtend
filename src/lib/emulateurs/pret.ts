// « Prêt à jouer » (décision de Seb, 03/10) : Frogtend est pour des gens qui veulent jouer, pas pour des connaisseurs
// de l'émulation. Quand une console n'a pas d'émulateur, Frogtend choisit seul celui que Firehouse recommande, et
// demande UN accord qui dit tout (quoi, combien, où, la manette, la première partie plus longue). Ici : les décisions,
// sans effet de bord (testées) ; l'enchaînement est dans `pret.svelte.ts`.

import type { BiosConseil, Conseil, EmulateurRecommande } from '$lib/api';

/** Un émulateur possible pour une console, tel que Frogtend le voit. */
export interface Possible {
  rec: EmulateurRecommande;
  /** L'identifiant que Frogtend connaît (il sait l'installer), sinon `null`. */
  id: string | null;
  nom: string;
  /** Déjà sur ce PC : son programme. */
  programme?: string;
  /** Firehouse donne sa source (contrat 14) : Frogtend peut l'installer même sans le connaître. */
  parFirehouse?: boolean;
}

/** Ce que Frogtend fera seul. */
export type Choix = Possible & { comment: 'deja' | 'frogtend' | 'firehouse' };

/** Cette ligne de la liste est-elle l'émulateur du conseil de la maison ? (Par l'id des sources de Firehouse.) */
export function estConseille(rec: EmulateurRecommande, conseil: Conseil | null | undefined): boolean {
  return !!conseil?.emulateur && rec.id === conseil.emulateur;
}

/**
 * La ligne de commande retenue (contrat 1.8). Pour l'émulateur du conseil : les options de Frogtend pour cet émulateur
 * (plein écran, sans interface…), puis la ligne DU CONSEIL (son cœur, `%rom%`) — JAMAIS celle de LaunchBox. Sinon (pas
 * de conseil, ou un autre émulateur choisi) : la ligne LaunchBox, sinon celle de Frogtend, comme avant.
 */
export function ligneDe(rec: EmulateurRecommande, ligneFrogtend: string, conseil: Conseil | null | undefined): string {
  if (!estConseille(rec, conseil)) return rec.ligne_de_commande || ligneFrogtend;
  const c = conseil!.ligne_de_commande.trim();
  const f = ligneFrogtend.trim();
  // Les options de Frogtend vont JUSTE AVANT le jeu : certaines finissent par une option qui attend le fichier du jeu
  // (« -- » de PCSX2/DuckStation, « -e » de Dolphin, « -g » de Cemu/Eden). Avec `%rom%` : devant lui ; sans : après la
  // ligne du conseil (le jeu sera ajouté à la fin).
  const i = c.search(/"?%rom%/i);
  if (i < 0) return [c, f].filter(Boolean).join(' ');
  return [c.slice(0, i).trim(), f, c.slice(i).trim()].filter(Boolean).join(' ');
}

/** Les fichiers système de `system/` que RetroArch doit trouver (vérifiés sur le disque après l'installation) : ceux
 * du conseil, obligatoires (jamais une clé ni un firmware à installer) ; sans conseil, la chaîne LaunchBox. */
export function biosRetroArch(rec: EmulateurRecommande, conseil: Conseil | null | undefined): string[] {
  if (estConseille(rec, conseil)) {
    return conseil!.bios
      .filter((b) => b.racine === 'system_retroarch' && b.type === 'fichier' && !b.facultatif && !b.coffre)
      .map((b) => b.chemin || b.fichier);
  }
  // Avec un conseil, seul `conseil.bios` fait foi (Firehouse, 16/10) : la chaîne LaunchBox n'a plus cours.
  return !conseil && rec.bios ? [rec.bios] : [];
}

/**
 * La liste des émulateurs d'un système avec le conseil de la maison : l'émulateur conseillé EN TÊTE, qu'il soit une
 * ligne de la liste, un de ses forks (jamais écarté pour `actif`, contrat : le conseil prime), ou absent de la liste —
 * il est alors décrit d'après le conseil (relevé réel du 16/10 : MS-DOS conseille RetroArch, la liste ne donne que
 * DOSBox Staging). Puis le reste, le recommandé de LaunchBox d'abord. Sans conseil : la liste telle quelle.
 */
export function avecConseil(recs: EmulateurRecommande[], conseil: Conseil | null | undefined): EmulateurRecommande[] {
  if (!conseil?.emulateur) return recs;
  const fork = recs.flatMap((r) => r.forks ?? []).find((f) => f.id === conseil.emulateur);
  const conseille: EmulateurRecommande = recs.find((r) => estConseille(r, conseil)) ??
    (fork ? { ...fork, forks: [] } : null) ?? {
      nom: conseil.emulateur,
      site: '',
      recommande: false,
      ligne_de_commande: '',
      extensions: [],
      bios: '',
      id: conseil.emulateur,
    };
  const autres = recs.filter((r) => r !== conseille).sort((a, b) => Number(b.recommande) - Number(a.recommande));
  return [conseille, ...autres];
}

/** Le premier émulateur que Frogtend peut préparer seul : déjà là, sinon installable. L'émulateur du conseil de la
 * maison d'abord (il PRIME, contrat 1.8) ; ensuite (ou sans conseil) le recommandé de LaunchBox d'abord. */
export function choisirSeul(possibles: Possible[], conseil?: Conseil | null): Choix | null {
  const ordre = [...possibles].sort(
    (a, b) => Number(estConseille(b.rec, conseil)) - Number(estConseille(a.rec, conseil)) || Number(b.rec.recommande) - Number(a.rec.recommande),
  );
  for (const p of ordre) {
    if (p.programme) return { ...p, comment: 'deja' };
    if (p.id) return { ...p, comment: 'frogtend' };
    if (p.parFirehouse) return { ...p, comment: 'firehouse' };
  }
  return null;
}

/** Le dossier des émulateurs proposé d'office : « Emulateurs » à la racine du disque des jeux (sinon E:). */
export function dossierParDefaut(emplacementsJeux: string[]): string {
  const lecteur = /^([a-z]):/i.exec(emplacementsJeux[0] ?? '')?.[1]?.toUpperCase() ?? 'E';
  return `${lecteur}:\\Emulateurs`;
}

/** Les émulateurs dont Frogtend règle la manette (une manette Xbox standard, ou toute manette reconnue). */
export const MANETTE_REGLEE = ['retroarch', 'duckstation', 'pcsx2', 'dolphin', 'rpcs3'];

/** Ceux qui préparent longuement la première partie de chaque jeu (compilation, une seule fois). */
const PREMIERE_PARTIE_LONGUE = ['rpcs3', 'cemu', 'eden', 'xenia', 'yuzu', 'suyu', 'ryujinx'];

/** Les fichiers système que Frogtend téléchargera (obligatoires, servis, ni clé ni firmware). */
export function biosATelecharger(bios: BiosConseil[]): BiosConseil[] {
  // Même critère que le cœur (`bios::posable`).
  return bios.filter((b) => !b.facultatif && !b.coffre && b.disponible && (b.type === 'fichier' || b.type === 'archive') && !!b.sha1 && !!b.url);
}

/** Leur poids (0 si Firehouse ne dit pas la taille : il ne sert qu'au texte). */
export function poidsBios(bios: BiosConseil[]): number {
  return biosATelecharger(bios).reduce((s, b) => s + (b.taille ?? 0), 0);
}

/** Le conseil a-t-il quelque chose à dire ou à poser (fichiers servis, à poser soi-même, clés) ? */
export function biosADireOuPoser(bios: BiosConseil[]): boolean {
  return bios.some((b) => !b.facultatif && b.type !== 'installation');
}

/** Les fichiers système obligatoires d'un conseil, pour le dire dans l'accord : ceux que Firehouse fournit, et les
 * clés que personne ne peut fournir (jamais servies). Les firmwares à installer par l'émulateur sont dits à part. */
export function biosADire(bios: BiosConseil[]): { fournis: string[]; aPoser: string[]; cles: string[] } {
  const obligatoires = bios.filter((b) => !b.facultatif && b.type !== 'installation');
  // Un motif (« *.bin ») n'est pas un nom de fichier : on dit sa note, sinon « fichiers système ».
  const nom = (b: BiosConseil) => (b.type === 'motif' ? b.note || 'fichiers système' : b.fichier);
  return {
    // Un motif n'est pas encore posé par Frogtend (aucun servi au 16/10) : jamais promis.
    fournis: biosATelecharger(obligatoires).map(nom),
    aPoser: obligatoires.filter((b) => !b.coffre && !biosATelecharger([b]).length).map(nom),
    cles: obligatoires.filter((b) => b.coffre).map((b) => b.fichier),
  };
}

/** Le texte de l'accord unique. */
export function messagePret(o: {
  plateforme: string;
  jeux: number;
  choix: Choix;
  taille: string | null;
  dossier: string;
  dossierNouveau: boolean;
  /** Le conseil de la maison, s'il y en a un : ses fichiers système remplacent la chaîne LaunchBox. */
  conseil?: Conseil | null;
  /** La taille des fichiers système à télécharger (déjà mise en forme). */
  tailleBios?: string | null;
}): string {
  const { choix } = o;
  const pour = o.jeux > 1 ? `tes ${o.jeux} jeux ${o.plateforme}` : `ce jeu ${o.plateforme}`;
  const lignes = [
    choix.comment === 'deja'
      ? `Pour jouer à ${pour}, Frogtend utilise ${choix.nom}, déjà installé sur ce PC.`
      : `Pour jouer à ${pour}, Frogtend installe ${choix.nom}, l’émulateur recommandé${o.taille ? ` (${o.taille})` : ''}, depuis son site officiel.`,
  ];
  if (choix.comment !== 'deja') lignes.push(`Dossier : ${o.dossier}\\${choix.nom}${o.dossierNouveau ? ' (le dossier des émulateurs, modifiable dans ⚙ Options ▸ Émulateurs)' : ''}`);
  if (choix.id && MANETTE_REGLEE.includes(choix.id)) lignes.push('Ta manette (Xbox ou autre) est réglée pour toi.');
  if (estConseille(choix.rec, o.conseil)) {
    const { fournis, aPoser, cles } = biosADire(o.conseil!.bios);
    if (fournis.length) {
      lignes.push(
        `Il lui faut aussi ${fournis.length > 1 ? 'des fichiers système' : 'un fichier système'} (${fournis.join(', ')}) : Firehouse ${fournis.length > 1 ? 'les fournit' : 'le fournit'}, Frogtend ${fournis.length > 1 ? 'les pose' : 'le pose'} pour toi (vérifié${o.tailleBios ? `, ${o.tailleBios}` : ''}).`,
      );
    }
    if (aPoser.length) lignes.push(`Il lui faut aussi : ${aPoser.join(', ')} — Firehouse ne les a pas ; Frogtend te dira où les mettre.`);
    if (cles.length) lignes.push(`Il te faudra aussi tes clés (${cles.join(', ')}) : personne ne peut les fournir, et sans elles les jeux ne démarrent pas.`);
  } else if (choix.rec.bios) {
    lignes.push(`Il lui faut aussi un fichier système (${choix.rec.bios}) : Firehouse le fournira ; en attendant, Frogtend te dira où le mettre.`);
  }
  if (choix.id && PREMIERE_PARTIE_LONGUE.includes(choix.id)) {
    lignes.push('La première partie de chaque jeu demande quelques minutes de préparation, une seule fois ; ensuite, il démarre vite.');
  }
  return lignes.join('\n');
}
