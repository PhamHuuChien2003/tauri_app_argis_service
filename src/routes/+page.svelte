<script lang="ts">
	import { onMount } from 'svelte';
	import { writable } from 'svelte/store';
	import { invoke } from '@tauri-apps/api/core';
	import { listen, type EventCallback, type UnlistenFn } from '@tauri-apps/api/event';
	import DetailItem from './DetailItem.svelte';

	// Định nghĩa kiểu cho dữ liệu kết quả
	interface ExampleResult {
		status: string;
		address: string;
		province: string;
		district: string;
		ward: string;
		poi_vn?: string;
		poi_en?: string;
		poi_ex?: string;
		type?: string;
		sub_type?: string;
		poi_st_sd?: string;
		room?: string;
		house_num?: string;
		buaname?: string;
		st_name?: string;
		sub_com?: string;
		phone?: string;
		fax?: string;
		web?: string;
		mail?: string;
		brandname?: string;
		import?: string;
		status_detail?: string;
		note?: string;
		dine?: string;
		update_?: string;
		source?: string;
		gen_type?: string;
		perform?: string;
		dup?: string;
		explain?: string;
		classify?: string;
		dtrend?: string;
		google_id?: string;
		be_id?: string;
	}

	// Store để lưu trữ dữ liệu
	const resultData = writable<ExampleResult | null>(null);
	const showDialog = writable<boolean>(false);
	const isProcessing = writable<boolean>(false);

	onMount(() => {
		let unlisten: UnlistenFn | undefined;

		async function setupListener() {
			try {
				// Lắng nghe sự kiện hiển thị dialog xác nhận
				unlisten = await listen('show-confirm-dialog', (event: { payload: ExampleResult }) => {
					console.log('Received show-confirm-dialog event:', event.payload);
					resultData.set(event.payload);
					showDialog.set(true);
					isProcessing.set(true);
					
					// Gọi command Rust để mở rộng cửa sổ
					invoke('expand_window').catch(error => {
						console.error('Failed to expand window:', error);
					});
				});
			} catch (error) {
				console.error('Error setting up event listener:', error);
			}
		}

		setupListener();

		return () => {
			if (unlisten) {
				unlisten();
			}
		};
	});

	async function handleConfirm(): Promise<void> {
		const currentData = $resultData;
		if (currentData) {
			try {
				await invoke('confirm_result', { result: currentData });
				await invoke('collapse_window');
				showDialog.set(false);
				isProcessing.set(false);
				resultData.set(null);
			} catch (error) {
				console.error('Error confirming result:', error);
			}
		}
	}

	async function handleReject(): Promise<void> {
		try {
			await invoke('reject_result');
			await invoke('collapse_window');
			showDialog.set(false);
			isProcessing.set(false);
			resultData.set(null);
		} catch (error) {
			console.error('Error rejecting result:', error);
		}
	}
</script>

<div class="container h-full mx-auto py-4">
	{#if !$showDialog}
		<!-- Floating Mini Window -->
		<div class="flex flex-col items-center justify-center h-full space-y-2">
			<div class="text-center">
				<div class="w-8 h-8 mx-auto mb-2">
					<svg viewBox="0 0 24 24" fill="currentColor" class="text-primary-400">
						<path d="M12 2C8.13 2 5 5.13 5 9c0 5.25 7 13 7 13s7-7.75 7-13c0-3.87-3.13-7-7-7zm0 9.5c-1.38 0-2.5-1.12-2.5-2.5s1.12-2.5 2.5-2.5 2.5 1.12 2.5 2.5-1.12 2.5-2.5 2.5z"/>
					</svg>
				</div>
				<p class="text-xs text-surface-400 font-medium">Goong Geocoder</p>
				<p class="text-[10px] text-surface-500">Ready</p>
			</div>
		</div>
	{:else}
		<!-- Popup Dialog -->
		<div class="max-w-4xl mx-auto space-y-6">
			<!-- Header -->
			<div class="text-center space-y-2">
				<h1 class="h1 text-primary-600">Confirm Address</h1>
				<p class="text-surface-600">
					Please review the address information and confirm
				</p>
			</div>

			<!-- Results Display -->
			{#if $resultData}
				<div class="space-y-6">
					<!-- Address Card -->
					<div class="card variant-filled-primary p-6">
						<h2 class="h2 mb-4"> Address Information</h2>
						<div class="space-y-2">
							<DetailItem label="Full Address" value={$resultData.address} />
							<DetailItem label="Province" value={$resultData.province} />
							<DetailItem label="District" value={$resultData.district} />
							<DetailItem label="Ward" value={$resultData.ward} />
							<DetailItem label="Street Name" value={$resultData.st_name} optional={true} />
							<DetailItem label="House Number" value={$resultData.house_num} optional={true} />
						</div>
					</div>

					<!-- POI Information -->
					<div class="grid grid-cols-1 md:grid-cols-2 gap-6">
						<!-- POI Details -->
						<div class="card variant-filled-surface p-6">
							<h3 class="h3 mb-4"> POI Details</h3>
							<div class="space-y-2">
								<DetailItem label="POI Name (VN)" value={$resultData.poi_vn} optional={true} />
								<DetailItem label="POI Name (EN)" value={$resultData.poi_en} optional={true} />
								<DetailItem label="Type" value={$resultData.type} optional={true} />
								<DetailItem label="Sub Type" value={$resultData.sub_type} optional={true} />
								<DetailItem label="Brand Name" value={$resultData.brandname} optional={true} />
							</div>
						</div>

						<!-- Additional Information -->
						<div class="card variant-filled-surface p-6">
							<h3 class="h3 mb-4"> Additional Info</h3>
							<div class="space-y-2">
								<DetailItem label="Status Detail" value={$resultData.status_detail} optional={true} />
								<DetailItem label="Last Update" value={$resultData.update_} optional={true} />
								<DetailItem label="Generation Type" value={$resultData.gen_type} optional={true} />
								<DetailItem label="Data Source" value={$resultData.source} optional={true} />
								<DetailItem label="Google ID" value={$resultData.google_id} optional={true} />
								<DetailItem label="BE ID" value={$resultData.be_id} optional={true} />
							</div>
						</div>
					</div>

					<!-- Action Buttons -->
					<div class="flex justify-center space-x-4 pt-6">
						<button
							class="btn variant-filled-error px-8"
							on:click={handleReject}
						>
							Reject
						</button>
						<button
							class="btn variant-filled-success px-8"
							on:click={handleConfirm}
						>
							Confirm
						</button>
					</div>
				</div>
			{/if}
		</div>
	{/if}
</div>

<style lang="postcss">
	.container {
		min-height: 100vh;
		background: transparent;
	}

	:global(body) {
		background: transparent;
	}

	/* Animation for dialog */
	.card {
		animation: slideUp 0.3s ease-out;
	}

	@keyframes slideUp {
		from {
			opacity: 0;
			transform: translateY(20px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}
</style>