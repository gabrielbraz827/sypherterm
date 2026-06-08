import { describe, expect, it } from 'vitest';

import { sanitizeNotificationBody } from './native';

describe('native integration wrappers', () => {
  it('redacts obvious secrets from notification bodies', () => {
    const body = sanitizeNotificationBody(
      'password=hunter2 token=abcd privateKey=/home/me/.ssh/id_ed25519',
    );

    expect(body).toContain('password=[redacted]');
    expect(body).toContain('token=[redacted]');
    expect(body).toContain('privateKey=[redacted]');
    expect(body).not.toContain('hunter2');
    expect(body).not.toContain('id_ed25519');
  });

  it('redacts long hashes from notification bodies', () => {
    const body = sanitizeNotificationBody(
      'version 87c3286d4efaaad9e81bfc4327358423e078ba1b23a595c0d9937823c2670299',
    );

    expect(body).toBe('version [hash]');
  });

  it('keeps notification bodies compact', () => {
    const body = sanitizeNotificationBody('x'.repeat(200));

    expect(body).toHaveLength(120);
  });
});
