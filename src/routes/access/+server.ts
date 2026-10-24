import type { RequestHandler } from '@sveltejs/kit';
import { ACCESS_COOKIE, createSessionToken, validAccessKey } from '#lib/server/access.ts';
import { readConfig } from '#lib/server/config.ts';
export const GET: RequestHandler = ({ url, cookies }) => {
  const config = readConfig();
  if (!validAccessKey(url.searchParams.get('key') ?? '', config.accessKey))
    return new Response('This private access link is invalid.', {
      status: 401,
      headers: { 'cache-control': 'no-store', 'referrer-policy': 'no-referrer' }
    });
  const secure = new URL(config.origin).protocol === 'https:';
  if (!secure && !['127.0.0.1', 'localhost', '[::1]'].includes(new URL(config.origin).hostname))
    return new Response('Use the private HTTPS address for access.', { status: 400 });
  cookies.set(ACCESS_COOKIE, createSessionToken(config.accessKey), {
    path: '/',
    httpOnly: true,
    secure,
    sameSite: 'strict',
    maxAge: 604800
  });
  return new Response(null, {
    status: 303,
    headers: { location: '/', 'cache-control': 'no-store', 'referrer-policy': 'no-referrer' }
  });
};
