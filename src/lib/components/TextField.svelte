<script lang="ts">
  import { Field, FieldDescription, FieldLabel } from "@ui-sdk/svelte/field";
  import { Input } from "@ui-sdk/svelte/input";

  interface Props {
    label: string;
    value: string;
    description?: string;
    required?: boolean;
    placeholder?: string;
    type?: string;
    maxlength?: number;
  }

  let {
    label,
    value = $bindable(),
    description,
    required = false,
    placeholder,
    type = "text",
    maxlength,
  }: Props = $props();
</script>

<Field>
  <FieldLabel>{label}</FieldLabel>
  <Input bind:value {required} {placeholder} {type} {maxlength} />
  {#if description}
    <FieldDescription>{description}</FieldDescription>
  {/if}
</Field>
