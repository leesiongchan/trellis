export const normalizeQuestion = (value: string): string => value.trim().normalize('NFC');
export const questionKey = (value: string): string =>
  normalizeQuestion(value)
    .toLocaleLowerCase()
    .replace(/\s+/g, ' ')
    .replace(/[?？]+$/u, '');
export function newId(): string {
  return Array.from(crypto.getRandomValues(new Uint8Array(16)), (n) =>
    n.toString(16).padStart(2, '0')
  ).join('');
}
export function isPublicSourceUrl(value: string): boolean {
  try {
    const u = new URL(value),
      h = u.hostname.toLowerCase().replace(/^\[|\]$/g, '');
    if (u.protocol !== 'https:' || u.username || u.password || (u.port && u.port !== '443'))
      return false;
    if (
      h === 'localhost' ||
      h.endsWith('.localhost') ||
      h.endsWith('.local') ||
      h.endsWith('.internal') ||
      !h.includes('.')
    )
      return false;
    if (h.includes(':')) return false;
    if (/^\d+\.\d+\.\d+\.\d+$/.test(h)) {
      const [a, b] = h.split('.').map(Number);
      if (
        a === 0 ||
        a === 10 ||
        a === 127 ||
        a >= 224 ||
        (a === 169 && b === 254) ||
        (a === 172 && b >= 16 && b <= 31) ||
        (a === 192 && (b === 168 || b === 0)) ||
        (a === 100 && b >= 64 && b <= 127)
      )
        return false;
    }
    return true;
  } catch {
    return false;
  }
}
