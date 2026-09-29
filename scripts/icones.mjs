// Fabrique les images du logo à partir de docs/logo/logo-origine.png (grenouille + nom, fond transparent) :
//   - docs/logo/logo.png         : le logo complet, recadré (README) ;
//   - docs/logo/grenouille.png   : la grenouille seule, carrée, fond transparent (source des icônes) ;
//   - static/grenouille.png      : la même, pour l'interface.
// Puis : pnpm tauri icon docs/logo/grenouille.png -o src-tauri/icons  (icônes de l'application et de
// l'installateur), supprimer les dossiers android/ et ios/ produits, et copier 128x128.png en static/favicon.png.
import { copyFileSync, readFileSync, writeFileSync } from 'node:fs';
import pngjs from 'pngjs';

const { PNG } = pngjs;
const source = PNG.sync.read(readFileSync('docs/logo/logo-origine.png'));
const { width: L, height: H } = source;

/** Un pixel occupé : assez opaque (le logo d'origine a déjà un fond transparent). */
const SEUIL_ALPHA = 40;

// Les lignes qui contiennent quelque chose, pour séparer la grenouille du nom.
const ligneOccupee = (y) => {
  for (let x = 0; x < L; x++) if (source.data[(y * L + x) * 4 + 3] > SEUIL_ALPHA) return true;
  return false;
};
const lignes = Array.from({ length: H }, (_, y) => ligneOccupee(y));
const blocs = [];
for (let y = 0; y < H; y++) {
  if (lignes[y] && (y === 0 || !lignes[y - 1])) blocs.push({ debut: y, fin: y });
  if (lignes[y]) blocs[blocs.length - 1].fin = y;
}
// On ignore les poussières (quelques lignes isolées).
const vrais = blocs.filter((b) => b.fin - b.debut > 20);
if (vrais.length < 2) {
  console.error(`Échec : ${vrais.length} bloc(s) trouvé(s) au lieu de 2 (grenouille, puis nom).`);
  process.exit(1);
}
const [grenouille, nom] = vrais;

/** Rectangle occupé entre deux lignes. */
function cadre(y0, y1) {
  let x0 = L;
  let x1 = 0;
  for (let y = y0; y <= y1; y++)
    for (let x = 0; x < L; x++)
      if (source.data[(y * L + x) * 4 + 3] > SEUIL_ALPHA) {
        x0 = Math.min(x0, x);
        x1 = Math.max(x1, x);
      }
  return { x0, y0, x1, y1 };
}

/** Copie un rectangle au centre d'une image transparente, avec une marge. */
function extraire({ x0, y0, x1, y1 }, marge, carre) {
  const l = x1 - x0 + 1;
  const h = y1 - y0 + 1;
  const W = (carre ? Math.max(l, h) : l) + 2 * marge;
  const Ht = (carre ? Math.max(l, h) : h) + 2 * marge;
  const img = new PNG({ width: W, height: Ht });
  img.data.fill(0);
  const dx = Math.floor((W - l) / 2);
  const dy = Math.floor((Ht - h) / 2);
  for (let y = 0; y < h; y++)
    for (let x = 0; x < l; x++) {
      const s = ((y0 + y) * L + x0 + x) * 4;
      const d = ((dy + y) * W + dx + x) * 4;
      for (let k = 0; k < 4; k++) img.data[d + k] = source.data[s + k];
    }
  return img;
}

const imgGrenouille = extraire(cadre(grenouille.debut, grenouille.fin), 16, true);
const imgLogo = extraire(cadre(grenouille.debut, nom.fin), 24, false);
writeFileSync('docs/logo/grenouille.png', PNG.sync.write(imgGrenouille));
writeFileSync('docs/logo/logo.png', PNG.sync.write(imgLogo));
copyFileSync('docs/logo/grenouille.png', 'static/grenouille.png');

console.log(
  `Grenouille : lignes ${grenouille.debut}–${grenouille.fin} → ${imgGrenouille.width}×${imgGrenouille.height} ; ` +
    `nom : lignes ${nom.debut}–${nom.fin} ; logo complet ${imgLogo.width}×${imgLogo.height}.`,
);
