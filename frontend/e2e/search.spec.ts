import { test, expect } from "@playwright/test";

const FIXTURE_DIR =
  process.env.KMATRIX_FIXTURE_DIR ??
  "/Users/SP8PTW8/Code_Development/K-matrix-search-tool/K-Matrizen/MLBevo 2/K-Matrix";

test.describe("K-Matrix Toolkit E2E", () => {
  test("homepage loads with import and search sections", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText("K-Matrix Toolkit")).toBeVisible();
    await expect(page.getByTestId("import-section")).toBeVisible();
    await expect(page.getByTestId("search-section")).toBeVisible();
  });

  test("import directory and search for signals", async ({ page }) => {
    await page.goto("/");

    // Import fixture directory
    await page.getByTestId("import-path-input").fill(FIXTURE_DIR);
    await page.getByTestId("import-button").click();

    // Wait for import to complete
    await expect(page.getByTestId("import-status")).toBeVisible({
      timeout: 30_000,
    });

    // Search for a known signal from the DBC fixture
    await page.getByTestId("search-input").fill("CIMU_01_CRC");

    // Wait for results
    await expect(page.getByTestId("results-table")).toBeVisible({
      timeout: 5_000,
    });

    // Verify the expected signal appears
    const table = page.getByTestId("results-table");
    await expect(table.getByText("CIMU_01_CRC")).toBeVisible();
    await expect(table.getByText("CIMU_01")).toBeVisible();
  });

  test("search returns no results for gibberish", async ({ page }) => {
    await page.goto("/");

    // Import first
    await page.getByTestId("import-path-input").fill(FIXTURE_DIR);
    await page.getByTestId("import-button").click();
    await expect(page.getByTestId("import-status")).toBeVisible({
      timeout: 30_000,
    });

    // Search for nonsense
    await page.getByTestId("search-input").fill("NONEXISTENT_SIGNAL_XYZ");

    // Should show 0 results
    await expect(page.getByTestId("result-count")).toContainText("0 results", {
      timeout: 5_000,
    });
  });

  test("search is case-insensitive", async ({ page }) => {
    await page.goto("/");

    await page.getByTestId("import-path-input").fill(FIXTURE_DIR);
    await page.getByTestId("import-button").click();
    await expect(page.getByTestId("import-status")).toBeVisible({
      timeout: 30_000,
    });

    // Search lowercase
    await page.getByTestId("search-input").fill("cimu");

    await expect(page.getByTestId("results-table")).toBeVisible({
      timeout: 5_000,
    });

    // Should find CIMU signals
    const table = page.getByTestId("results-table");
    await expect(table.getByText("CIMU_01_CRC")).toBeVisible();
  });
});
