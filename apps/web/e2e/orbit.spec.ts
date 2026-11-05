import { expect, test } from "@playwright/test";

import { mockChain } from "./chain-mock";

test.beforeEach(async ({ page }) => {
  await mockChain(page);
});

test("overview shows the registry and the testnet indicator", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toContainText("Launch tokens, NFT collections");
  await expect(page.getByText("Testnet", { exact: true })).toBeVisible();
  await expect(page.getByRole("link", { name: /Orbit Genesis/ }).first()).toBeVisible();
});

test("explore filters and searches launches", async ({ page }) => {
  await page.goto("/explore");
  await expect(page.getByRole("link", { name: /Orbit Genesis/ })).toBeVisible();
  await page.getByRole("searchbox", { name: "Search launches" }).fill("does-not-exist");
  await expect(page.getByText("No launches match")).toBeVisible();
  await page.getByRole("searchbox", { name: "Search launches" }).fill("genesis");
  await page.getByRole("radio", { name: "Token" }).click();
  await expect(page.getByText("No launches match")).toBeVisible();
  await page.getByRole("radio", { name: "Connected" }).click();
  await expect(page.getByRole("link", { name: /Orbit Genesis/ })).toBeVisible();
  // Mint state is read from the collection contract.
  await page.getByRole("radio", { name: "Not minting" }).click();
  await expect(page.getByText("No launches match")).toBeVisible();
  await page.getByRole("radio", { name: "Minting now" }).click();
  await expect(page.getByRole("link", { name: /Orbit Genesis/ })).toBeVisible();
});

test("connected launch page shows the relationship, tiers, supply and mint rules", async ({ page }) => {
  await page.goto("/launches/0");
  await expect(page.getByRole("heading", { level: 1 })).toContainText("Orbit Genesis");
  await expect(page.getByRole("heading", { name: "Holder allocation" })).toBeVisible();
  await expect(page.getByText("Claims open")).toBeVisible();
  await expect(page.getByRole("table", { name: "Allocation tiers" })).toContainText("#1–#2000");
  await expect(page.getByRole("table", { name: "Initial token allocation" })).toContainText(
    "NFT holder claims",
  );
  await expect(page.getByText("Connect a wallet to see your eligibility")).toBeVisible();
  await expect(page.getByRole("button", { name: "Connect wallet to mint" })).toBeVisible();
  await expect(page.getByText("genesis.e2e-factory.testnet").first()).toBeVisible();
});

test("unknown launch IDs show an error instead of fake data", async ({ page }) => {
  await page.goto("/launches/abc");
  await expect(page.getByRole("heading", { name: "This page does not exist" })).toBeVisible();
});

test("token form validates input and enforces exact allocation", async ({ page }) => {
  await page.goto("/create/token");
  await page.getByLabel("Token name").fill("Test Token");
  await page.getByLabel("Symbol").fill("TT");
  await page.getByLabel("Total supply").fill("1000");
  await page.getByRole("textbox", { name: "Allocation 1 recipient" }).fill("Not An Account");
  await page.getByRole("textbox", { name: "Allocation 1 amount" }).fill("40");
  await expect(page.getByText(/600 TT unallocated/)).toBeVisible();
  await page.getByRole("textbox", { name: "Allocation 1 amount" }).fill("100");
  await expect(page.getByText("Fully allocated")).toBeVisible();
  // The slug follows the name until edited.
  await expect(page.getByLabel("Launch slug")).toHaveValue("test-token");
});

test("connected wizard requires a holder rule before continuing", async ({ page }) => {
  await page.goto("/create/connected");
  await expect(page.getByRole("button", { name: /Collection/ })).toHaveAttribute("aria-current", "step");
  await page.getByLabel("Collection name").fill("Genesis Pass");
  await page.getByLabel("Symbol").fill("GEN");
  await page.getByLabel("Cover artwork").fill("ipfs://bafycover");
  await page.getByLabel("Project description").fill("A test launch");
  await page.getByRole("button", { name: "Continue" }).click();
  await expect(page.getByRole("button", { name: /Holder rules/ })).toHaveAttribute("aria-current", "step");
  await page.getByLabel("Enable holder claims").uncheck();
  await expect(page.getByText("Choose holder claims, a holder sale, or both")).toBeVisible();
  await expect(page.getByRole("button", { name: "Continue" })).toBeDisabled();
  await page.getByLabel("Enable NFT-gated sale").check();
  await expect(page.getByText("Set when the sale ends")).toBeVisible();
});

test("portfolio and activity ask for a wallet instead of showing balances", async ({ page }) => {
  await page.goto("/portfolio");
  await expect(page.getByText("Connect a wallet to see your assets")).toBeVisible();
  await page.goto("/activity");
  await expect(page.getByText("Connect a wallet to see your activity")).toBeVisible();
});

test("navigation works on every screen size", async ({ page, isMobile }) => {
  await page.goto("/");
  const nav = page.getByRole("navigation", { name: "Main" }).filter({ visible: true });
  await nav.getByRole("link", { name: "Explore" }).click();
  await expect(page).toHaveURL(/\/explore$/);
  await nav.getByRole("link", { name: "Create" }).click();
  await expect(page.getByRole("heading", { name: "Create a launch" })).toBeVisible();
  if (isMobile) {
    await page.getByRole("button", { name: "More sections" }).click();
    await page.getByRole("menuitem", { name: "How it works" }).click();
  } else {
    await page.getByRole("link", { name: "How it works" }).click();
  }
  await expect(page.getByRole("heading", { name: "How Orbit works" })).toBeVisible();
});

test("pages do not scroll horizontally", async ({ page }) => {
  for (const path of ["/", "/explore", "/create", "/create/connected", "/launches/0", "/docs"]) {
    await page.goto(path);
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(overflow, path).toBeLessThanOrEqual(0);
  }
});
