<script lang="ts">
  import { Field, FieldDescription, FieldLabel } from "@ui-sdk/svelte/field";
  import {
    NumberInput,
    NumberInputControl,
    NumberInputDecrement,
    NumberInputGroup,
    NumberInputIncrement,
  } from "@ui-sdk/svelte/number-input";

  interface Props {
    label: string;
    value: number;
    min?: number;
    max?: number;
    description?: string;
    required?: boolean;
  }

  let { label, value = $bindable(), min, max, description, required = false }: Props = $props();
</script>

<Field>
  <FieldLabel>{label}</FieldLabel>
  <NumberInput
    {value}
    onValueChange={(next) => {
      if (next !== null) value = next;
    }}
    {min}
    {max}
    step={1}
    {required}
  >
    <NumberInputGroup>
      <NumberInputDecrement aria-label={`Decrease ${label}`} />
      <NumberInputControl />
      <NumberInputIncrement aria-label={`Increase ${label}`} />
    </NumberInputGroup>
  </NumberInput>
  {#if description}
    <FieldDescription>{description}</FieldDescription>
  {/if}
</Field>
