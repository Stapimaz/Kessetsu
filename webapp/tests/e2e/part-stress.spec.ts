import { expect, test } from '@playwright/test';
import { encodeShareFragment } from '../../src/share';

test('shows provided part limits separately from engineering requirements', async ({ page }) => {
  test.setTimeout(90_000);
  const source = `net GND
net OUT
source V1 10V
resistor R1 100Ohm
part R1 manufacturer="Example" mpn="R-100" peak_voltage_limit="12V" peak_voltage_conditions="Across p1-p2 at 25 C" peak_current_limit="90mA" peak_current_conditions="Continuous at 25 C" average_dissipation_limit="1.2W" average_dissipation_conditions="Free air at 25 C" rating_source="User-supplied example record"
connect V1.plus,R1.p1 to OUT
connect V1.minus,R1.p2 to GND
simulate tran 10us 1ms
`;
  const fragment = await encodeShareFragment(source, 'kessetsu.compile.v9', null, 'Part stress example');
  await page.goto(`/${fragment}`);
  await expect(page.getByTestId('compile-success')).toBeVisible({ timeout: 15_000 });
  await page.getByRole('button', { name: 'Run simulation', exact: true }).click();

  const summary = page.getByTestId('simulation-summary');
  await expect(summary).toHaveAttribute('data-state', 'succeeded', { timeout: 60_000 });
  const table = summary.getByRole('table', { name: 'Provided part limit comparison' });
  await expect(table).toBeVisible();
  await expect(table.locator('tbody tr')).toHaveCount(3);
  await expect(table.locator('.part-stress-within_provided_limit')).toHaveCount(2);
  await expect(table.locator('.part-stress-exceeds_provided_limit')).toHaveCount(1);
  await expect(table).toContainText('111.1% of limit');
  await expect(table).toContainText('User-supplied example record');
  await expect(summary.locator('.assertion-pass, .assertion-fail')).toHaveCount(0);
});
