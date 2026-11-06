"use client";

import { Plus, Trash2 } from "lucide-react";
import { useFieldArray, type Control, type FieldErrors, type UseFormRegister } from "react-hook-form";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/field";
import { formatShare, formatUnits } from "@/lib/format/units";
import type { AllocationResult, ContractAllocation, TokenForm } from "@/lib/validation/launch";
import { LIMITS } from "@/lib/validation/launch";

const SUGGESTED_LABELS = ["Creator", "Community", "Treasury", "Liquidity reserve", "Ecosystem"];

/**
 * Allocation rows for a token. `fixed` rows (the holder reserve of a connected
 * launch) are shown read-only above the editable ones.
 */
export function AllocationEditor({
  control,
  register,
  errors,
  result,
  decimals,
  symbol,
  fixed = [],
}: {
  control: Control<TokenForm>;
  register: UseFormRegister<TokenForm>;
  errors: FieldErrors<TokenForm>;
  result: AllocationResult;
  decimals: number;
  symbol: string;
  fixed?: ContractAllocation[];
}) {
  const { fields, append, remove } = useFieldArray({ control, name: "allocations" });
  const canAdd = fields.length + fixed.length < LIMITS.allocations;

  return (
    <div className="space-y-3">
      <div className="overflow-x-auto rounded-md border border-line">
        <table className="w-full min-w-[500px] table-fixed text-left">
          <caption className="sr-only">Token allocations</caption>
          <thead>
            <tr className="border-b border-line text-2xs uppercase tracking-wider text-ink-faint">
              <th scope="col" className="w-[112px] py-2 pl-3 pr-1.5 font-medium">
                Label
              </th>
              <th scope="col" className="px-1.5 py-2 font-medium">
                Recipient
              </th>
              <th scope="col" className="w-[150px] px-1.5 py-2 font-medium">
                Amount
              </th>
              <th scope="col" className="w-[52px] px-1.5 py-2 text-right font-medium">
                Share
              </th>
              <th scope="col" className="w-9 py-2 pl-1 pr-2">
                <span className="sr-only">Remove</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {fixed.map((a) => (
              <tr key={a.account_id} className="border-b border-line bg-raised/40">
                <td className="py-2 pl-3 pr-1.5 text-sm text-ink">{a.label}</td>
                <td className="tabular truncate px-1.5 py-2 text-xs text-ink-muted">{a.account_id}</td>
                <td className="tabular px-1.5 py-2 text-sm">
                  {formatUnits(BigInt(a.amount), decimals, { maxFractionDigits: 4 })}
                </td>
                <td className="tabular px-1.5 py-2 text-right text-xs text-ink-muted">
                  {result.total ? formatShare(BigInt(a.amount), result.total) : "—"}
                </td>
                <td className="py-2 pl-1 pr-2 text-2xs text-ink-faint">Auto</td>
              </tr>
            ))}
            {fields.map((field, index) => {
              const rowError = errors.allocations?.[index];
              const resolved = result.allocations[fixed.length + index];
              return (
                <tr key={field.id} className="border-b border-line align-top last:border-0">
                  <td className="py-2 pl-3 pr-1.5">
                    <Input
                      {...register(`allocations.${index}.label`)}
                      list="allocation-labels"
                      aria-label={`Allocation ${index + 1} label`}
                      aria-invalid={Boolean(rowError?.label)}
                    />
                    {rowError?.label && (
                      <p className="mt-1 text-2xs text-negative">{rowError.label.message}</p>
                    )}
                  </td>
                  <td className="px-1.5 py-2">
                    <Input
                      {...register(`allocations.${index}.accountId`)}
                      placeholder="account.testnet"
                      spellCheck={false}
                      aria-label={`Allocation ${index + 1} recipient`}
                      aria-invalid={Boolean(rowError?.accountId)}
                    />
                    {rowError?.accountId && (
                      <p className="mt-1 text-2xs text-negative">{rowError.accountId.message}</p>
                    )}
                  </td>
                  <td className="px-1.5 py-2">
                    <div className="flex">
                      <Input
                        {...register(`allocations.${index}.value`)}
                        inputMode="decimal"
                        className="min-w-0 rounded-r-none"
                        aria-label={`Allocation ${index + 1} amount`}
                        aria-invalid={Boolean(rowError?.value)}
                      />
                      <select
                        {...register(`allocations.${index}.mode`)}
                        className="h-8 w-14 shrink-0 rounded-r-sm border border-l-0 border-line-strong bg-raised px-1 text-xs text-ink-muted"
                        aria-label={`Allocation ${index + 1} unit`}
                      >
                        <option value="amount">{symbol ? symbol.slice(0, 6) : "Units"}</option>
                        <option value="percent">%</option>
                      </select>
                    </div>
                  </td>
                  <td className="tabular px-1.5 py-2 pt-3.5 text-right text-xs text-ink-muted">
                    {resolved && result.total ? formatShare(BigInt(resolved.amount), result.total) : "—"}
                  </td>
                  <td className="py-2 pl-1 pr-2">
                    <button
                      type="button"
                      onClick={() => remove(index)}
                      className="rounded-xs p-1.5 text-ink-faint hover:text-negative"
                      aria-label={`Remove allocation ${index + 1}`}
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
      <datalist id="allocation-labels">
        {SUGGESTED_LABELS.map((label) => (
          <option key={label} value={label} />
        ))}
      </datalist>

      <div className="flex flex-wrap items-center justify-between gap-3">
        <Button
          type="button"
          size="sm"
          variant="secondary"
          disabled={!canAdd}
          onClick={() => append({ label: "", accountId: "", mode: "percent", value: "" })}
        >
          <Plus className="size-3.5" aria-hidden /> Add allocation
        </Button>
        <AllocationTotals result={result} decimals={decimals} symbol={symbol} />
      </div>
    </div>
  );
}

function AllocationTotals({
  result,
  decimals,
  symbol,
}: {
  result: AllocationResult;
  decimals: number;
  symbol: string;
}) {
  if (!result.total) return null;
  const done = result.unallocated === 0n && !result.error;
  return (
    <p className={`tabular text-xs ${done ? "text-positive" : "text-caution"}`} aria-live="polite">
      {done
        ? `Fully allocated${result.dust ? ` (${formatUnits(result.dust, decimals)} rounding added to the first % row)` : ""}`
        : result.unallocated > 0n
          ? `${formatUnits(result.unallocated, decimals)} ${symbol} unallocated`
          : result.error}
    </p>
  );
}
