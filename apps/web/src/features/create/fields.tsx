"use client";

import { useQuery } from "@tanstack/react-query";
import { CheckCircle2, ImageUp, LoaderCircle, XCircle } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { Button } from "@/components/ui/button";
import { Field, Input } from "@/components/ui/field";
import { network } from "@/config/network";
import { uploadImage, uploadsEnabled } from "@/features/media/upload";
import { LaunchImage } from "@/features/launches/launch-display";
import { factory } from "@/lib/orbit/contracts";
import { isContentAddressed } from "@/lib/orbit/media";
import { UPLOAD_MIME_TYPES } from "@/lib/upload-policy";
import { slugSchema } from "@/lib/validation/launch";

function useDebounced<T>(value: T, ms: number): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), ms);
    return () => clearTimeout(timer);
  }, [value, ms]);
  return debounced;
}

/** Slug input that checks availability in the factory registry as you type. */
export function SlugField({
  value,
  onChange,
  error,
  suffixes,
}: {
  value: string;
  onChange: (value: string) => void;
  error?: string;
  suffixes: string[];
}) {
  const debounced = useDebounced(value, 400);
  const valid = slugSchema.safeParse(debounced).success;
  const availability = useQuery({
    queryKey: ["slug", debounced],
    queryFn: () => factory.isSlugAvailable(debounced),
    enabled: valid && Boolean(network.factoryId),
    staleTime: 5_000,
  });
  const status =
    !valid || !network.factoryId ? null : availability.isPending ? (
      <LoaderCircle className="size-3.5 animate-spin text-ink-faint" aria-label="Checking" />
    ) : availability.data ? (
      <span className="flex items-center gap-1 text-2xs text-positive">
        <CheckCircle2 className="size-3.5" aria-hidden /> Available
      </span>
    ) : availability.isError ? (
      <span className="text-2xs text-caution">Could not check</span>
    ) : (
      <span className="flex items-center gap-1 text-2xs text-negative">
        <XCircle className="size-3.5" aria-hidden /> Taken
      </span>
    );
  const accounts = suffixes.map((s) => `${value || "slug"}-${s}.${network.factoryId ?? "factory.testnet"}`);

  return (
    <Field
      label="Launch slug"
      error={error ?? (availability.data === false ? "This slug is already registered" : undefined)}
      aside={status}
      hint={
        <>
          Your contracts will live at{" "}
          {accounts.map((a, i) => (
            <span key={a}>
              <span className="tabular text-ink-muted">{a}</span>
              {i < accounts.length - 1 ? ", " : ""}
            </span>
          ))}
          . A slug can be used once.
        </>
      }
    >
      {(props) => (
        <Input
          {...props}
          value={value}
          onChange={(e) => onChange(e.target.value.toLowerCase())}
          placeholder="orbit-genesis"
          autoComplete="off"
          spellCheck={false}
        />
      )}
    </Field>
  );
}

/** URL input for ipfs:// or https:// media, with optional upload to IPFS. */
export function MediaField({
  label,
  value,
  onChange,
  onHash,
  error,
  hint,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  onHash?: (sha256Base64: string) => void;
  error?: string;
  hint?: string;
}) {
  const enabled = useQuery({ queryKey: ["uploads-enabled"], queryFn: uploadsEnabled, staleTime: Infinity });
  const fileInput = useRef<HTMLInputElement>(null);
  const [uploading, setUploading] = useState(false);
  const [uploadError, setUploadError] = useState<string | null>(null);

  async function handleFile(file: File | undefined) {
    if (!file) return;
    setUploading(true);
    setUploadError(null);
    try {
      const { reference, sha256 } = await uploadImage(file);
      onChange(reference);
      onHash?.(sha256);
    } catch (e) {
      setUploadError(e instanceof Error ? e.message : "Upload failed");
    } finally {
      setUploading(false);
    }
  }

  const warning =
    value && !error && !isContentAddressed(value)
      ? "This is not content-addressed: its host can change or remove it."
      : undefined;
  return (
    <Field
      label={label}
      error={error ?? uploadError ?? undefined}
      hint={warning ?? hint ?? "ipfs://… or https://…"}
    >
      {(props) => (
        <div className="flex items-start gap-3">
          <LaunchImage src={value} alt="" fallback="IMG" className="size-16" />
          <div className="flex min-w-0 flex-1 flex-col gap-2">
            <Input
              {...props}
              value={value}
              onChange={(e) => {
                onChange(e.target.value.trim());
                onHash?.("");
              }}
              placeholder="ipfs://bafy…"
              spellCheck={false}
            />
            {enabled.data && (
              <div>
                <input
                  ref={fileInput}
                  type="file"
                  accept={UPLOAD_MIME_TYPES.join(",")}
                  className="sr-only"
                  tabIndex={-1}
                  onChange={(e) => void handleFile(e.target.files?.[0])}
                />
                <Button
                  type="button"
                  size="sm"
                  variant="secondary"
                  loading={uploading}
                  onClick={() => fileInput.current?.click()}
                >
                  <ImageUp className="size-3.5" aria-hidden /> Upload to IPFS
                </Button>
              </div>
            )}
          </div>
        </div>
      )}
    </Field>
  );
}
