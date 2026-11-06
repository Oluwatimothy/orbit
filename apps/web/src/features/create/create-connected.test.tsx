import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";

import { renderWithWallet, walletState } from "@/test/render";
import { CreateConnectedPage } from "./create-connected-page";

vi.mock("next/navigation", () => ({ useRouter: () => ({ push: vi.fn() }), usePathname: () => "/" }));

it("explains a holder-rule problem introduced after that step, and the missing factory", async () => {
  const user = userEvent.setup();
  renderWithWallet(<CreateConnectedPage />, walletState({ status: "connected", accountId: "me.testnet" }));
  await user.type(screen.getByLabelText("Collection name"), "Test Club");
  await user.type(screen.getByLabelText("Symbol"), "TC");
  await user.type(screen.getByLabelText("Project description"), "hello");
  await user.type(
    screen.getByLabelText("Cover artwork"),
    "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
  );
  await user.click(screen.getByRole("button", { name: "Continue" }));
  await user.click(screen.getByRole("button", { name: "Continue" }));
  // Back to the collection tab, shrink the supply, then jump straight to the token tab.
  await user.click(screen.getByRole("button", { name: /Collection/ }));
  const max = screen.getByLabelText("Maximum supply");
  await user.clear(max);
  await user.type(max, "10");
  await user.click(screen.getByRole("button", { name: /Token/ }));
  await user.type(screen.getByLabelText("Token name"), "Test Coin");
  await user.type(screen.getByLabelText("Symbol"), "TCN");
  await user.type(screen.getByLabelText("Allocation 1 recipient"), "me.testnet");
  await user.type(screen.getByLabelText("Allocation 1 amount"), "8000000");
  await user.click(screen.getByRole("button", { name: "Review launch" }));
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Holder rules: Holders: IDs must be between 1 and 10",
  );
  expect(screen.queryByRole("dialog")).toBeNull();
});

it("says how much is unallocated when the allocation does not add up", async () => {
  const user = userEvent.setup();
  renderWithWallet(<CreateConnectedPage />, walletState({ status: "connected", accountId: "me.testnet" }));
  await user.type(screen.getByLabelText("Collection name"), "Test Club");
  await user.type(screen.getByLabelText("Symbol"), "TC");
  await user.type(screen.getByLabelText("Project description"), "hello");
  await user.type(
    screen.getByLabelText("Cover artwork"),
    "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
  );
  await user.click(screen.getByRole("button", { name: "Continue" }));
  await user.click(screen.getByRole("button", { name: "Continue" }));
  await user.type(screen.getByLabelText("Token name"), "Kay");
  await user.type(screen.getByLabelText("Symbol"), "KAY");
  await user.type(screen.getByLabelText("Allocation 1 recipient"), "me.testnet");
  await user.type(screen.getByLabelText("Allocation 1 amount"), "1000000");
  await user.click(screen.getByRole("button", { name: "Review launch" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("7,000,000 KAY is unallocated");
  expect(screen.queryByRole("dialog")).toBeNull();
});
