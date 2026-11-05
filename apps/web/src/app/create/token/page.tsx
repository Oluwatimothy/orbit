import type { Metadata } from "next";

import { CreateTokenPage } from "@/features/create/create-token-page";

export const metadata: Metadata = { title: "Launch a token" };

export default function Page() {
  return <CreateTokenPage />;
}
