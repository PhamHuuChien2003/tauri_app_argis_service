<script>
	export let label;
	export let value;
	export let optional = false;
</script>

{#if !optional || (value && value !== '')}
	<div class="grid grid-cols-3 gap-4 py-2 border-b border-surface-500/10">
		<span class="font-semibold text-surface-700">{label}:</span>
		<span class="col-span-2 text-surface-900">{value || 'N/A'}</span>
	</div>
{/if}