import type { Metadata } from "next";

import { PortfolioPage } from "@/features/portfolio/portfolio-page";

export const metadata: Metadata = { title: "My assets" };

export default function Page() {
  return <PortfolioPage />;
}
