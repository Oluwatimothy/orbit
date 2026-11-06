"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { useState } from "react";
import { useForm, useWatch } from "react-hook-form";

import { Button } from "@/components/ui/button";
import { KeyValue } from "@/components/ui/data";
import { Field, Textarea } from "@/components/ui/field";
import { useWallet } from "@/features/wallet/wallet-provider";
import { formatNear, tryParseUnits, YOCTO_DECIMALS } from "@/lib/format/units";
import {
  buildCollectionConfig,
  collectionFormSchema,
  slugify,
  type CollectionForm,
} from "@/lib/validation/launch";

import { COLLECTION_DEFAULTS, CollectionIdentityFields, MintRuleFields } from "./collection-fields";
import { SlugField } from "./fields";
import { CreateLayout, FormSection, SummaryPanel } from "./layout";
import { ReviewDialog } from "./review-dialog";
import { useCreateLaunch } from "./use-create-launch";

export function CreateCollectionPage() {
  const { accountId, connect } = useWallet();
  const form = useForm<CollectionForm>({
    resolver: zodResolver(collectionFormSchema),
    defaultValues: COLLECTION_DEFAULTS,
    mode: "onTouched",
  });
  const { register, control, handleSubmit, setValue, formState } = form;
  const values = useWatch({ control }) as CollectionForm;
  const [slugEdited, setSlugEdited] = useState(false);
  const [reviewing, setReviewing] = useState(false);
  const { quote, quoteProblem, submit, state, reset } = useCreateLaunch("collection");
  const price = tryParseUnits(values.price ?? "", YOCTO_DECIMALS);

  const onReview = handleSubmit(() => {
    reset();
    setReviewing(true);
  });

  return (
    <CreateLayout
      title="Launch an NFT collection"
      description="A NEP-171 collection with a public mint. Supply cap, price, wallet limit and mint window are written into the contract at launch and checked on every mint."
      aside={
        <SummaryPanel
          title="Summary"
          footer={
            <>
              <KeyValue label="Deposit">
                <span className="tabular">{quote.data !== undefined ? formatNear(quote.data) : "—"}</span>
              </KeyValue>
              <p className="text-2xs text-ink-faint">
                Minters pay for their own NFTs&apos; storage; this covers the collection account.
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
            <KeyValue label="Collection">{values.symbol || "—"}</KeyValue>
            <KeyValue label="Supply">
              {values.supplyMode === "fixed" ? values.maxSupply || "—" : "Unlimited"}
            </KeyValue>
            <KeyValue label="Price">{price !== null ? formatNear(price) : "—"}</KeyValue>
            <KeyValue label="Per wallet">{values.maxPerWallet || "No limit"}</KeyValue>
          </dl>
        </SummaryPanel>
      }
    >
      <form onSubmit={onReview} noValidate>
        <FormSection step={1} title="Collection" description="Name, symbol and artwork (NEP-177 metadata).">
          <CollectionIdentityFields
            values={values}
            register={register}
            setValue={setValue}
            errors={formState.errors}
            onNameChange={(name) => !slugEdited && setValue("slug", slugify(name))}
          />
          <Field label="Description" error={formState.errors.description?.message}>
            {(props) => <Textarea {...props} {...register("description")} rows={3} />}
          </Field>
          <SlugField
            value={values.slug}
            onChange={(v) => {
              setSlugEdited(true);
              setValue("slug", v, { shouldValidate: true });
            }}
            error={formState.errors.slug?.message}
          />
        </FormSection>
        <FormSection
          step={2}
          title="Mint rules"
          description="Enforced by the contract on every mint. None can be changed after launch."
        >
          <MintRuleFields values={values} register={register} setValue={setValue} errors={formState.errors} />
        </FormSection>
      </form>

      <ReviewDialog
        open={reviewing}
        onOpenChange={setReviewing}
        title={`Launch ${values.name}`}
        state={state}
        deposit={quote.data ?? null}
        depositProblem={quoteProblem}
        items={[
          { label: "Collection", value: `${values.name} (${values.symbol})` },
          {
            label: "Max supply",
            value: values.supplyMode === "fixed" ? Number(values.maxSupply).toLocaleString() : "Unlimited",
          },
          { label: "Price", value: price !== null ? formatNear(price) : "—" },
          { label: "Per wallet", value: values.maxPerWallet || "No limit" },
          {
            label: "Mint opens",
            value: values.startsAt ? new Date(values.startsAt).toLocaleString() : "Immediately",
          },
          { label: "Mint closes", value: values.endsAt ? new Date(values.endsAt).toLocaleString() : "Never" },
          { label: "Public mint", value: values.publicMint ? "Yes" : "Creator only" },
        ]}
        consequences={[
          "Price, supply, wallet limit and mint window are fixed in the contract.",
          "Mint proceeds are paid to your account in the mint transaction.",
          "You keep two controls: pausing minting, and minting to any account yourself (within the supply cap).",
        ]}
        onConfirm={() =>
          void submit(
            { slug: values.slug, collection: buildCollectionConfig(values) },
            `Launch ${values.name}`,
          )
        }
      />
    </CreateLayout>
  );
}
