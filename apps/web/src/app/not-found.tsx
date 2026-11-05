import Link from "next/link";

import { Button } from "@/components/ui/button";

export default function NotFound() {
  return (
    <div className="mx-auto flex max-w-md flex-col items-center gap-4 px-4 py-24 text-center">
      <p className="tabular text-xs text-ink-faint">404</p>
      <h1 className="text-xl font-semibold text-ink">This page does not exist</h1>
      <p className="text-sm text-ink-muted">
        Launch pages are addressed by their registry number, for example{" "}
        <span className="tabular">/launches/0</span>.
      </p>
      <Button asChild variant="primary">
        <Link href="/explore">Explore launches</Link>
      </Button>
    </div>
  );
}
