import { describe, expect, it } from 'vite-plus/test';
import { SourceSchema } from '../../src/lib/knowledge/schemas';
import { normalizeQuestion } from '../../src/lib/knowledge/normalize';

describe('knowledge contracts', () => {
  it('preservesUnicodeQuestions', () => {
    expect(normalizeQuestion('  为什么能存储数据？  ')).toBe('为什么能存储数据？');
  });
  it('rejectsUnsafeSourceUrls', () => {
    const source = {
      id: 's1',
      title: 'Reference',
      publisher: null,
      retrievedAt: new Date().toISOString()
    };
    for (const url of [
      'javascript:alert(1)',
      'data:text/html,bad',
      'http://localhost/a',
      'http://127.0.0.1',
      'https://10.1.2.3',
      'https://[::1]',
      'https://user:pass@example.org',
      'https://192.168.1.2'
    ])
      expect(SourceSchema.safeParse({ ...source, url }).success, url).toBe(false);
    expect(
      SourceSchema.safeParse({ ...source, url: 'https://example.org/reference' }).success
    ).toBe(true);
  });
});
