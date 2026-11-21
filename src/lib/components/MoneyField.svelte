<script lang="ts">
  import { Field, FieldDescription, FieldLabel } from "@ui-sdk/svelte/field";
  import { NumberInput, NumberInputControl, NumberInputGroup } from "@ui-sdk/svelte/number-input";
  import { poundsToPence } from "../format";

  interface Props {
    label: string;
    /** Minor units (pence). */
    value: number;
    description?: string;
    min?: number;
    currency?: string;
  }

  let { label, value = $bindable(), description, min = 0, currency = "GBP" }: Props = $props();
</script>

<Field>
  <FieldLabel>{label}</FieldLabel>
  <NumberInput
    value={value / 100}
    onValueChange={(next) => {
      if (next !== null) value = poundsToPence(next);
    }}
    {min}
    step={0.01}
    formatOptions={{ style: "currency", currency }}
  >
    <NumberInputGroup>
      <NumberInputControl />
    </NumberInputGroup>
  </NumberInput>
  {#if description}
    <FieldDescription>{description}</FieldDescription>
  {/if}
</Field>
