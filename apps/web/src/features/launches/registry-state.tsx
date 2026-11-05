"use client";

import type { UseQueryResult } from "@tanstack/react-query";
import { Rocket } from "lucide-react";
import Link from "next/link";
import type { ReactNode } from "react";

import { Button } from "@/components/ui/button";
import { EmptyState, ErrorState, Notice, Skeleton } from "@/components/ui/status";
import { network } from "@/config/network";
import type { Launch } from "@/lib/orbit/schemas";

export function FactoryMissingNotice() {
  return (
    <Notice tone="caution">
      No launch factory is configured for this deployment (
      <span className="tabular">NEXT_PUBLIC_ORBIT_FACTORY</span>
      ). Orbit reads launches only from the on-chain registry, so there is nothing to show until a factory
      address is set.
    </Notice>
  );
}

export function TableSkeleton({ rows = 5 }: { rows?: number }) {
  return (
    <div className="divide-y divide-line" aria-label="Loading" role="status">
      {Array.from({ length: rows }, (_, i) => (
        <div key={i} className="flex items-center gap-3 px-4 py-2.5">
          <Skeleton className="size-8" />
          <div className="flex-1 space-y-1.5">
            <Skeleton className="h-3 w-40" />
            <Skeleton className="h-2.5 w-16" />
          </div>
          <Skeleton className="h-3 w-12" />
        </div>
      ))}
    </div>
  );
}

/** Renders the loading, error, missing-factory and empty states around a list of launches. */
export function RegistryState({
  query,
  empty,
  children,
}: {
  query: UseQueryResult<Launch[]>;
  empty?: ReactNode;
  children: (launches: Launch[]) => ReactNode;
}) {
  if (!network.factoryId) return <FactoryMissingNotice />;
  if (query.isPending) return <TableSkeleton />;
  if (query.isError) {
    return (
      <ErrorState
        title="Could not load launches from the registry"
        error={query.error}
        onRetry={() => query.refetch()}
      />
    );
  }
  if (query.data.length === 0) {
    return (
      empty ?? (
        <EmptyState
          icon={Rocket}
          title="No launches yet"
          description="Launches appear here as soon as they are created on testnet."
          action={
            <Button asChild variant="primary" size="sm">
              <Link href="/create">Create the first launch</Link>
            </Button>
          }
        />
      )
    );
  }
  return <>{children(query.data)}</>;
}
