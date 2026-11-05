import type { Metadata } from "next";

import { MyLaunchesPage } from "@/features/portfolio/portfolio-page";

export const metadata: Metadata = { title: "My launches" };

export default function Page() {
  return <MyLaunchesPage />;
}
