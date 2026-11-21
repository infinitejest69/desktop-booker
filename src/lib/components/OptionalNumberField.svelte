<script lang="ts">
  import { Field, FieldLabel } from "@ui-sdk/svelte/field";
  import { NumberInput, NumberInputControl, NumberInputGroup } from "@ui-sdk/svelte/number-input";

  interface Props {
    label: string;
    value: number | null;
    min?: number;
    max?: number;
  }

  let { label, value = $bindable(), min = 0, max }: Props = $props();
</script>

<Field>
  <FieldLabel>{label}</FieldLabel>
  <NumberInput bind:value {min} {max} step={1}>
    <NumberInputGroup>
      <NumberInputControl placeholder="—" />
    </NumberInputGroup>
  </NumberInput>
</Field>
