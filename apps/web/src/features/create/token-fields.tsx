"use client";

import type { FieldErrors, UseFormRegister } from "react-hook-form";

import { Field, Input } from "@/components/ui/field";
import type { TokenForm } from "@/lib/validation/launch";

/** Name, symbol, decimals and supply — shared by the token and connected flows. */
export function TokenIdentityFields({
  register,
  errors,
  onNameChange,
}: {
  register: UseFormRegister<TokenForm>;
  errors: FieldErrors<TokenForm>;
  onNameChange?: (name: string) => void;
}) {
  return (
    <>
      <div className="grid gap-4 sm:grid-cols-[1fr_160px]">
        <Field label="Token name" error={errors.name?.message}>
          {(props) => (
            <Input
              {...props}
              {...register("name", { onChange: (e) => onNameChange?.(e.target.value) })}
              placeholder="Orbit Genesis"
            />
          )}
        </Field>
        <Field label="Symbol" error={errors.symbol?.message}>
          {(props) => (
            <Input
              {...props}
              {...register("symbol", { setValueAs: (v: string) => v.toUpperCase() })}
              placeholder="ORBIT"
              className="tabular uppercase"
            />
          )}
        </Field>
      </div>
      <div className="grid gap-4 sm:grid-cols-[1fr_160px]">
        <Field
          label="Total supply"
          error={errors.totalSupply?.message}
          hint="Minted once at launch. The contract has no way to mint more."
        >
          {(props) => (
            <Input
              {...props}
              {...register("totalSupply")}
              inputMode="decimal"
              placeholder="10,000,000"
              className="tabular"
            />
          )}
        </Field>
        <Field label="Decimals" error={errors.decimals?.message} hint="18 is common; 0–24.">
          {(props) => (
            <Input
              {...props}
              {...register("decimals", { valueAsNumber: true })}
              type="number"
              min={0}
              max={24}
              className="tabular"
            />
          )}
        </Field>
      </div>
    </>
  );
}
