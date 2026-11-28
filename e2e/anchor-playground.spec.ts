import { test, expect, type Page } from "@playwright/test";

/**
 * End-to-end tests for the AnchorPlayground component, run against the
 * Storybook build. Covers tab navigation, form submission, theme switching
 * and the request panel.
 */

const STORY_ID = "anchor-anchorplayground--default";

async function openPlayground(page: Page) {
  await page.goto(`/iframe.html?id=${STORY_ID}&viewMode=story`);
  await expect(page.getByTestId("anchor-playground")).toBeVisible();
}

test.describe("AnchorPlayground", () => {
  test.beforeEach(async ({ page }) => {
    await openPlayground(page);
  });

  test("navigates between tabs", async ({ page }) => {
    const tabs = page.getByRole("tab");
    await expect(tabs.first()).toBeVisible();

    const count = await tabs.count();
    for (let i = 0; i < count; i++) {
      const tab = tabs.nth(i);
      await tab.click();
      await expect(tab).toHaveAttribute("aria-selected", "true");
      await expect(page.getByRole("tabpanel")).toBeVisible();
    }
  });

  test("submits the form", async ({ page }) => {
    const form = page.getByTestId("anchor-playground-form");
    await expect(form).toBeVisible();

    const submit = form.getByRole("button", { name: /submit|run|send/i });
    await submit.click();

    await expect(page.getByTestId("anchor-playground-result")).toBeVisible();
  });

  test("switches theme", async ({ page }) => {
    const toggle = page.getByRole("button", { name: /theme|dark|light/i });
    await expect(toggle).toBeVisible();

    const root = page.getByTestId("anchor-playground");
    const before = await root.getAttribute("data-theme");
    await toggle.click();
    await expect(root).not.toHaveAttribute("data-theme", before ?? "");
  });

  test("shows the request panel", async ({ page }) => {
    const panel = page.getByTestId("anchor-playground-request-panel");
    await expect(panel).toBeVisible();
    await expect(panel).toContainText(/request/i);
  });
});
