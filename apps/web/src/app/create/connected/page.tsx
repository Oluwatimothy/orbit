import type { Metadata } from "next";

import { CreateConnectedPage } from "@/features/create/create-connected-page";

export const metadata: Metadata = { title: "Launch a connected token + NFT" };

export default function Page() {
  return <CreateConnectedPage />;
}
