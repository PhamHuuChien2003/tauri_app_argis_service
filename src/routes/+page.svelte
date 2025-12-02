<script lang="ts">
	import { onMount } from 'svelte';
	import { writable } from 'svelte/store';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
	import { open } from '@tauri-apps/plugin-shell';

	interface ExampleResult {
		status: string;
		address: string;
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
		done?: string;
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
		latitude?: number;
		longitude?: number;
	}

	interface ApiConfig {
		base_url: string;
		opacity: number;
	}

	interface MapPoint {
		id: string;
		lat: number;
		lng: number;
		address?: string;
		timestamp: number;
	}

	// Store để lưu trữ dữ liệu
	const resultData = writable<ExampleResult | null>(null);
	const showConfig = writable<boolean>(false);
	const showUrlInput = writable<boolean>(false);
	const showOpacitySelector = writable<boolean>(false);
	const isProcessing = writable<boolean>(false);
	const apiConfig = writable<ApiConfig>({
		base_url: '',
		opacity: 0.8
	});
	const currentPoint = writable<MapPoint | null>(null);

	let isDragging = false;
	let dragOffset = { x: 0, y: 0 };
	let currentWindow: any;
	let currentPosition = { x: 0, y: 0 };
	let newBaseUrl = '';

	// Tạo ID duy nhất cho điểm
	function generatePointId(): string {
		return `point_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
	}

	function updateContainerOpacity(opacity: number) {
		const container = document.querySelector('.floating-mode') as HTMLElement;
		if (container) {
			container.style.opacity = opacity.toString();
		}
	}

	onMount(() => {
		let unlistenProcessing: (() => void) | undefined;
		let unlistenResult: (() => void) | undefined;
		let unlistenError: (() => void) | undefined;
		let unlistenUrlInput: (() => void) | undefined;
		let unlistenOpacitySelector: (() => void) | undefined;

		async function setupListeners() {
			try {
				currentWindow = getCurrentWebviewWindow();
				
				// Load cấu hình API
				loadApiConfig();
				
				// Lấy vị trí hiện tại của cửa sổ
				updateWindowPosition();
				
				// Lấy trạng thái processing hiện tại
				updateProcessingState();
				
				// Lắng nghe sự kiện từ backend
				unlistenProcessing = await listen('update-processing-state', (event: { payload: boolean }) => {
					console.log('Processing state updated:', event.payload);
					isProcessing.set(event.payload);
				});

				unlistenResult = await listen('update-result', (event: { payload: ExampleResult }) => {
					console.log('Received result update:', event.payload);
					resultData.set(event.payload);
					
					// Chỉ lưu điểm hiện tại, không lưu điểm cũ
					if (event.payload.latitude && event.payload.longitude) {
						currentPoint.set({
							id: generatePointId(),
							lat: event.payload.latitude,
							lng: event.payload.longitude,
							address: event.payload.address,
							timestamp: Date.now()
						});
					}
				});

				unlistenError = await listen('show-error', (event: { payload: string }) => {
					console.log('Received error:', event.payload);
				});

				unlistenUrlInput = await listen('open-url-input', () => {
					console.log('Opening URL input');
					newBaseUrl = $apiConfig.base_url;
					showUrlInput.set(true);
				});

				unlistenOpacitySelector = await listen('open-opacity-selector', () => {
					console.log('Opening opacity selector');
					showOpacitySelector.set(true);
				});
			} catch (error) {
				console.error('Error setting up event listeners:', error);
			}
		}

		async function loadApiConfig() {
			try {
				const config: ApiConfig = await invoke('get_api_config');
				apiConfig.set(config);
				updateContainerOpacity(config.opacity);
			} catch (error) {
				console.error('Failed to load API config:', error);
			}
		}

		async function updateProcessingState() {
			try {
				const processing: boolean = await invoke('get_processing_state');
				isProcessing.set(processing);
			} catch (error) {
				console.error('Failed to get processing state:', error);
				isProcessing.set(false);
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
			if (unlistenProcessing) unlistenProcessing();
			if (unlistenResult) unlistenResult();
			if (unlistenError) unlistenError();
			if (unlistenUrlInput) unlistenUrlInput();
			if (unlistenOpacitySelector) unlistenOpacitySelector();
		};
	});

	// Hàm bắt đầu kéo cửa sổ
	function startDrag(event: MouseEvent) {
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

	// Cập nhật Base URL
	async function updateBaseUrl() {
		try {
			const newConfig: ApiConfig = {
				...$apiConfig,
				base_url: newBaseUrl
			};
			await invoke('update_api_config', { newConfig });
			apiConfig.set(newConfig);
			showUrlInput.set(false);
			newBaseUrl = '';
		} catch (error) {
			console.error('Failed to update base URL:', error);
		}
	}

	// Cập nhật opacity
	async function updateOpacity(newOpacity: number) {
		try {
			const newConfig: ApiConfig = {
				...$apiConfig,
				opacity: newOpacity
			};
			await invoke('update_api_config', { newConfig });
			apiConfig.set(newConfig);
			updateContainerOpacity(newOpacity);
			showOpacitySelector.set(false);
		} catch (error) {
			console.error('Failed to update opacity:', error);
		}
	}

	// Mở Google Maps trong webview
	async function openGoogleMapsWebview(point: MapPoint) {
		try {
			await invoke('open_map_view', {
				lat: point.lat,
				lng: point.lng,
				mapType: 'google',
				pointId: point.id
			});
		} catch (error) {
			console.error('Failed to open Google Maps webview:', error);
			await openGoogleMapsExternal(point);
		}
	}

	// Mở OpenStreetMap trong webview
	async function openOpenStreetMapWebview(point: MapPoint) {
		try {
			await invoke('open_map_view', {
				lat: point.lat,
				lng: point.lng,
				mapType: 'openstreetmap',
				pointId: point.id
			});
		} catch (error) {
			console.error('Failed to open OpenStreetMap webview:', error);
			await openOpenStreetMapExternal(point);
		}
	}

	// Mở cả hai map cùng lúc
	async function openBothMaps(point: MapPoint) {
		try {
			await invoke('open_multiple_map_views', {
				lat: point.lat,
				lng: point.lng,
				pointId: point.id
			});
		} catch (error) {
			console.error('Failed to open both maps:', error);
			// Fallback: mở từng cái một
			await openGoogleMapsWebview(point);
			await openOpenStreetMapWebview(point);
		}
	}

	// Mở Google Maps trong trình duyệt external
	async function openGoogleMapsExternal(point: MapPoint) {
		const url = `https://www.google.com/maps?q=${point.lat},${point.lng}`;
		try {
			await open(url);
		} catch (error) {
			console.error('Failed to open Google Maps:', error);
		}
	}

	// Mở OpenStreetMap trong trình duyệt external
	async function openOpenStreetMapExternal(point: MapPoint) {
		const url = `https://www.openstreetmap.org/?mlat=${point.lat}&mlon=${point.lng}&zoom=17`;
		try {
			await open(url);
		} catch (error) {
			console.error('Failed to open OpenStreetMap:', error);
		}
	}
</script>

<div class="container h-full mx-auto" class:floating-mode={true} style:opacity={$apiConfig.opacity}>
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
				{#if $isProcessing}
					<!-- Hiển thị khi đang xử lý -->
					<div class="w-10 h-10 mx-auto mb-1 bg-yellow-500 rounded-full flex items-center justify-center shadow-lg animate-pulse">
						<svg viewBox="0 0 24 24" fill="white" class="w-6 h-6">
							<path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z"/>
						</svg>
					</div>
					<p class="text-[10px] text-yellow-400 font-medium">Processing</p>
				{:else if $currentPoint}
					<!-- Hiển thị 2 nút khi có kết quả -->
					<div class="flex space-x-1 mb-1">
						<button
							class="w-20 h-20 bg-blue-500 rounded-full flex items-center justify-center shadow hover:bg-blue-600 transition-colors"
							on:click={() => openGoogleMapsWebview($currentPoint)}
							title="Open Google Maps"
						>
							<svg viewBox="0 0 24 24" fill="white" class="w-15 h-15">
								<path d="M12 2C8.13 2 5 5.13 5 9c0 5.25 7 13 7 13s7-7.75 7-13c0-3.87-3.13-7-7-7zm0 9.5c-1.38 0-2.5-1.12-2.5-2.5s1.12-2.5 2.5-2.5 2.5 1.12 2.5 2.5-1.12 2.5-2.5 2.5z"/>
							</svg>
						</button>
						<button
							class="w-20 h-20 bg-green-500 rounded-full flex items-center justify-center shadow hover:bg-green-600 transition-colors"
							on:click={() => openOpenStreetMapWebview($currentPoint)}
							title="Open OpenStreetMap"
						>
							<svg viewBox="0 0 24 24" fill="white" class="w-15 h-15">
								<path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z"/>
							</svg>
						</button>
					</div>
					<p class="text-[8px] text-green-400 font-medium">Ready</p>
				{:else}
					<!-- Trạng thái bình thường -->
					<div class="w-10 h-10 mx-auto mb-1 bg-primary-500 rounded-full flex items-center justify-center shadow-lg">
						<svg viewBox="0 0 24 24" fill="white" class="w-6 h-6">
							<path d="M12 2C8.13 2 5 5.13 5 9c0 5.25 7 13 7 13s7-7.75 7-13c0-3.87-3.13-7-7-7zm0 9.5c-1.38 0-2.5-1.12-2.5-2.5s1.12-2.5 2.5-2.5 2.5 1.12 2.5 2.5-1.12 2.5-2.5 2.5z"/>
						</svg>
					</div>
					<p class="text-[10px] text-surface-400 font-medium">Geocoder</p>
					<p class="text-[8px] text-surface-500">{$apiConfig.base_url ? 'Base URL set' : 'No URL'}</p>
				{/if}
				<p class="text-[7px] text-surface-400 mt-1">Drag to move</p>
			</div>
		</div>
	</div>

	<!-- Custom URL Input Popup -->
	{#if $showUrlInput}
		<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
			<div class="bg-surface-800 rounded-lg p-6 w-96">
				<h3 class="text-lg font-semibold text-surface-200 mb-4">Set Base URL</h3>
				<div class="space-y-4">
					<div>
						<label for="base-url-input" class="block text-sm font-medium text-surface-400 mb-2">Base URL</label>
						<input
							id="base-url-input"
							type="text"
							class="w-full bg-surface-700 border border-surface-600 rounded px-3 py-2 text-surface-200"
							placeholder="http://my-domain-ip:my-port"
							bind:value={newBaseUrl}
						/>
						<p class="text-xs text-surface-500 mt-1">
							Chỉ nhập base URL. Ứng dụng sẽ tự động gọi các endpoint:
						</p>
						<ul class="text-xs text-surface-500 mt-1 space-y-1">
							<!-- <li>• <code class="bg-surface-700 px-1 rounded">/geocode?latlng={lat},{lng}</code> - Lấy thông tin địa chỉ</li>
							<li>• <code class="bg-surface-700 px-1 rounded">/placedetails?place_id={place_id}</code> - Lấy chi tiết địa điểm</li> -->
						</ul>
					</div>
				</div>
				<div class="flex justify-end space-x-3 mt-6">
					<button
						class="btn variant-filled-surface px-4"
						on:click={() => {
							showUrlInput.set(false);
							newBaseUrl = '';
						}}
					>
						Cancel
					</button>
					<button
						class="btn variant-filled-success px-4"
						on:click={updateBaseUrl}
						disabled={!newBaseUrl.trim()}
					>
						Save
					</button>
				</div>
			</div>
		</div>
	{/if}

	<!-- Opacity Selector Popup -->
	{#if $showOpacitySelector}
		<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
			<div class="bg-surface-800 rounded-lg p-6 w-80">
				<h3 class="text-lg font-semibold text-surface-200 mb-4">Set Opacity</h3>
				<div class="space-y-3">
					<button
						class="w-full p-3 text-left rounded border {$apiConfig.opacity === 0.2 ? 'border-primary-500 bg-primary-500 bg-opacity-20' : 'border-surface-600 hover:border-surface-500'}"
						on:click={() => updateOpacity(0.2)}
					>
						<div class="font-medium text-surface-200">20% - Rất trong</div>
					</button>
					
					<button
						class="w-full p-3 text-left rounded border {$apiConfig.opacity === 0.5 ? 'border-primary-500 bg-primary-500 bg-opacity-20' : 'border-surface-600 hover:border-surface-500'}"
						on:click={() => updateOpacity(0.5)}
					>
						<div class="font-medium text-surface-200">50% - Trong</div>
					</button>
					
					<button
						class="w-full p-3 text-left rounded border {$apiConfig.opacity === 0.75 ? 'border-primary-500 bg-primary-500 bg-opacity-20' : 'border-surface-600 hover:border-surface-500'}"
						on:click={() => updateOpacity(0.75)}
					>
						<div class="font-medium text-surface-200">75% - Hơi trong</div>
					</button>
					
					<button
						class="w-full p-3 text-left rounded border {$apiConfig.opacity === 1 ? 'border-primary-500 bg-primary-500 bg-opacity-20' : 'border-surface-600 hover:border-surface-500'}"
						on:click={() => updateOpacity(1)}
					>
						<div class="font-medium text-surface-200">100% - Đầy đủ</div>
					</button>
				</div>
				<div class="flex justify-end mt-6">
					<button
						class="btn variant-filled-surface px-4"
						on:click={() => showOpacitySelector.set(false)}
					>
						Cancel
					</button>
				</div>
			</div>
		</div>
	{/if}

	<!-- System Tray Context Menu -->
	{#if $showConfig}
		<div class="context-menu-overlay" role="presentation" on:click|self={() => showConfig.set(false)}>
			<div class="context-menu" class:show={$showConfig}>
				<button class="context-menu-item" on:click={() => {
					newBaseUrl = $apiConfig.base_url;
					showUrlInput.set(true);
					showConfig.set(false);
				}}>
					<svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"/>
					</svg>
					Set Base URL
				</button>
				<button class="context-menu-item" on:click={() => {
					showOpacitySelector.set(true);
					showConfig.set(false);
				}}>
					<svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 100 4m0-4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 100 4m0-4v2m0-6V4"/>
					</svg>
					Set Opacity
				</button>
			</div>
		</div>
	{/if}
</div>

<style lang="postcss">
	.container {
		min-height: 100vh;
		background: transparent !important;
		transition: opacity 0.3s ease;
	}

	:global(body) {
		background: transparent !important;
		margin: 0;
		padding: 0;
		overflow: hidden;
		font-family: system-ui, -apple-system, sans-serif;
	}

	/* Floating mode - Tạo trong suốt giống Discord Gaming */
	.floating-mode {
		background: rgba(0, 0, 0, 0.5) !important; /* Giảm độ mờ */
		backdrop-filter: blur(10px) !important; /* Giảm blur để tạo trong suốt */
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 12px;
		box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
	}

	/* Floating container */
	.floating-container {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 12px;
		position: relative;
	}

	/* Cursor styles */
	.cursor-grab {
		cursor: grab;
	}

	.cursor-grabbing {
		cursor: grabbing;
	}

	/* Animation for processing */
	.animate-pulse {
		animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
	}

	@keyframes pulse {
		0%, 100% {
			opacity: 1;
		}
		50% {
			opacity: 0.7;
		}
	}

	/* Hover effect for floating window */
	.floating-container:hover {
		background: rgba(0, 0, 0, 0.6) !important;
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

	/* Popup styles */
	.fixed {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		z-index: 10000;
		display: flex;
		align-items: center;
		justify-content: center;
		animation: fadeIn 0.2s ease-out;
	}

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
		background: rgba(0, 0, 0, 0.8);
		backdrop-filter: blur(10px);
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
		background: rgba(0, 0, 0, 0.85) !important;
		backdrop-filter: blur(10px);
		border: 1px solid rgba(255, 255, 255, 0.1);
	}

	.text-surface-200 {
		color: #e5e7eb;
	}

	.text-surface-300 {
		color: #d1d5db;
	}

	.text-surface-400 {
		color: #9ca3af;
	}

	.text-surface-500 {
		color: #6b7280;
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

	/* Code style for URL template hints */
	code {
		font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
		font-size: 0.75rem;
	}

	/* Map selector button hover effects */
	button:hover .text-surface-200 {
		color: white;
	}

	button:hover .text-surface-500 {
		color: #d1d5db;
	}

	/* Style cho current point */
	.bg-green-500\.bg-opacity-10 {
		background-color: rgba(34, 197, 94, 0.1);
	}

	.bg-blue-500\.bg-opacity-20 {
		background-color: rgba(59, 130, 246, 0.2);
	}

	.bg-green-500\.bg-opacity-20 {
		background-color: rgba(34, 197, 94, 0.2);
	}
</style>