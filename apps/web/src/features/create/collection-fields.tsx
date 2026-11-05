"use client";

import type { FieldErrors, UseFormRegister, UseFormSetValue } from "react-hook-form";

import { Field, Input } from "@/components/ui/field";
import { cn } from "@/lib/cn";
import type { CollectionForm } from "@/lib/validation/launch";

import { MediaField } from "./fields";

function Segmented<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: Array<{ value: T; label: string }>;
  onChange: (value: T) => void;
}) {
  return (
    <div
      role="radiogroup"
      aria-label={label}
      className="inline-flex rounded-sm border border-line-strong p-0.5"
    >
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          role="radio"
          aria-checked={value === option.value}
          onClick={() => onChange(option.value)}
          className={cn(
            "h-7 rounded-xs px-3 text-xs transition-colors",
            value === option.value ? "bg-raised text-ink" : "text-ink-muted hover:text-ink",
          )}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

type Props = {
  values: CollectionForm;
  register: UseFormRegister<CollectionForm>;
  setValue: UseFormSetValue<CollectionForm>;
  errors: FieldErrors<CollectionForm>;
  /** Connected launches require a fixed supply so claim tiers can cover it. */
  requireFixedSupply?: boolean;
};

export function CollectionIdentityFields({
  values,
  register,
  setValue,
  errors,
  onNameChange,
}: Props & { onNameChange?: (name: string) => void }) {
  return (
    <>
      <div className="grid gap-4 sm:grid-cols-[1fr_160px]">
        <Field label="Collection name" error={errors.name?.message}>
          {(props) => (
            <Input
              {...props}
              {...register("name", { onChange: (e) => onNameChange?.(e.target.value) })}
              placeholder="Genesis Pass"
            />
          )}
        </Field>
        <Field label="Symbol" error={errors.symbol?.message}>
          {(props) => (
            <Input
              {...props}
              {...register("symbol", { setValueAs: (v: string) => v.toUpperCase() })}
              placeholder="GEN"
              className="tabular uppercase"
            />
          )}
        </Field>
      </div>
      <MediaField
        label="Cover artwork"
        value={values.cover}
        onChange={(v) => setValue("cover", v, { shouldValidate: true })}
        onHash={(hash) => setValue("coverHash", hash)}
        error={errors.cover?.message}
        hint="Shown on the launch page and, in shared mode, on every NFT."
      />
      <div className="space-y-2">
        <p className="text-xs font-medium text-ink-muted">Token artwork</p>
        <Segmented
          label="Token artwork"
          value={values.mediaMode}
          onChange={(v) => setValue("mediaMode", v, { shouldValidate: true })}
          options={[
            { value: "shared", label: "Same for every NFT" },
            { value: "directory", label: "One file per NFT" },
          ]}
        />
        {values.mediaMode === "directory" && (
          <div className="grid gap-4 sm:grid-cols-[1fr_120px]">
            <Field
              label="Folder URL"
              error={errors.baseUri?.message}
              hint="An https:// IPFS gateway folder. NFT #n uses {folder}/n.{extension}."
            >
              {(props) => (
                <Input
                  {...props}
                  {...register("baseUri")}
                  placeholder="https://gateway.pinata.cloud/ipfs/bafy…"
                  spellCheck={false}
                />
              )}
            </Field>
            <Field label="Extension">
              {(props) => (
                <select
                  {...props}
                  {...register("extension")}
                  className="h-8 rounded-sm border border-line-strong bg-surface px-2 text-sm"
                >
                  {["png", "jpg", "jpeg", "webp", "gif", "svg"].map((ext) => (
                    <option key={ext}>{ext}</option>
                  ))}
                </select>
              )}
            </Field>
          </div>
        )}
      </div>
    </>
  );
}

export function MintRuleFields({ values, register, setValue, errors, requireFixedSupply }: Props) {
  return (
    <>
      <div className="space-y-2">
        {!requireFixedSupply && <p className="text-xs font-medium text-ink-muted">Supply</p>}
        {!requireFixedSupply && (
          <Segmented
            label="Supply"
            value={values.supplyMode}
            onChange={(v) => setValue("supplyMode", v, { shouldValidate: true })}
            options={[
              { value: "fixed", label: "Fixed maximum" },
              { value: "unlimited", label: "Unlimited" },
            ]}
          />
        )}
        {values.supplyMode === "fixed" ? (
          <Field
            label="Maximum supply"
            error={errors.maxSupply?.message}
            hint="The contract refuses any mint beyond this number. 1–10,000."
          >
            {(props) => (
              <Input
                {...props}
                {...register("maxSupply")}
                inputMode="numeric"
                className="tabular sm:max-w-48"
              />
            )}
          </Field>
        ) : (
          <p className="text-xs text-caution">
            Unlimited: anyone can keep minting for as long as the mint window is open.
          </p>
        )}
      </div>
      <div className="grid gap-4 sm:grid-cols-2">
        <Field
          label="Mint price (NEAR)"
          error={errors.price?.message}
          hint="Paid to you on every mint. 0 for free."
        >
          {(props) => <Input {...props} {...register("price")} inputMode="decimal" className="tabular" />}
        </Field>
        <Field label="Max per wallet" error={errors.maxPerWallet?.message} hint="Leave empty for no limit.">
          {(props) => (
            <Input {...props} {...register("maxPerWallet")} inputMode="numeric" className="tabular" />
          )}
        </Field>
      </div>
      <div className="grid gap-4 sm:grid-cols-2">
        <Field
          label="Mint opens"
          error={errors.startsAt?.message}
          hint="Leave empty to open immediately. Your local time."
        >
          {(props) => <Input {...props} {...register("startsAt")} type="datetime-local" />}
        </Field>
        <Field label="Mint closes" error={errors.endsAt?.message} hint="Leave empty for no end.">
          {(props) => <Input {...props} {...register("endsAt")} type="datetime-local" />}
        </Field>
      </div>
      <label className="flex items-start gap-2.5 text-sm text-ink">
        <input
          type="checkbox"
          {...register("publicMint")}
          className="mt-0.5 size-4 accent-[var(--color-primary)]"
        />
        <span>
          Public mint
          <span className="block text-xs text-ink-muted">
            Off means only you can mint (to any account). Price, supply and window cannot be changed after
            launch.
          </span>
        </span>
      </label>
    </>
  );
}

export const COLLECTION_DEFAULTS: CollectionForm = {
  slug: "",
  name: "",
  symbol: "",
  description: "",
  cover: "",
  coverHash: "",
  mediaMode: "shared",
  baseUri: "",
  extension: "png",
  supplyMode: "fixed",
  maxSupply: "1000",
  price: "0.1",
  startsAt: "",
  endsAt: "",
  maxPerWallet: "",
  publicMint: true,
};
