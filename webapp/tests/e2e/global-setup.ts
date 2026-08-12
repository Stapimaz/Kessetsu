import type { FullConfig } from '@playwright/test';
import { preview } from 'vite';

export default async function globalSetup(_config: FullConfig) {
  const server = await preview({
    preview: {
      host: '127.0.0.1',
      port: 4173,
      strictPort: true,
    },
  });

  return async () => {
    await new Promise<void>((resolve, reject) => {
      server.httpServer.close((error) => {
        if (error) reject(error);
        else resolve();
      });
    });
  };
}
