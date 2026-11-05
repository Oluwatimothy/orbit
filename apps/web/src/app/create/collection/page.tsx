import type { Metadata } from "next";

import { CreateCollectionPage } from "@/features/create/create-collection-page";

export const metadata: Metadata = { title: "Launch an NFT collection" };

export default function Page() {
  return <CreateCollectionPage />;
}
