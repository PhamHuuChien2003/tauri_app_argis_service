<script lang="ts">
	import { onMount } from 'svelte';
	import { writable } from 'svelte/store';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
	import DetailItem from './DetailItem.svelte';

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

	interface ApiConfig {
		provider: string;
		goong_api_key: string;
		google_api_key: string;
	}

	// Store để lưu trữ dữ liệu
	const resultData = writable<ExampleResult | null>(null);
	const showDialog = writable<boolean>(false);
	const showConfig = writable<boolean>(false);
	const showProviderSelector = writable<boolean>(false);
	const showGoongKeyInput = writable<boolean>(false);
	const showGoogleKeyInput = writable<boolean>(false);
	const isProcessing = writable<boolean>(false);
	const apiConfig = writable<ApiConfig>({
		provider: 'goong',
		goong_api_key: '',
		google_api_key: ''
	});

	let isDragging = false;
	let dragOffset = { x: 0, y: 0 };
	let currentWindow: any;
	let currentPosition = { x: 0, y: 0 };
	let newGoongKey = '';
	let newGoogleKey = '';

	onMount(() => {
		let unlistenShowDialog: (() => void) | undefined;
		let unlistenProviderSelector: (() => void) | undefined;
		let unlistenGoongKeyInput: (() => void) | undefined;
		let unlistenGoogleKeyInput: (() => void) | undefined;

		async function setupListeners() {
			try {
				currentWindow = getCurrentWebviewWindow();
				
				// Load cấu hình API
				loadApiConfig();
				
				// Lấy vị trí hiện tại của cửa sổ
				updateWindowPosition();
				
				// Lắng nghe sự kiện từ system tray
				unlistenShowDialog = await listen('show-confirm-dialog', (event: { payload: ExampleResult }) => {
					console.log('Received show-confirm-dialog event:', event.payload);
					resultData.set(event.payload);
					showDialog.set(true);
					isProcessing.set(true);
					
					invoke('expand_window').catch(error => {
						console.error('Failed to expand window:', error);
					});
				});

				unlistenProviderSelector = await listen('open-provider-selector', () => {
					console.log('Opening provider selector');
					showProviderSelector.set(true);
				});

				unlistenGoongKeyInput = await listen('open-goong-key-input', () => {
					console.log('Opening Goong API key input');
					newGoongKey = $apiConfig.goong_api_key;
					showGoongKeyInput.set(true);
				});

				unlistenGoogleKeyInput = await listen('open-google-key-input', () => {
					console.log('Opening Google API key input');
					newGoogleKey = $apiConfig.google_api_key;
					showGoogleKeyInput.set(true);
				});
			} catch (error) {
				console.error('Error setting up event listeners:', error);
			}
		}

		async function loadApiConfig() {
			try {
				const config: ApiConfig = await invoke('get_api_config');
				apiConfig.set(config);
			} catch (error) {
				console.error('Failed to load API config:', error);
			}
		}

		async function updateWindowPosition() {
			try {
				const position = await invoke('get_window_position');
				if (Array.isArray(position)) {
					currentPosition.x = position[0];
					currentPosition.y = position[1];
				}
			} catch (error) {
				console.error('Failed to get window position:', error);
			}
		}

		setupListeners();

		return () => {
			if (unlistenShowDialog) unlistenShowDialog();
			if (unlistenProviderSelector) unlistenProviderSelector();
			if (unlistenGoongKeyInput) unlistenGoongKeyInput();
			if (unlistenGoogleKeyInput) unlistenGoogleKeyInput();
		};
	});

	// Hàm bắt đầu kéo cửa sổ
	function startDrag(event: MouseEvent) {
		if ($showDialog) return;
		
		event.preventDefault();
		event.stopPropagation();
		
		isDragging = true;
		const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
		dragOffset.x = event.clientX - rect.left;
		dragOffset.y = event.clientY - rect.top;
		
		document.addEventListener('mousemove', handleDrag);
		document.addEventListener('mouseup', stopDrag);
		
		document.body.style.cursor = 'grabbing';
		document.body.style.userSelect = 'none';
	}

	// Hàm xử lý kéo cửa sổ
	async function handleDrag(event: MouseEvent) {
		if (!isDragging) return;
		
		event.preventDefault();
		event.stopPropagation();
		
		const newX = event.screenX - dragOffset.x;
		const newY = event.screenY - dragOffset.y;
		
		try {
			await invoke('set_window_position', { x: newX, y: newY });
			currentPosition.x = newX;
			currentPosition.y = newY;
		} catch (error) {
			console.error('Error moving window:', error);
		}
	}

	// Hàm dừng kéo cửa sổ
	function stopDrag() {
		if (!isDragging) return;
		
		isDragging = false;
		document.removeEventListener('mousemove', handleDrag);
		document.removeEventListener('mouseup', stopDrag);
		document.body.style.cursor = '';
		document.body.style.userSelect = '';
	}

	// Hàm xử lý touch events
	function startDragTouch(event: TouchEvent) {
		if ($showDialog) return;
		
		event.preventDefault();
		event.stopPropagation();
		
		isDragging = true;
		const touch = event.touches[0];
		const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
		dragOffset.x = touch.clientX - rect.left;
		dragOffset.y = touch.clientY - rect.top;
		
		document.addEventListener('touchmove', handleDragTouch, { passive: false });
		document.addEventListener('touchend', stopDragTouch);
	}

	async function handleDragTouch(event: TouchEvent) {
		if (!isDragging) return;
		
		event.preventDefault();
		event.stopPropagation();
		
		const touch = event.touches[0];
		const newX = touch.screenX - dragOffset.x;
		const newY = touch.screenY - dragOffset.y;
		
		try {
			await invoke('set_window_position', { x: newX, y: newY });
			currentPosition.x = newX;
			currentPosition.y = newY;
		} catch (error) {
			console.error('Error moving window:', error);
		}
	}

	function stopDragTouch() {
		if (!isDragging) return;
		
		isDragging = false;
		document.removeEventListener('touchmove', handleDragTouch);
		document.removeEventListener('touchend', stopDragTouch);
	}

	// Cập nhật provider
	async function updateProvider(newProvider: string) {
		try {
			const newConfig: ApiConfig = {
				...$apiConfig,
				provider: newProvider
			};
			await invoke('update_api_config', { newConfig });
			apiConfig.set(newConfig);
			showProviderSelector.set(false);
		} catch (error) {
			console.error('Failed to update provider:', error);
		}
	}

	// Cập nhật Goong API key
	async function updateGoongKey() {
		try {
			const newConfig: ApiConfig = {
				...$apiConfig,
				goong_api_key: newGoongKey
			};
			await invoke('update_api_config', { newConfig });
			apiConfig.set(newConfig);
			showGoongKeyInput.set(false);
			newGoongKey = '';
		} catch (error) {
			console.error('Failed to update Goong API key:', error);
		}
	}

	// Cập nhật Google API key
	async function updateGoogleKey() {
		try {
			const newConfig: ApiConfig = {
				...$apiConfig,
				google_api_key: newGoogleKey
			};
			await invoke('update_api_config', { newConfig });
			apiConfig.set(newConfig);
			showGoogleKeyInput.set(false);
			newGoogleKey = '';
		} catch (error) {
			console.error('Failed to update Google API key:', error);
		}
	}

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

<div class="container h-full mx-auto" class:floating-mode={!$showDialog} class:popup-mode={$showDialog}>
	{#if !$showDialog}
		<!-- Floating Icon - Có thể kéo được -->
		<div 
			class="floating-container"
			role="button"
			tabindex="0"
			on:mousedown={startDrag}
			on:touchstart={startDragTouch}
			on:contextmenu|preventDefault={() => showConfig.set(true)}
			class:cursor-grab={!isDragging}
			class:cursor-grabbing={isDragging}
		>
			<div class="flex flex-col items-center justify-center h-full space-y-1">
				<div class="text-center p-2">
					<!-- Icon chính -->
					<div class="w-10 h-10 mx-auto mb-1 bg-primary-500 rounded-full flex items-center justify-center shadow-lg">
						<svg viewBox="0 0 24 24" fill="white" class="w-6 h-6">
							<path d="M12 2C8.13 2 5 5.13 5 9c0 5.25 7 13 7 13s7-7.75 7-13c0-3.87-3.13-7-7-7zm0 9.5c-1.38 0-2.5-1.12-2.5-2.5s1.12-2.5 2.5-2.5 2.5 1.12 2.5 2.5-1.12 2.5-2.5 2.5z"/>
						</svg>
					</div>
					<p class="text-[10px] text-surface-400 font-medium">Geocoder</p>
					<p class="text-[8px] text-surface-500">{$apiConfig.provider}</p>
					<p class="text-[7px] text-surface-400 mt-1">Drag to move</p>
				</div>
			</div>
		</div>
	{:else}
		<!-- Popup Dialog - Không thể kéo -->
		<div class="max-w-4xl mx-auto space-y-6 p-6">
			<!-- Header -->
			<div class="text-center space-y-2">
				<h1 class="h1 text-primary-600">Confirm Address</h1>
				<p class="text-surface-600">
					Please review the address information and confirm
				</p>
				<p class="text-sm text-surface-500">
					Using: <strong>{$apiConfig.provider.toUpperCase()} API</strong>
				</p>
			</div>

			<!-- Results Display -->
			{#if $resultData}
				<div class="space-y-6">
					<!-- Address Card -->
					<div class="card variant-filled-primary p-6">
						<h2 class="h2 mb-4">📍 Address Information</h2>
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
							<h3 class="h3 mb-4">🏢 POI Details</h3>
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
							<h3 class="h3 mb-4">📊 Additional Info</h3>
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

	<!-- Provider Selector Popup -->
	{#if $showProviderSelector}
		<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
			<div class="bg-surface-800 rounded-lg p-6 w-80">
				<h3 class="text-lg font-semibold text-surface-200 mb-4">Select Provider</h3>
				<div class="space-y-3">
										<button
						class="w-full p-3 text-left rounded border {$apiConfig.provider === 'goong' ? 'border-primary-500 bg-primary-500 bg-opacity-20' : 'border-surface-600 hover:border-surface-500'}"
						on:click={() => updateProvider('goong')}
					>
						<div class="font-medium text-surface-200">Goong API</div>
						<div class="text-sm text-surface-400 mt-1">Sử dụng Goong Geocoding API</div>
					</button>
					
					<button
						class="w-full p-3 text-left rounded border {$apiConfig.provider === 'google' ? 'border-primary-500 bg-primary-500 bg-opacity-20' : 'border-surface-600 hover:border-surface-500'}"
						on:click={() => updateProvider('google')}
					>
						<div class="font-medium text-surface-200">Google Geocoding</div>
						<div class="text-sm text-surface-400 mt-1">Sử dụng Google Maps Geocoding API</div>
					</button>
				</div>
				<div class="flex justify-end mt-6">
					<button
						class="btn variant-filled-surface px-4"
						on:click={() => showProviderSelector.set(false)}
					>
						Cancel
					</button>
				</div>
			</div>
		</div>
	{/if}

	<!-- Goong API Key Input Popup -->
	{#if $showGoongKeyInput}
		<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
			<div class="bg-surface-800 rounded-lg p-6 w-96">
				<h3 class="text-lg font-semibold text-surface-200 mb-4">Set Goong API Key</h3>
				<div class="space-y-4">
					<div>
						<label class="block text-sm font-medium text-surface-400 mb-2">Goong API Key</label>
						<input
							type="password"
							class="w-full bg-surface-700 border border-surface-600 rounded px-3 py-2 text-surface-200"
							placeholder="Nhập Goong API key"
							bind:value={newGoongKey}
						/>
						<p class="text-xs text-surface-500 mt-1">
							API key sẽ được lưu và sử dụng cho các lần chạy sau
						</p>
					</div>
				</div>
				<div class="flex justify-end space-x-3 mt-6">
					<button
						class="btn variant-filled-surface px-4"
						on:click={() => {
							showGoongKeyInput.set(false);
							newGoongKey = '';
						}}
					>
						Cancel
					</button>
					<button
						class="btn variant-filled-success px-4"
						on:click={updateGoongKey}
						disabled={!newGoongKey.trim()}
					>
						Save
					</button>
				</div>
			</div>
		</div>
	{/if}

	<!-- Google API Key Input Popup -->
	{#if $showGoogleKeyInput}
		<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
			<div class="bg-surface-800 rounded-lg p-6 w-96">
				<h3 class="text-lg font-semibold text-surface-200 mb-4">Set Google API Key</h3>
				<div class="space-y-4">
					<div>
						<label class="block text-sm font-medium text-surface-400 mb-2">Google API Key</label>
						<input
							type="password"
							class="w-full bg-surface-700 border border-surface-600 rounded px-3 py-2 text-surface-200"
							placeholder="Nhập Google API key"
							bind:value={newGoogleKey}
						/>
						<p class="text-xs text-surface-500 mt-1">
							API key sẽ được lưu và sử dụng cho các lần chạy sau
						</p>
					</div>
				</div>
				<div class="flex justify-end space-x-3 mt-6">
					<button
						class="btn variant-filled-surface px-4"
						on:click={() => {
							showGoogleKeyInput.set(false);
							newGoogleKey = '';
						}}
					>
						Cancel
					</button>
					<button
						class="btn variant-filled-success px-4"
						on:click={updateGoogleKey}
						disabled={!newGoogleKey.trim()}
					>
						Save
					</button>
				</div>
			</div>
		</div>
	{/if}

	<!-- Thêm phần này vào cuối file page.svelte, trước thẻ đóng </div> -->

	<!-- System Tray Context Menu (hiện khi click chuột phải vào icon) -->
	{#if !$showDialog}
		<div class="context-menu-overlay"  role="presentation" on:click|self={() => showConfig.set(false)}>
			<div class="context-menu" class:show={$showConfig}>
				<button class="context-menu-item" on:click={() => {
					showProviderSelector.set(true);
					showConfig.set(false);
				}}>
					<svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
					</svg>
					Select Provider
				</button>
				<button class="context-menu-item" on:click={() => {
					newGoongKey = $apiConfig.goong_api_key;
					showGoongKeyInput.set(true);
					showConfig.set(false);
				}}>
					<svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/>
					</svg>
					Set Goong API Key
				</button>
				<button class="context-menu-item" on:click={() => {
					newGoogleKey = $apiConfig.google_api_key;
					showGoogleKeyInput.set(true);
					showConfig.set(false);
				}}>
					<svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/>
					</svg>
					Set Google API Key
				</button>
			</div>
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
		margin: 0;
		padding: 0;
		overflow: hidden;
		font-family: system-ui, -apple-system, sans-serif;
	}

	/* Floating mode */
	.floating-mode {
		background: rgba(17, 24, 39, 0.9);
		backdrop-filter: blur(20px);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 16px;
		box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
	}

	/* Popup mode */
	.popup-mode {
		background: rgba(255, 255, 255, 0.98);
		backdrop-filter: blur(30px);
	}

	/* Floating container */
	.floating-container {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 16px;
		position: relative;
	}

	/* Cursor styles */
	.cursor-grab {
		cursor: grab;
	}

	.cursor-grabbing {
		cursor: grabbing;
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

	/* Hover effect for floating window */
	.floating-container:hover {
		background: rgba(31, 41, 55, 0.95);
		transition: all 0.2s ease;
	}

	/* Input styles */
	input {
		outline: none;
		transition: all 0.2s ease;
		border: 1px solid rgba(255, 255, 255, 0.2);
		background: rgba(255, 255, 255, 0.1);
		color: white;
	}

	input:focus {
		border-color: #3b82f6;
		box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.2);
	}

	/* Popup styles - QUAN TRỌNG: Sửa z-index */
	.fixed {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		z-index: 10000; /* Tăng z-index rất cao */
		display: flex;
		align-items: center;
		justify-content: center;
		animation: fadeIn 0.2s ease-out;
	}

	/* Đảm bảo popup content có z-index cao */
	.fixed > div {
		z-index: 10001;
		position: relative;
	}

	@keyframes fadeIn {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}

	/* Button disabled state */
	button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	/* Context menu styles */
	.context-menu-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		z-index: 9999;
	}

	.context-menu {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		background: rgba(31, 41, 55, 0.95);
		backdrop-filter: blur(20px);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 12px;
		padding: 8px;
		min-width: 200px;
		box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
		opacity: 0;
		visibility: hidden;
		transition: all 0.2s ease;
		z-index: 10000;
	}

	.context-menu.show {
		opacity: 1;
		visibility: visible;
	}

	.context-menu-item {
		display: flex;
		align-items: center;
		padding: 12px 16px;
		border-radius: 8px;
		color: #e5e7eb;
		font-size: 14px;
		cursor: pointer;
		transition: background-color 0.2s ease;
		width: 100%;
		text-align: left;
		border: none;
		background: transparent;
	}

	.context-menu-item:hover {
		background: rgba(59, 130, 246, 0.2);
	}

	.btn {
		cursor: pointer;
		border: none;
		padding: 8px 16px;
		border-radius: 6px;
		font-weight: 500;
		transition: all 0.2s ease;
	}

	.btn:not(:disabled):hover {
		transform: translateY(-1px);
	}

	.btn.variant-filled-surface {
		background: rgba(75, 85, 99, 0.8);
		color: white;
	}

	.btn.variant-filled-success {
		background: rgba(34, 197, 94, 0.8);
		color: white;
	}

	.btn.variant-filled-surface:hover:not(:disabled) {
		background: rgba(75, 85, 99, 1);
	}

	.btn.variant-filled-success:hover:not(:disabled) {
		background: rgba(34, 197, 94, 1);
	}

	.bg-surface-800 {
		background: rgba(31, 41, 55, 0.95);
		backdrop-filter: blur(20px);
		border: 1px solid rgba(255, 255, 255, 0.1);
	}

	.text-surface-200 {
		color: #e5e7eb;
	}

	.text-surface-400 {
		color: #9ca3af;
	}

	.text-surface-500 {
		color: #6b7280;
	}

	.text-surface-600 {
		color: #4b5563;
	}

	.border-surface-600 {
		border-color: #4b5563;
	}

	.bg-surface-700 {
		background: rgba(55, 65, 81, 0.8);
	}

	input::placeholder {
		color: #9ca3af;
	}

	button:focus,
	input:focus {
		outline: 2px solid #3b82f6;
		outline-offset: 2px;
	}
</style>