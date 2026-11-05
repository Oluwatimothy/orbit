"use client";

import { Plus, Trash2 } from "lucide-react";
import { useFieldArray, type Control, type FieldErrors, type UseFormRegister } from "react-hook-form";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/field";
import { formatUnits } from "@/lib/format/units";
import { LIMITS, type SaleForm, type TierResult, type TierRow } from "@/lib/validation/launch";

export interface TiersForm {
  claimsEnabled: boolean;
  tiers: TierRow[];
  claimStartsAt: string;
  saleEnabled: boolean;
  sale: SaleForm;
}

export function TierEditor({
  control,
  register,
  errors,
  result,
  decimals,
  symbol,
  maxSupply,
}: {
  control: Control<TiersForm>;
  register: UseFormRegister<TiersForm>;
  errors: FieldErrors<TiersForm>;
  result: TierResult;
  decimals: number;
  symbol: string;
  maxSupply: number;
}) {
  const { fields, append, remove } = useFieldArray({ control, name: "tiers" });
  const covered = result.tiers.reduce((n, t) => n + (t.to - t.from + 1), 0);

  return (
    <div className="space-y-3">
      <div className="overflow-x-auto rounded-md border border-line">
        <table className="w-full min-w-[500px] table-fixed text-left">
          <caption className="sr-only">Allocation tiers</caption>
          <thead>
            <tr className="border-b border-line text-2xs uppercase tracking-wider text-ink-faint">
              <th scope="col" className="w-[120px] py-2 pl-3 pr-1.5 font-medium">
                Tier
              </th>
              <th scope="col" className="w-[84px] px-1.5 py-2 font-medium">
                From ID
              </th>
              <th scope="col" className="w-[84px] px-1.5 py-2 font-medium">
                To ID
              </th>
              <th scope="col" className="px-1.5 py-2 font-medium">
                {symbol || "Tokens"} per NFT
              </th>
              <th scope="col" className="w-9 py-2 pl-1 pr-2">
                <span className="sr-only">Remove</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {fields.map((field, index) => {
              const rowErrors = errors.tiers?.[index];
              return (
                <tr key={field.id} className="border-b border-line last:border-0">
                  <td className="py-2 pl-3 pr-1.5">
                    <Input
                      {...register(`tiers.${index}.name`)}
                      aria-label={`Tier ${index + 1} name`}
                      aria-invalid={Boolean(rowErrors?.name)}
                    />
                  </td>
                  <td className="px-1.5 py-2">
                    <Input
                      {...register(`tiers.${index}.from`)}
                      inputMode="numeric"
                      className="tabular"
                      aria-label={`Tier ${index + 1} first NFT ID`}
                      aria-invalid={Boolean(rowErrors?.from)}
                    />
                  </td>
                  <td className="px-1.5 py-2">
                    <Input
                      {...register(`tiers.${index}.to`)}
                      inputMode="numeric"
                      className="tabular"
                      aria-label={`Tier ${index + 1} last NFT ID`}
                      aria-invalid={Boolean(rowErrors?.to)}
                    />
                  </td>
                  <td className="px-1.5 py-2">
                    <Input
                      {...register(`tiers.${index}.amount`)}
                      inputMode="decimal"
                      className="tabular"
                      aria-label={`Tier ${index + 1} amount per NFT`}
                      aria-invalid={Boolean(rowErrors?.amount)}
                    />
                  </td>
                  <td className="py-2 pl-1 pr-2">
                    <button
                      type="button"
                      onClick={() => remove(index)}
                      disabled={fields.length === 1}
                      className="rounded-xs p-1.5 text-ink-faint hover:text-negative disabled:opacity-30"
                      aria-label={`Remove tier ${index + 1}`}
                    >
                      <Trash2 className="size-3.5" aria-hidden />
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <Button
          type="button"
          size="sm"
          variant="secondary"
          disabled={fields.length >= LIMITS.tiers}
          onClick={() => {
            const last = result.tiers.at(-1);
            const from = last ? last.to + 1 : 1;
            append({
              name: `Tier ${fields.length + 1}`,
              from: String(from),
              to: String(Math.min(maxSupply, from)),
              amount: "",
            });
          }}
        >
          <Plus className="size-3.5" aria-hidden /> Add tier
        </Button>
        <p
          className={`tabular text-xs ${result.error ? "text-caution" : "text-ink-muted"}`}
          aria-live="polite"
        >
          {result.error ??
            `${covered.toLocaleString()} of ${maxSupply.toLocaleString()} NFTs eligible · reserve ${formatUnits(result.reserved, decimals, { maxFractionDigits: 4 })} ${symbol}`}
        </p>
      </div>
    </div>
  );
}
