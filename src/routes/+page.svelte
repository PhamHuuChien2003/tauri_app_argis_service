<script lang="ts">
	import { onMount } from 'svelte';
	import { writable } from 'svelte/store';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
	import { open } from '@tauri-apps/plugin-shell';

	let lat: number = 0;
	let lng: number = 0;

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
		latitude?: number;
		longitude?: number;
	}

	interface ApiConfig {
		custom_url: string;
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
	const showMapSelector = writable<boolean>(false);
	const isProcessing = writable<boolean>(false);
	const apiConfig = writable<ApiConfig>({
		custom_url: '',
		opacity: 0.8
	});
	const mapPoints = writable<MapPoint[]>([]);
	const currentPointId = writable<string>('');
	const autoShowMapSelector = writable<boolean>(false);

	let isDragging = false;
	let dragOffset = { x: 0, y: 0 };
	let currentWindow: any;
	let currentPosition = { x: 0, y: 0 };
	let newCustomUrl = '';

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
					
					// Thêm điểm mới vào danh sách khi có kết quả
					if (event.payload.latitude && event.payload.longitude) {
						const newPoint: MapPoint = {
							id: generatePointId(),
							lat: event.payload.latitude,
							lng: event.payload.longitude,
							address: event.payload.address,
							timestamp: Date.now()
						};
						
						mapPoints.update(points => {
							// Giới hạn số lượng điểm để tránh tràn bộ nhớ
							const newPoints = [newPoint, ...points].slice(0, 50);
							return newPoints;
						});
						
						currentPointId.set(newPoint.id);
						
						// Chỉ tự động hiển thị map selector cho điểm đầu tiên hoặc khi chưa có popup nào mở
						if ((event.payload.status === 'success' || event.payload.status === 'OK') && !$showMapSelector) {
							autoShowMapSelector.set(true);
							showMapSelector.set(true);
						}
					}
				});

				unlistenError = await listen('show-error', (event: { payload: string }) => {
					console.log('Received error:', event.payload);
				});

				unlistenUrlInput = await listen('open-url-input', () => {
					console.log('Opening URL input');
					newCustomUrl = $apiConfig.custom_url;
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

	// Cập nhật Custom URL
	async function updateCustomUrl() {
		try {
			const newConfig: ApiConfig = {
				...$apiConfig,
				custom_url: newCustomUrl
			};
			await invoke('update_api_config', { newConfig });
			apiConfig.set(newConfig);
			showUrlInput.set(false);
			newCustomUrl = '';
		} catch (error) {
			console.error('Failed to update custom URL:', error);
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

	// Định dạng thời gian
	function formatTime(timestamp: number): string {
		return new Date(timestamp).toLocaleTimeString();
	}

	// Lấy điểm hiện tại (mới nhất)
	$: currentPoint = $mapPoints[0];
	
	// Lấy các điểm cũ (trừ điểm hiện tại)
	$: previousPoints = $mapPoints.slice(1);
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
				{:else if $mapPoints.length > 0}
					<!-- Hiển thị map options khi có kết quả -->
					<button
						class="w-10 h-10 mx-auto mb-1 bg-green-500 rounded-full flex items-center justify-center shadow-lg hover:bg-green-600 transition-colors"
						on:click={() => showMapSelector.set(true)}
						title="Open Map View"
					>
						<svg viewBox="0 0 24 24" fill="white" class="w-6 h-6">
							<path d="M12 2C8.13 2 5 5.13 5 9c0 5.25 7 13 7 13s7-7.75 7-13c0-3.87-3.13-7-7-7zm0 9.5c-1.38 0-2.5-1.12-2.5-2.5s1.12-2.5 2.5-2.5 2.5 1.12 2.5 2.5-1.12 2.5-2.5 2.5z"/>
						</svg>
					</button>
					<p class="text-[10px] text-green-400 font-medium">Ready</p>
					<p class="text-[8px] text-surface-500">{$mapPoints.length} points</p>
				{:else}
					<!-- Trạng thái bình thường -->
					<div class="w-10 h-10 mx-auto mb-1 bg-primary-500 rounded-full flex items-center justify-center shadow-lg">
						<svg viewBox="0 0 24 24" fill="white" class="w-6 h-6">
							<path d="M12 2C8.13 2 5 5.13 5 9c0 5.25 7 13 7 13s7-7.75 7-13c0-3.87-3.13-7-7-7zm0 9.5c-1.38 0-2.5-1.12-2.5-2.5s1.12-2.5 2.5-2.5 2.5 1.12 2.5 2.5-1.12 2.5-2.5 2.5z"/>
						</svg>
					</div>
					<p class="text-[10px] text-surface-400 font-medium">Geocoder</p>
					<p class="text-[8px] text-surface-500">{$apiConfig.custom_url ? 'Custom URL' : 'No URL'}</p>
				{/if}
				<p class="text-[7px] text-surface-400 mt-1">Drag to move</p>
			</div>
		</div>
	</div>

	<!-- Map Selector Popup -->
	{#if $showMapSelector && $mapPoints.length > 0}
		<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
			<div class="bg-surface-800 rounded-lg p-6 w-96 max-w-sm max-h-[80vh] overflow-y-auto">
				<div class="flex justify-between items-center mb-4">
					<h3 class="text-lg font-semibold text-surface-200">Map Views</h3>
					<button
						class="text-surface-400 hover:text-surface-200 transition-colors"
						on:click={() => showMapSelector.set(false)}
					>
						<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
						</svg>
					</button>
				</div>

				<!-- Current Point (luôn hiển thị đầu tiên) -->
				{#if currentPoint}
					<div class="mb-6">
						<div class="flex items-center mb-3">
							<div class="w-3 h-3 bg-green-500 rounded-full mr-2 animate-pulse"></div>
							<h4 class="font-semibold text-green-400 text-lg">Current Point</h4>
						</div>
						
						<div class="p-3 border-2 border-green-500 rounded-lg bg-green-500 bg-opacity-10">
							<div class="flex justify-between items-start mb-3">
								<div>
									<p class="text-sm text-surface-200 font-medium">
										{currentPoint.lat.toFixed(6)}, {currentPoint.lng.toFixed(6)}
									</p>
									<p class="text-xs text-surface-400">
										{formatTime(currentPoint.timestamp)}
									</p>
								</div>
								<div class="text-right">
									<span class="inline-block px-2 py-1 text-xs bg-green-500 text-white rounded">
										NEW
									</span>
								</div>
							</div>

							<!-- Quick Actions for Current Point -->
							<div class="grid grid-cols-3 gap-2 mb-3">
								<button
									class="p-2 bg-blue-500 hover:bg-blue-600 rounded text-white text-xs transition-colors"
									on:click={() => openGoogleMapsWebview(currentPoint)}
									title="Open Google Maps"
								>
									Google
								</button>
								<button
									class="p-2 bg-green-500 hover:bg-green-600 rounded text-white text-xs transition-colors"
									on:click={() => openOpenStreetMapWebview(currentPoint)}
									title="Open OpenStreetMap"
								>
									OSM
								</button>
								<button
									class="p-2 bg-purple-500 hover:bg-purple-600 rounded text-white text-xs transition-colors"
									on:click={() => openBothMaps(currentPoint)}
									title="Open Both Maps"
								>
									Both
								</button>
							</div>

							<!-- Detailed Options for Current Point -->
							<div class="space-y-2">
								<!-- Google Maps Options -->
								<div class="flex space-x-2">
									<button
										class="flex-1 p-2 text-left rounded border border-blue-500 hover:border-blue-400 hover:bg-blue-500 hover:bg-opacity-20 transition-all text-xs"
										on:click={() => openGoogleMapsWebview(currentPoint)}
									>
										<div class="font-medium text-surface-200">Google Maps</div>
										<div class="text-blue-400">Webview</div>
									</button>
									
									<button
										class="flex-1 p-2 text-left rounded border border-blue-400 hover:border-blue-300 hover:bg-blue-400 hover:bg-opacity-20 transition-all text-xs"
										on:click={() => openGoogleMapsExternal(currentPoint)}
									>
										<div class="font-medium text-surface-200">Google Maps</div>
										<div class="text-blue-300">External</div>
									</button>
								</div>

								<!-- OpenStreetMap Options -->
								<div class="flex space-x-2">
									<button
										class="flex-1 p-2 text-left rounded border border-green-500 hover:border-green-400 hover:bg-green-500 hover:bg-opacity-20 transition-all text-xs"
										on:click={() => openOpenStreetMapWebview(currentPoint)}
									>
										<div class="font-medium text-surface-200">OpenStreetMap</div>
										<div class="text-green-400">Webview</div>
									</button>
									
									<button
										class="flex-1 p-2 text-left rounded border border-green-400 hover:border-green-300 hover:bg-green-400 hover:bg-opacity-20 transition-all text-xs"
										on:click={() => openOpenStreetMapExternal(currentPoint)}
									>
										<div class="font-medium text-surface-200">OpenStreetMap</div>
										<div class="text-green-300">External</div>
									</button>
								</div>
							</div>

							<!-- Point Address (nếu có) -->
							{#if currentPoint.address}
								<div class="mt-2 p-2 bg-surface-700 rounded text-xs text-surface-400">
									{currentPoint.address}
								</div>
							{/if}
						</div>
					</div>
				{/if}

				<!-- Previous Points (chỉ hiển thị nếu có) -->
				{#if previousPoints.length > 0}
					<div class="mb-4">
						<div class="flex items-center mb-3">
							<svg class="w-4 h-4 text-surface-400 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
							</svg>
							<h4 class="font-semibold text-surface-400">Previous Points ({previousPoints.length})</h4>
						</div>

						<div class="space-y-3 max-h-60 overflow-y-auto">
							{#each previousPoints as point, index}
								<div class="p-3 border border-surface-600 rounded-lg bg-surface-700 bg-opacity-50">
									<div class="flex justify-between items-start mb-2">
										<div>
											<h5 class="font-medium text-surface-300 text-sm">
												Point {index + 2} <!-- Bắt đầu từ 2 vì point 1 là current -->
											</h5>
											<p class="text-xs text-surface-500">
												{point.lat.toFixed(6)}, {point.lng.toFixed(6)}
											</p>
											<p class="text-xs text-surface-600">
												{formatTime(point.timestamp)}
											</p>
										</div>
									</div>

									<!-- Compact Actions for Previous Points -->
									<div class="grid grid-cols-2 gap-1">
										<button
											class="p-1 bg-blue-500 hover:bg-blue-600 rounded text-white text-xs transition-colors"
											on:click={() => openGoogleMapsWebview(point)}
											title="Open Google Maps"
										>
											Google
										</button>
										<button
											class="p-1 bg-green-500 hover:bg-green-600 rounded text-white text-xs transition-colors"
											on:click={() => openOpenStreetMapWebview(point)}
											title="Open OpenStreetMap"
										>
											OSM
										</button>
									</div>
								</div>
							{/each}
						</div>
					</div>
				{/if}

				<!-- Global Actions -->
				<div class="mt-4 pt-4 border-t border-surface-600">
					<h4 class="font-medium text-surface-300 mb-2">Global Actions</h4>
					<div class="grid grid-cols-2 gap-2">
						<button
							class="p-2 bg-blue-500 hover:bg-blue-600 rounded text-white text-xs transition-colors"
							on:click={() => {
								// Chỉ mở current point trong Google Maps
								if (currentPoint) {
									openGoogleMapsWebview(currentPoint);
								}
							}}
						>
							Open Current Google
						</button>
						<button
							class="p-2 bg-green-500 hover:bg-green-600 rounded text-white text-xs transition-colors"
							on:click={() => {
								// Chỉ mở current point trong OSM
								if (currentPoint) {
									openOpenStreetMapWebview(currentPoint);
								}
							}}
						>
							Open Current OSM
						</button>
					</div>
					<div class="mt-2">
						<button
							class="w-full p-2 bg-red-500 hover:bg-red-600 rounded text-white text-xs transition-colors"
							on:click={() => {
								mapPoints.set([]);
								currentPointId.set('');
							}}
						>
							Clear All Points
						</button>
					</div>
				</div>
			</div>
		</div>
	{/if}

	<!-- Custom URL Input Popup -->
	{#if $showUrlInput}
		<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
			<div class="bg-surface-800 rounded-lg p-6 w-96">
				<h3 class="text-lg font-semibold text-surface-200 mb-4">Set Custom URL</h3>
				<div class="space-y-4">
					<div>
						<label for="custom-url-input" class="block text-sm font-medium text-surface-400 mb-2">Custom URL Template</label>
						<input
							id="custom-url-input"
							type="text"
							class="w-full bg-surface-700 border border-surface-600 rounded px-3 py-2 text-surface-200"
							placeholder="http://my-domain-ip:my-port/geocode?latlng={lat},{lng}"
							bind:value={newCustomUrl}
						/>
						<p class="text-xs text-surface-500 mt-1">
							Sử dụng <code class="bg-surface-700 px-1 rounded">{lat}</code> và <code class="bg-surface-700 px-1 rounded">{lng}</code> hoặc <code class="bg-surface-700 px-1 rounded">{lng}</code> làm placeholder
						</p>
					</div>
				</div>
				<div class="flex justify-end space-x-3 mt-6">
					<button
						class="btn variant-filled-surface px-4"
						on:click={() => {
							showUrlInput.set(false);
							newCustomUrl = '';
						}}
					>
						Cancel
					</button>
					<button
						class="btn variant-filled-success px-4"
						on:click={updateCustomUrl}
						disabled={!newCustomUrl.trim()}
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
					newCustomUrl = $apiConfig.custom_url;
					showUrlInput.set(true);
					showConfig.set(false);
				}}>
					<svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"/>
					</svg>
					Set Custom URL
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
		background: transparent;
		transition: opacity 0.3s ease;
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

	/* Scrollbar styling for map selector */
	.max-h-\[80vh\] {
		max-height: 80vh;
	}

	.max-h-60 {
		max-height: 15rem;
	}

	.overflow-y-auto {
		overflow-y: auto;
	}

	.overflow-y-auto::-webkit-scrollbar {
		width: 6px;
	}

	.overflow-y-auto::-webkit-scrollbar-track {
		background: rgba(255, 255, 255, 0.1);
		border-radius: 3px;
	}

	.overflow-y-auto::-webkit-scrollbar-thumb {
		background: rgba(255, 255, 255, 0.3);
		border-radius: 3px;
	}

	.overflow-y-auto::-webkit-scrollbar-thumb:hover {
		background: rgba(255, 255, 255, 0.5);
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