"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { Check } from "lucide-react";
import { useState, type BaseSyntheticEvent } from "react";
import { useForm, useWatch, type UseFormRegister } from "react-hook-form";

import { Button } from "@/components/ui/button";
import { KeyValue } from "@/components/ui/data";
import { Field, Input, Textarea } from "@/components/ui/field";
import { Notice } from "@/components/ui/status";
import { useWallet } from "@/features/wallet/wallet-provider";
import { cn } from "@/lib/cn";
import { formatCompact, formatNear, formatShare, formatUnits } from "@/lib/format/units";
import {
  buildCollectionConfig,
  buildTokenConfig,
  collectionFormSchema,
  resolveAllocations,
  resolveSale,
  resolveTiers,
  slugify,
  tokenFormSchema,
  toNs,
  type CollectionForm,
  type SaleForm,
  type ContractAllocation,
  type TokenForm,
} from "@/lib/validation/launch";

import { AllocationEditor } from "./allocation-editor";
import { COLLECTION_DEFAULTS, CollectionIdentityFields, MintRuleFields } from "./collection-fields";
import { launchAccountId, MediaField, SlugField } from "./fields";
import { CreateLayout, FormSection, SummaryPanel } from "./layout";
import { ReviewDialog } from "./review-dialog";
import { TierEditor, type TiersForm } from "./tier-editor";
import { TokenIdentityFields } from "./token-fields";
import { useCreateLaunch } from "./use-create-launch";

const STEPS = ["Collection", "Holder rules", "Token"] as const;

const TOKEN_DEFAULTS: TokenForm = {
  slug: "",
  name: "",
  symbol: "",
  description: "",
  icon: "",
  decimals: 18,
  totalSupply: "10,000,000",
  allocations: [{ label: "Creator", accountId: "", mode: "amount", value: "" }],
};

function Steps({
  current,
  onSelect,
  done,
}: {
  current: number;
  onSelect: (step: number) => void;
  done: boolean[];
}) {
  return (
    <ol className="mb-6 grid grid-cols-3 border-b border-line">
      {STEPS.map((label, i) => (
        <li key={label}>
          <button
            type="button"
            onClick={() => onSelect(i)}
            disabled={i > current && !done.slice(0, i).every(Boolean)}
            aria-current={i === current ? "step" : undefined}
            className={cn(
              "-mb-px flex w-full items-center gap-2 border-b-2 py-2.5 text-left text-xs transition-colors disabled:cursor-not-allowed",
              i === current
                ? "border-primary text-ink"
                : "border-transparent text-ink-faint hover:text-ink-muted",
            )}
          >
            <span
              className={cn(
                "tabular flex size-5 items-center justify-center rounded-full border text-2xs",
                done[i] ? "border-positive text-positive" : "border-line-strong",
              )}
            >
              {done[i] ? <Check className="size-3" aria-hidden /> : i + 1}
            </span>
            <span className="truncate">{label}</span>
          </button>
        </li>
      ))}
    </ol>
  );
}

export function CreateConnectedPage() {
  const { accountId, connect } = useWallet();
  const [step, setStep] = useState(0);
  const [done, setDone] = useState([false, false, false]);
  const [slugEdited, setSlugEdited] = useState(false);
  const [reviewing, setReviewing] = useState(false);
  const { quote, submit, state, reset } = useCreateLaunch("connected");

  const collectionForm = useForm<CollectionForm>({
    resolver: zodResolver(collectionFormSchema),
    defaultValues: { ...COLLECTION_DEFAULTS, maxSupply: "2000" },
    mode: "onTouched",
  });
  const tokenForm = useForm<TokenForm>({
    resolver: zodResolver(tokenFormSchema),
    defaultValues: TOKEN_DEFAULTS,
    mode: "onTouched",
  });
  const tiersForm = useForm<TiersForm>({
    defaultValues: {
      claimsEnabled: true,
      tiers: [{ name: "Holders", from: "1", to: "2000", amount: "1000" }],
      claimStartsAt: "",
      saleEnabled: false,
      sale: {
        price: "0.01",
        total: "1,000,000",
        perNft: "1,000",
        holderStartsAt: "",
        publicEnabled: false,
        publicStartsAt: "",
        perAccount: "",
        endsAt: "",
      },
    },
  });

  const collection = useWatch({ control: collectionForm.control }) as CollectionForm;
  const token = useWatch({ control: tokenForm.control }) as TokenForm;
  const tiersValues = useWatch({ control: tiersForm.control }) as TiersForm;

  const maxSupply = Number(collection.maxSupply) || 0;
  const decimals = Number.isInteger(token.decimals) ? token.decimals : 0;
  const claimsEnabled = tiersValues.claimsEnabled;
  const saleEnabled = tiersValues.saleEnabled;
  const tiers = claimsEnabled
    ? resolveTiers(tiersValues.tiers ?? [], maxSupply, decimals)
    : { tiers: [], reserved: 0n, error: null };
  const saleResult = saleEnabled
    ? resolveSale(tiersValues.sale, decimals)
    : { config: null, total: 0n, error: null };
  const relationshipError =
    !claimsEnabled && !saleEnabled
      ? "Choose holder claims, a holder sale, or both"
      : (tiers.error ?? saleResult.error);
  // Claims and the sale pay out of one reserve held by the launch contract itself.
  const reservedTotal = tiers.reserved + saleResult.total;
  const reserveLabel =
    tiers.reserved > 0n && saleResult.total > 0n
      ? "Holder claims + sale"
      : tiers.reserved > 0n
        ? "NFT holder claims"
        : "NFT holder sale";
  const reserve: ContractAllocation[] =
    reservedTotal > 0n
      ? [
          {
            account_id: launchAccountId(collection.slug || "slug"),
            amount: reservedTotal.toString(),
            label: reserveLabel,
          },
        ]
      : [];
  const allocation = resolveAllocations(token.totalSupply ?? "", decimals, token.allocations ?? [], reserve);

  function complete(index: number) {
    setDone((d) => d.map((v, i) => (i === index ? true : v)));
    setStep(Math.min(index + 1, STEPS.length - 1));
  }

  const nextFromCollection = collectionForm.handleSubmit(() => complete(0));
  const nextFromTiers = () => (relationshipError ? undefined : complete(1));
  const review = (event?: BaseSyntheticEvent) => {
    tokenForm.setValue("slug", collection.slug);
    tokenForm.setValue("description", collection.description || "-");
    return submitToken(event);
  };
  const submitToken = tokenForm.handleSubmit(() => {
    if (allocation.error || relationshipError) return;
    setDone([true, true, true]);
    reset();
    setReviewing(true);
  });

  function launch() {
    void submit(
      {
        slug: collection.slug,
        description: collection.description.trim(),
        token: buildTokenConfig(token, reserve),
        collection: buildCollectionConfig(collection),
        claim: claimsEnabled
          ? { tiers: tiers.tiers, starts_at_ns: (toNs(tiersValues.claimStartsAt) ?? 0n).toString() }
          : null,
        sale: saleResult.config,
      },
      `Launch ${collection.name} + ${token.symbol}`,
    );
  }

  return (
    <CreateLayout
      title="Launch a connected token + NFT"
      description="Deploys an NFT collection and a token together, connected by contract-enforced rules: NFT holders can claim token allocations by tier, buy tokens in an exclusive holder window, or both."
      aside={
        <SummaryPanel
          title="Connected launch"
          footer={
            <>
              <KeyValue label="Deposit">
                <span className="tabular">{quote.data !== undefined ? formatNear(quote.data) : "—"}</span>
              </KeyValue>
              <p className="text-2xs text-ink-faint">
                Covers up to four contract accounts and the registry entry; unused balances are refunded.
              </p>
              {!accountId && (
                <Button variant="primary" className="w-full" onClick={() => void connect()}>
                  Connect wallet
                </Button>
              )}
            </>
          }
        >
          <dl>
            <KeyValue label="Collection">{collection.symbol || "—"}</KeyValue>
            <KeyValue label="NFT supply">{maxSupply ? maxSupply.toLocaleString() : "—"}</KeyValue>
            <KeyValue label="Token">{token.symbol || "—"}</KeyValue>
            <KeyValue label="Supply">
              <span className="tabular">
                {allocation.total ? formatCompact(allocation.total, decimals) : "—"}
              </span>
            </KeyValue>
            <KeyValue label="Holder reserve">
              <span className="tabular">
                {reservedTotal ? formatCompact(reservedTotal, decimals) : "—"}
                {allocation.total > 0n && reservedTotal > 0n && (
                  <span className="text-ink-faint"> · {formatShare(reservedTotal, allocation.total)}</span>
                )}
              </span>
            </KeyValue>
          </dl>
        </SummaryPanel>
      }
    >
      <Steps current={step} onSelect={setStep} done={done} />

      {step === 0 && (
        <form onSubmit={nextFromCollection} noValidate>
          <FormSection step={1} title="Collection" description="The NFTs that carry the token allocation.">
            <CollectionIdentityFields
              values={collection}
              register={collectionForm.register}
              setValue={collectionForm.setValue}
              errors={collectionForm.formState.errors}
              onNameChange={(name) => !slugEdited && collectionForm.setValue("slug", slugify(name))}
            />
            <Field label="Project description" error={collectionForm.formState.errors.description?.message}>
              {(props) => <Textarea {...props} {...collectionForm.register("description")} rows={3} />}
            </Field>
            <SlugField
              value={collection.slug}
              onChange={(v) => {
                setSlugEdited(true);
                collectionForm.setValue("slug", v, { shouldValidate: true });
              }}
              error={collectionForm.formState.errors.slug?.message}
            />
          </FormSection>
          <FormSection
            step={2}
            title="Mint rules"
            description="A connected launch needs a fixed supply so every NFT ID can be assigned a tier."
          >
            <MintRuleFields
              values={collection}
              register={collectionForm.register}
              setValue={collectionForm.setValue}
              errors={collectionForm.formState.errors}
              requireFixedSupply
            />
          </FormSection>
          <div className="flex justify-end border-t border-line pt-4">
            <Button type="submit" variant="primary">
              Continue
            </Button>
          </div>
        </form>
      )}

      {step === 1 && (
        <div>
          <FormSection
            step={3}
            title="Holder claims"
            description="NFT holders claim a free token allocation, by tier of NFT IDs."
          >
            <Toggle
              checked={claimsEnabled}
              onChange={(v) => tiersForm.setValue("claimsEnabled", v)}
              label="Enable holder claims"
              detail="A claim contract holds the reserve. Each NFT ID can claim its tier's amount once."
            />
            {claimsEnabled && (
              <>
                <Notice>
                  Eligibility is checked at claim time: whoever holds NFT #n when claiming receives its
                  allocation, once. Amounts are in {token.symbol || "your token"}; set the token&apos;s
                  decimals in the next step.
                </Notice>
                <TierEditor
                  control={tiersForm.control}
                  register={tiersForm.register}
                  errors={tiersForm.formState.errors}
                  result={tiers}
                  decimals={decimals}
                  symbol={token.symbol}
                  maxSupply={maxSupply}
                />
                <Field label="Claims open" hint="Leave empty to open claims as soon as the launch is live.">
                  {(props) => (
                    <Input
                      {...props}
                      {...tiersForm.register("claimStartsAt")}
                      type="datetime-local"
                      className="sm:max-w-64"
                    />
                  )}
                </Field>
              </>
            )}
          </FormSection>
          <FormSection
            step={4}
            title="Holder sale"
            description="NFT holders get an exclusive window to buy tokens at a fixed NEAR price."
          >
            <Toggle
              checked={saleEnabled}
              onChange={(v) => tiersForm.setValue("saleEnabled", v)}
              label="Enable NFT-gated sale"
              detail="The launch contract holds the tokens for sale and checks NFT ownership on every purchase. You are paid in the same step as each purchase."
            />
            {saleEnabled && (
              <SaleFields register={tiersForm.register} values={tiersValues.sale} symbol={token.symbol} />
            )}
          </FormSection>
          {relationshipError && <p className="pb-3 text-xs text-caution">{relationshipError}</p>}
          <div className="flex justify-between border-t border-line pt-4">
            <Button variant="ghost" onClick={() => setStep(0)}>
              Back
            </Button>
            <Button variant="primary" onClick={nextFromTiers} disabled={Boolean(relationshipError)}>
              Continue
            </Button>
          </div>
        </div>
      )}

      {step === 2 && (
        <form onSubmit={review} noValidate>
          <FormSection step={4} title="Token" description="The fungible token NFT holders claim.">
            <TokenIdentityFields register={tokenForm.register} errors={tokenForm.formState.errors} />
            <MediaField
              label="Token icon (optional)"
              value={token.icon ?? ""}
              onChange={(v) => tokenForm.setValue("icon", v, { shouldValidate: true })}
              error={tokenForm.formState.errors.icon?.message}
            />
          </FormSection>
          <FormSection
            step={5}
            title="Allocation"
            description="The holder reserve is fixed by your tiers. Allocate the rest of the supply below."
          >
            <AllocationEditor
              control={tokenForm.control}
              register={tokenForm.register}
              errors={tokenForm.formState.errors}
              result={allocation}
              decimals={decimals}
              symbol={token.symbol}
              fixed={reserve}
            />
            {allocation.total > 0n && tiers.reserved > allocation.total && (
              <Notice tone="negative">
                The holder reserve ({formatUnits(tiers.reserved, decimals)}) is larger than the total supply.
              </Notice>
            )}
          </FormSection>
          <div className="flex justify-between border-t border-line pt-4">
            <Button variant="ghost" onClick={() => setStep(1)}>
              Back
            </Button>
            <Button type="submit" variant="primary" disabled={!accountId}>
              Review launch
            </Button>
          </div>
        </form>
      )}

      <ReviewDialog
        open={reviewing}
        onOpenChange={setReviewing}
        title={`Launch ${collection.name} + ${token.symbol}`}
        state={state}
        deposit={quote.data ?? null}
        items={[
          {
            label: "Collection",
            value: `${collection.name} (${collection.symbol}) · ${maxSupply.toLocaleString()} max`,
          },
          {
            label: "Token",
            value: `${token.name} (${token.symbol}) · ${formatCompact(allocation.total, decimals)}`,
          },
          {
            label: "Holder claims",
            value: claimsEnabled
              ? `${formatCompact(tiers.reserved, decimals)} ${token.symbol} in ${tiers.tiers.length} tier(s)`
              : "None",
          },
          {
            label: "Holder sale",
            value: saleEnabled
              ? `${formatCompact(saleResult.total, decimals)} ${token.symbol} at ${formatNear(BigInt(saleResult.config?.price_per_token ?? "0"))}`
              : "None",
          },
          {
            label: "Claims open",
            value: tiersValues.claimStartsAt
              ? new Date(tiersValues.claimStartsAt).toLocaleString()
              : "At launch",
          },
          {
            label: "Contract",
            value: <span className="tabular text-xs">{launchAccountId(collection.slug)}</span>,
          },
        ]}
        consequences={[
          "The token supply is minted at launch; the holder reserve goes straight to the launch contract.",
          "Token, collection, claims and sale live in one contract, so claims and purchases check ownership and pay out in a single step.",
          "Each NFT ID can claim once, by whoever holds it at claim time. Unclaimed tokens stay in the launch contract.",
          "Sale purchases are limited per NFT (and per account in the public window); unsold tokens return to you after the end.",
          "The launch account is keyless: nobody can change these rules after launch.",
        ]}
        onConfirm={launch}
      />
    </CreateLayout>
  );
}

function Toggle({
  checked,
  onChange,
  label,
  detail,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
  detail: string;
}) {
  return (
    <label className="flex items-start gap-2.5 text-sm text-ink">
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="mt-0.5 size-4 accent-[var(--color-primary)]"
      />
      <span>
        {label}
        <span className="block text-xs text-ink-muted">{detail}</span>
      </span>
    </label>
  );
}

function SaleFields({
  register,
  values,
  symbol,
}: {
  register: UseFormRegister<TiersForm>;
  values: SaleForm;
  symbol: string;
}) {
  const unit = symbol || "tokens";
  return (
    <div className="space-y-4">
      <div className="grid gap-4 sm:grid-cols-3">
        <Field label={`Price per ${symbol || "token"} (NEAR)`}>
          {(props) => (
            <Input {...props} {...register("sale.price")} inputMode="decimal" className="tabular" />
          )}
        </Field>
        <Field label={`For sale (${unit})`} hint="Reserved from the supply.">
          {(props) => (
            <Input {...props} {...register("sale.total")} inputMode="decimal" className="tabular" />
          )}
        </Field>
        <Field label={`Per NFT (${unit})`} hint="Holder window limit.">
          {(props) => (
            <Input {...props} {...register("sale.perNft")} inputMode="decimal" className="tabular" />
          )}
        </Field>
      </div>
      <div className="grid gap-4 sm:grid-cols-2">
        <Field label="Holder window opens" hint="Leave empty to open at launch.">
          {(props) => <Input {...props} {...register("sale.holderStartsAt")} type="datetime-local" />}
        </Field>
        <Field label="Sale ends" hint="Required. Unsold tokens can then be returned to you.">
          {(props) => <Input {...props} {...register("sale.endsAt")} type="datetime-local" />}
        </Field>
      </div>
      <label className="flex items-center gap-2.5 text-sm text-ink">
        <input
          type="checkbox"
          {...register("sale.publicEnabled")}
          className="size-4 accent-[var(--color-primary)]"
        />
        Open to everyone after the holder window
      </label>
      {values.publicEnabled && (
        <div className="grid gap-4 sm:grid-cols-2">
          <Field label="Public window opens">
            {(props) => <Input {...props} {...register("sale.publicStartsAt")} type="datetime-local" />}
          </Field>
          <Field label={`Per account (${unit})`}>
            {(props) => (
              <Input {...props} {...register("sale.perAccount")} inputMode="decimal" className="tabular" />
            )}
          </Field>
        </div>
      )}
    </div>
  );
}
