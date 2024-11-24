import { test } from '@playwright/test';

for (let name of ['web']) {
  test(name, async ({ page }) => {
    /** @type {Promise<void>} */
    let functionExposed;
    const donePromise = new Promise((resolve, reject) => {
      functionExposed = page.exposeFunction('onDone', resolve);
      page.on('pageerror', reject);
    });
    await functionExposed;
    await page.goto(`/${name}/index.html`);
    await donePromise;
  });
}
