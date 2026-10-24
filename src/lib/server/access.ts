import { createHash, createHmac, timingSafeEqual } from 'node:crypto';
import { readConfig } from './config';
export const ACCESS_COOKIE = 'branch_access';
const digest = (value: string) => createHash('sha256').update(value).digest();
export function validAccessKey(candidate: string, expected: string): boolean {
  return expected.length >= 32 && timingSafeEqual(digest(candidate), digest(expected));
}
const signature = (expires: string, key: string) =>
  createHmac('sha256', key)
    .update('branch-session-v1:' + expires)
    .digest('base64url');
export function createSessionToken(key: string, expires = Date.now() + 7 * 86400000): string {
  const time = String(expires);
  return time + '.' + signature(time, key);
}
export function authorizeRequest(
  _request: Request,
  cookie: string | undefined,
  key = readConfig().accessKey
): boolean {
  if (!cookie || key.length < 32) return false;
  const [expires, sig, ...extra] = cookie.split('.');
  return (
    !extra.length &&
    Number.isFinite(Number(expires)) &&
    Number(expires) > Date.now() &&
    typeof sig === 'string' &&
    timingSafeEqual(digest(sig), digest(signature(expires, key)))
  );
}
