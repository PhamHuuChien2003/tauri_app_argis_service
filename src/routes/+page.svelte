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

	interface MapConfig {
		google: boolean;
		openstreetmap: boolean;
		bing: boolean;
		streetviewvn: boolean;
		mapillary: boolean;
		vietbando: boolean;
		herewego: boolean;
		wikimapia: boolean;
	}

	interface ApiConfig {
		base_url: string;
		opacity: number;
		maps: MapConfig;
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
		base_url: '',
		opacity: 0.8,
		maps: {
			google: true,
			openstreetmap: true,
			bing: false,
			streetviewvn: false,
			mapillary: false,
			vietbando: false,
			herewego: false,
			wikimapia: false,
		}
	});
	const currentPoint = writable<MapPoint | null>(null);

	let isDragging = false;
	let dragOffset = { x: 0, y: 0 };
	let currentWindow: any;
	let currentPosition = { x: 0, y: 0 };
	let newBaseUrl = '';

	// Định nghĩa các loại bản đồ với thông tin chi tiết
	const mapTypes = [
		{ id: 'google', name: 'Google Maps', icon: '🗺️', color: 'bg-red-500', default: true },
		{ id: 'openstreetmap', name: 'OpenStreetMap', icon: '🌍', color: 'bg-green-500', default: true },
		{ id: 'bing', name: 'Bing Maps', icon: '🅱️', color: 'bg-blue-500', default: false },
		{ id: 'streetviewvn', name: 'StreetView.vn', icon: '👁️', color: 'bg-purple-500', default: false },
		{ id: 'mapillary', name: 'Mapillary', icon: '📷', color: 'bg-yellow-500', default: false },
		{ id: 'vietbando', name: 'Vietbando', icon: '🇻🇳', color: 'bg-red-600', default: false },
		{ id: 'herewego', name: 'Here WeGo', icon: '📍', color: 'bg-blue-600', default: false },
		{ id: 'wikimapia', name: 'Wikimapia', icon: '📖', color: 'bg-gray-500', default: false },
	];

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

	function getMapEnabled(mapId: string): boolean {
		return $apiConfig.maps[mapId as keyof MapConfig];
	}

	function toggleMap(mapId: string) {
		apiConfig.update(config => {
			const newMaps = { ...config.maps };
			newMaps[mapId as keyof MapConfig] = !newMaps[mapId as keyof MapConfig];
			return { ...config, maps: newMaps };
		});
	}

	onMount(() => {
		let unlistenProcessing: (() => void) | undefined;
		let unlistenResult: (() => void) | undefined;
		let unlistenError: (() => void) | undefined;
		let unlistenUrlInput: (() => void) | undefined;
		let unlistenOpacitySelector: (() => void) | undefined;
		let unlistenMapSelector: (() => void) | undefined;

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

				unlistenMapSelector = await listen('open-map-selector', () => {
					console.log('Opening map selector');
					showMapSelector.set(true);
				});
			} catch (error) {
				console.error('Error setting up event listeners:', error);
			}
		}

		async function loadApiConfig() {
			try {
				const config: ApiConfig = await invoke('get_api_config');
				// Đảm bảo cấu hình maps có đầy đủ các trường (nếu config cũ không có maps)
				if (!config.maps) {
					config.maps = {
						google: true,
						openstreetmap: true,
						bing: false,
						streetviewvn: false,
						mapillary: false,
						vietbando: false,
						herewego: false,
						wikimapia: false,
					};
				}
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
			if (unlistenMapSelector) unlistenMapSelector();
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

	// Hàm lấy URL bản đồ theo loại
	function getMapUrl(mapType: string, lat: number, lng: number): string {
		switch (mapType) {
			case 'google': return `https://www.google.com/maps?q=${lat},${lng}`;
			case 'openstreetmap': return `https://www.openstreetmap.org/?mlat=${lat}&mlon=${lng}&zoom=17`;
			case 'bing': return `https://www.bing.com/maps?cp=${lat}~${lng}&lvl=11&style=r`;
			case 'streetviewvn': return `https://www.streetview.vn/?lat=${lat}&lng=${lng}`;
			case 'mapillary': return `https://www.mapillary.com/app/?lat=${lat}&lng=${lng}&z=16.53889080546365&menu=false`;
			case 'vietbando': return 'http://maps.vietbando.com/maps/';
			case 'herewego': return `https://wego.here.com/?map=${lat},${lng},10`;
			case 'wikimapia': return `https://wikimapia.org/#lang=en&lat=${lat}&lon=${lng}&z=12&m=w`;
			default: return `https://www.google.com/maps?q=${lat},${lng}`;
		}
	}

	// Mở bản đồ theo loại
	async function openMap(point: MapPoint, mapType: string) {
		try {
			await invoke('open_map_view', {
				lat: point.lat,
				lng: point.lng,
				mapType: mapType,
				pointId: point.id
			});
		} catch (error) {
			console.error(`Failed to open ${mapType}:`, error);
			// Mở bằng trình duyệt external
			const url = getMapUrl(mapType, point.lat, point.lng);
			await open(url);
		}
	}

	// Mở tất cả bản đồ được chọn
	async function openAllSelectedMaps(point: MapPoint) {
		try {
			await invoke('open_selected_maps', {
				lat: point.lat,
				lng: point.lng,
				pointId: point.id,
				mapConfig: $apiConfig.maps
			});
		} catch (error) {
			console.error('Failed to open selected maps:', error);
			// Fallback: mở từng cái một
			for (const mapType of mapTypes) {
				if ($apiConfig.maps[mapType.id as keyof MapConfig]) {
					await openMap(point, mapType.id);
				}
			}
		}
	}

	// Đếm số bản đồ được chọn
	function countSelectedMaps(): number {
		return Object.values($apiConfig.maps).filter(v => v).length;
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
					<!-- Hiển thị các nút bản đồ được chọn -->
					<div class="grid grid-cols-2 gap-1 mb-1">
						{#each mapTypes as map}
							{#if getMapEnabled(map.id)}
								<button
									class="w-15 h-15 {map.color} rounded-full flex flex-col items-center justify-center shadow hover:opacity-90 transition-all text-white p-1"
									on:click={() => openMap($currentPoint, map.id)}
									title={map.name}
								>
									<div class="text-lg mb-0.5">{map.icon}</div>
									<div class="text-[8px] font-medium leading-tight text-center">{map.name.split(' ')[0]}</div>
								</button>
							{/if}
						{/each}
						
						<!-- Nút mở tất cả (nếu có nhiều hơn 1 bản đồ được chọn) -->
						{#if countSelectedMaps() > 1}
							<button
								class="w-15 h-15 bg-gradient-to-br from-purple-500 to-pink-500 rounded-full flex flex-col items-center justify-center shadow hover:opacity-90 transition-all text-white p-1"
								on:click={() => openAllSelectedMaps($currentPoint)}
								title="Open All Selected Maps"
							>
								<div class="text-lg mb-0.5">🚀</div>
								<div class="text-[8px] font-medium leading-tight text-center">All</div>
							</button>
						{/if}
					</div>
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
							<li>• <code class="bg-surface-700 px-1 rounded">/geocode?latlng&#123;lat&#125,&#123;lng&#125</code></li>
							<li>• <code class="bg-surface-700 px-1 rounded">/placedetails?place_id=&#123;place_id&#125</code></li>
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
						<div class="font-medium text-surface-200">20%</div>
					</button>
					
					<button
						class="w-full p-3 text-left rounded border {$apiConfig.opacity === 0.5 ? 'border-primary-500 bg-primary-500 bg-opacity-20' : 'border-surface-600 hover:border-surface-500'}"
						on:click={() => updateOpacity(0.5)}
					>
						<div class="font-medium text-surface-200">50%</div>
					</button>
					
					<button
						class="w-full p-3 text-left rounded border {$apiConfig.opacity === 0.75 ? 'border-primary-500 bg-primary-500 bg-opacity-20' : 'border-surface-600 hover:border-surface-500'}"
						on:click={() => updateOpacity(0.75)}
					>
						<div class="font-medium text-surface-200">75%</div>
					</button>
					
					<button
						class="w-full p-3 text-left rounded border {$apiConfig.opacity === 1 ? 'border-primary-500 bg-primary-500 bg-opacity-20' : 'border-surface-600 hover:border-surface-500'}"
						on:click={() => updateOpacity(1)}
					>
						<div class="font-medium text-surface-200">100%</div>
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

	<!-- Map Selector Popup -->
	{#if $showMapSelector}
		<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
			<div class="bg-surface-800 rounded-lg p-6 w-96 max-h-96 overflow-y-auto">
				<h3 class="text-lg font-semibold text-surface-200 mb-4">Select Maps to Display</h3>
				<div class="space-y-2">
					{#each mapTypes as map}
						<label class="flex items-center justify-between p-3 rounded border border-surface-600 hover:border-surface-500 transition-colors cursor-pointer">
							<div class="flex items-center space-x-3">
								<div class="text-2xl">{map.icon}</div>
								<div>
									<div class="font-medium text-surface-200">{map.name}</div>
									<!-- <div class="text-xs text-surface-500 truncate max-w-xs">{getMapUrl(map.id, 0, 0)}</div> -->
								</div>
							</div>
							<input
								type="checkbox"
								class="w-5 h-5 rounded border-surface-600 bg-surface-700 text-primary-500 focus:ring-primary-500"
								checked={getMapEnabled(map.id)}
								on:change={() => toggleMap(map.id)}
							/>
						</label>
					{/each}
				</div>
				<div class="flex justify-end space-x-3 mt-6">
					<button
						class="btn variant-filled-surface px-4"
						on:click={() => showMapSelector.set(false)}
					>
						Cancel
					</button>
					<button
						class="btn variant-filled-success px-4"
						on:click={async () => {
							try {
								await invoke('update_api_config', { newConfig: $apiConfig });
								showMapSelector.set(false);
							} catch (error) {
								console.error('Failed to update map selection:', error);
							}
						}}
					>
						Save
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
				<button class="context-menu-item" on:click={() => {
					showMapSelector.set(true);
					showConfig.set(false);
				}}>
					<svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 20l-5.447-2.724A1 1 0 013 16.382V5.618a1 1 0 011.447-.894L9 7m0 13l6-3m-6 3V7m6 10l4.553 2.276A1 1 0 0021 18.382V7.618a1 1 0 00-.553-.894L15 4m0 13V4m0 0L9 7"/>
					</svg>
					Select Maps
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
		background: rgba(0, 0, 0, 0.5) !important;
		backdrop-filter: blur(10px) !important;
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

	/* Grid styles for map buttons */
	.grid-cols-2 {
		grid-template-columns: repeat(2, minmax(0, 1fr));
	}

	.gap-1 {
		gap: 0.25rem;
	}

	.w-15 {
		width: 3.75rem;
	}

	.h-15 {
		height: 3.75rem;
	}

	.max-h-96 {
		max-height: 24rem;
	}

	.overflow-y-auto {
		overflow-y: auto;
	}

	.max-w-xs {
		max-width: 20rem;
	}

	.truncate {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.from-purple-500 {
		--tw-gradient-from: #a855f7;
		--tw-gradient-stops: var(--tw-gradient-from), var(--tw-gradient-to, rgba(168, 85, 247, 0));
	}

	.to-pink-500 {
		--tw-gradient-to: #ec4899;
	}

	/* Hiệu ứng hover cho nút bản đồ */
	button:hover .text-lg {
		transform: scale(1.1);
		transition: transform 0.2s ease;
	}
</style>