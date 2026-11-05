"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { useState } from "react";
import { useForm, useWatch } from "react-hook-form";

import { Button } from "@/components/ui/button";
import { Field, Textarea } from "@/components/ui/field";
import { KeyValue } from "@/components/ui/data";
import { useWallet } from "@/features/wallet/wallet-provider";
import { formatCompact, formatNear } from "@/lib/format/units";
import {
  buildTokenConfig,
  resolveAllocations,
  slugify,
  tokenFormSchema,
  type TokenForm,
} from "@/lib/validation/launch";

import { AllocationEditor } from "./allocation-editor";
import { MediaField, SlugField } from "./fields";
import { CreateLayout, FormSection, SummaryPanel } from "./layout";
import { ReviewDialog } from "./review-dialog";
import { TokenIdentityFields } from "./token-fields";
import { useCreateLaunch } from "./use-create-launch";

const DEFAULTS: TokenForm = {
  slug: "",
  name: "",
  symbol: "",
  description: "",
  icon: "",
  decimals: 18,
  totalSupply: "",
  allocations: [{ label: "Creator", accountId: "", mode: "percent", value: "100" }],
};

export function CreateTokenPage() {
  const { accountId, connect } = useWallet();
  const form = useForm<TokenForm>({
    resolver: zodResolver(tokenFormSchema),
    defaultValues: DEFAULTS,
    mode: "onTouched",
  });
  const { register, control, handleSubmit, setValue, formState } = form;
  const values = useWatch({ control }) as TokenForm;
  const [slugEdited, setSlugEdited] = useState(false);
  const [reviewing, setReviewing] = useState(false);
  const { quote, submit, state, reset } = useCreateLaunch("token");

  const decimals = Number.isInteger(values.decimals) ? values.decimals : 0;
  const allocation = resolveAllocations(values.totalSupply ?? "", decimals, values.allocations ?? []);

  function fillCreator() {
    values.allocations?.forEach((row, i) => {
      if (!row.accountId && accountId)
        setValue(`allocations.${i}.accountId`, accountId, { shouldValidate: true });
    });
  }

  const onReview = handleSubmit(() => {
    if (!allocation.error) {
      reset();
      setReviewing(true);
    }
  });

  return (
    <CreateLayout
      title="Launch a token"
      description="A fixed-supply NEP-141 token. The whole supply is minted to your allocations in the deployment transaction; no account — including yours — can mint more."
      aside={
        <SummaryPanel
          title="Summary"
          footer={
            <>
              <KeyValue label="Deposit">
                <span className="tabular">{quote.data !== undefined ? formatNear(quote.data) : "—"}</span>
              </KeyValue>
              <p className="text-2xs text-ink-faint">
                Covers the token account&apos;s storage and its registry entry. Unused registry allowance is
                refunded; the account balance stays locked with the keyless contract.
              </p>
              {accountId ? (
                <Button variant="primary" className="w-full" onClick={onReview}>
                  Review launch
                </Button>
              ) : (
                <Button variant="primary" className="w-full" onClick={() => void connect()}>
                  Connect wallet
                </Button>
              )}
            </>
          }
        >
          <dl>
            <KeyValue label="Token">{values.symbol || "—"}</KeyValue>
            <KeyValue label="Supply">
              <span className="tabular">
                {allocation.total ? formatCompact(allocation.total, decimals) : "—"}
              </span>
            </KeyValue>
            <KeyValue label="Allocations">{values.allocations?.length ?? 0}</KeyValue>
            <KeyValue label="Status">
              <span className={allocation.error ? "text-caution" : "text-positive"}>
                {allocation.error ? "Incomplete" : "Ready"}
              </span>
            </KeyValue>
          </dl>
        </SummaryPanel>
      }
    >
      <form onSubmit={onReview} noValidate>
        <FormSection
          step={1}
          title="Identity"
          description="How wallets and explorers display your token (NEP-148)."
        >
          <TokenIdentityFields
            register={register}
            errors={formState.errors}
            onNameChange={(name) => !slugEdited && setValue("slug", slugify(name))}
          />
          <Field
            label="Description"
            error={formState.errors.description?.message}
            hint="Shown on the launch page. Stored in the Orbit registry."
          >
            {(props) => <Textarea {...props} {...register("description")} rows={3} />}
          </Field>
          <MediaField
            label="Icon (optional)"
            value={values.icon ?? ""}
            onChange={(v) => setValue("icon", v, { shouldValidate: true })}
            error={formState.errors.icon?.message}
            hint="Square image. Wallets show it next to your balance."
          />
          <SlugField
            value={values.slug ?? ""}
            onChange={(v) => {
              setSlugEdited(true);
              setValue("slug", v, { shouldValidate: true });
            }}
            error={formState.errors.slug?.message}
            suffixes={["ft"]}
          />
        </FormSection>

        <FormSection
          step={2}
          title="Allocation"
          description="Who receives the supply at launch. Amounts or percentages must add up to exactly the total supply."
        >
          <AllocationEditor
            control={control}
            register={register}
            errors={formState.errors}
            result={allocation}
            decimals={decimals}
            symbol={values.symbol}
          />
          {accountId && (
            <button
              type="button"
              onClick={fillCreator}
              className="text-xs text-ink-muted underline-offset-2 hover:underline"
            >
              Fill empty recipients with {accountId}
            </button>
          )}
          <p className="text-xs text-ink-faint">
            Vesting is not supported yet: allocations are transferred in full at launch. Send tokens to a
            vesting contract yourself if you need lock-ups.
          </p>
        </FormSection>
      </form>

      <ReviewDialog
        open={reviewing}
        onOpenChange={setReviewing}
        title={`Launch ${values.symbol}`}
        state={state}
        deposit={quote.data ?? null}
        items={[
          { label: "Name", value: values.name },
          { label: "Symbol", value: <span className="tabular">{values.symbol}</span> },
          {
            label: "Total supply",
            value: <span className="tabular">{formatCompact(allocation.total, decimals)}</span>,
          },
          { label: "Decimals", value: decimals },
          { label: "Allocations", value: allocation.allocations.length },
          { label: "Contract", value: <span className="tabular text-xs">{values.slug}-ft</span> },
        ]}
        consequences={[
          "The total supply is minted once to the listed accounts. No mint method exists afterwards.",
          "The token account is created without access keys and cannot be upgraded by anyone.",
          "Your launch is recorded in the public Orbit registry on testnet.",
        ]}
        onConfirm={() =>
          void submit(
            { slug: values.slug, description: values.description.trim(), token: buildTokenConfig(values) },
            `Launch ${values.symbol}`,
          )
        }
      />
    </CreateLayout>
  );
}
