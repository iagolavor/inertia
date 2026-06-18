/** Deterministic GitHub-style block identicon from a seed string (e.g. signing pubkey). */

function hashSeed(seed: string): number[] {
  const bytes: number[] = [];
  let h = 5381;
  for (let i = 0; i < seed.length; i++) {
    h = (h * 33) ^ seed.charCodeAt(i);
  }
  for (let i = 0; i < 16; i++) {
    h = (h * 33) ^ i;
    bytes.push(h & 0xff);
  }
  return bytes;
}

export function identiconSvg(seed: string, size = 64): string {
  const hash = hashSeed(seed);
  const hue = hash[0] % 360;
  const bg = `hsl(${hue}, 42%, 58%)`;
  const fg = `hsl(${hue}, 52%, 32%)`;

  const cell = size / 5;
  let rects = '';
  let bit = 0;

  for (let y = 0; y < 5; y++) {
    for (let x = 0; x < 3; x++) {
      const filled = (hash[1 + Math.floor(bit / 8)] >> (bit % 8)) & 1;
      bit++;
      if (!filled) continue;
      for (const col of [x, 4 - x]) {
        rects += `<rect x="${col * cell}" y="${y * cell}" width="${cell}" height="${cell}" fill="${fg}"/>`;
      }
    }
  }

  return `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 ${size} ${size}" role="img"><rect width="${size}" height="${size}" fill="${bg}" rx="${size * 0.12}"/>${rects}</svg>`;
}

export function identiconDataUrl(seed: string, size = 64): string {
  return `data:image/svg+xml,${encodeURIComponent(identiconSvg(seed, size))}`;
}
