// Un Markdown SIMPLE et SÛR (réponses de l'assistant, textes des annexes : « Markdown simple », contrat) : tout est
// d'abord échappé, puis seuls titres, gras, italique, listes, code et liens http(s) sont mis en forme.

const echapper = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

function enLigne(s: string): string {
  return s
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/(^|[\s(])\*([^*\s][^*]*)\*/g, '$1<em>$2</em>')
    .replace(/\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g, '$1 <span class="lien">($2)</span>');
}

/** Rend un texte Markdown simple en HTML sûr. */
export function markdownSimple(texte: string): string {
  const lignes = echapper(texte ?? '').split(/\r?\n/);
  const sortie: string[] = [];
  let liste: 'ul' | 'ol' | null = null;
  const fermer = () => {
    if (liste) sortie.push(`</${liste}>`);
    liste = null;
  };
  for (const l of lignes) {
    const titre = /^(#{1,4})\s+(.*)$/.exec(l);
    const puce = /^\s*[-*]\s+(.*)$/.exec(l);
    const num = /^\s*\d+[.)]\s+(.*)$/.exec(l);
    if (titre) {
      fermer();
      const n = Math.min(titre[1].length + 1, 5);
      sortie.push(`<h${n}>${enLigne(titre[2])}</h${n}>`);
    } else if (puce || num) {
      const voulu = puce ? 'ul' : 'ol';
      if (liste !== voulu) {
        fermer();
        sortie.push(`<${voulu}>`);
        liste = voulu;
      }
      sortie.push(`<li>${enLigne((puce ?? num)![1])}</li>`);
    } else if (l.trim() === '') {
      fermer();
    } else {
      fermer();
      sortie.push(`<p>${enLigne(l)}</p>`);
    }
  }
  fermer();
  return sortie.join('');
}
