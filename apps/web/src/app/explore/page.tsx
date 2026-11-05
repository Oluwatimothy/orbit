import type { Metadata } from "next";

import { ExplorePage } from "@/features/explore/explore-page";

export const metadata: Metadata = { title: "Explore" };

export default function Page() {
  return <ExplorePage />;
}
