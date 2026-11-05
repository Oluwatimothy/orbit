import { notFound } from "next/navigation";

import { LaunchPage } from "@/features/launch/launch-page";

export default async function Page({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  if (!/^\d+$/.test(id)) notFound();
  return <LaunchPage id={Number(id)} />;
}
