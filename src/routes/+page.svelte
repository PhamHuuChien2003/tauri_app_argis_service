<script>
	import { onMount } from 'svelte';
	import { writable } from 'svelte/store';
	import { listen } from '@tauri-apps/api/event';
	import DetailItem from './DetailItem.svelte';

	// Định nghĩa kiểu cho dữ liệu kết quả
	/**
	 * @typedef {Object} ExampleResult
	 * @property {string} status
	 * @property {string} address
	 * @property {string} province
	 * @property {string} district
	 * @property {string} ward
	 * @property {string} [poi_vn]
	 * @property {string} [poi_en]
	 * @property {string} [poi_ex]
	 * @property {string} [type]
	 * @property {string} [sub_type]
	 * @property {string} [poi_st_sd]
	 * @property {string} [room]
	 * @property {string} [house_num]
	 * @property {string} [buaname]
	 * @property {string} [st_name]
	 * @property {string} [sub_com]
	 * @property {string} [phone]
	 * @property {string} [fax]
	 * @property {string} [web]
	 * @property {string} [mail]
	 * @property {string} [brandname]
	 * @property {string} [import]
	 * @property {string} [status_detail]
	 * @property {string} [note]
	 * @property {string} [dine]
	 * @property {string} [update_]
	 * @property {string} [source]
	 * @property {string} [gen_type]
	 * @property {string} [perform]
	 * @property {string} [dup]
	 * @property {string} [explain]
	 * @property {string} [classify]
	 * @property {string} [dtrend]
	 * @property {string} [google_id]
	 * @property {string} [be_id]
	 */

	// Store để lưu trữ dữ liệu với kiểu được định nghĩa
	/** @type {import('svelte/store').Writable<ExampleResult | null>} */
	const resultData = writable(null);
	const hasData = writable(false);

	onMount(() => {
		/** @type {import('@tauri-apps/api/event').UnlistenFn | undefined} */
		let unlisten;

		async function setupListener() {
			try {
				unlisten = await listen('new-data', (event) => {
					console.log('Received new data from backend:', event.payload);
					resultData.set(event.payload);
					hasData.set(true);
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

	function handleReset() {
		resultData.set(null);
		hasData.set(false);
	}
</script>

<div class="container h-full mx-auto py-8">
	<div class="max-w-6xl mx-auto space-y-8">
		<!-- Header -->
		<div class="text-center space-y-4">
			<h1 class="h1 text-primary-600">Arcgis POI Services</h1>
			<p class="text-xl text-surface-600">
				Real-time Geocoding from Goong.io API
			</p>
			<div class="flex justify-center items-center space-x-4">
				<div class="flex items-center space-x-2">
					<div class="w-3 h-3 rounded-full bg-success-500 animate-pulse"></div>
					<span class="text-sm text-surface-500">Server: http://127.0.0.1:31203</span>
				</div>
				{#if $hasData}
					<div class="flex items-center space-x-2">
						<div class="w-3 h-3 rounded-full bg-primary-500"></div>
						<span class="text-sm text-surface-500">Waiting for Python Addin calls...</span>
					</div>
				{/if}
			</div>
		</div>

		<!-- Main Content -->
		<div class="grid grid-cols-1 lg:grid-cols-3 gap-8">
			<!-- Status Panel -->
			<div class="lg:col-span-1 space-y-6">
				<!-- Connection Status -->
				<div class="card variant-filled-surface p-6">
					<h2 class="h2 mb-4">Connection Status</h2>
					<div class="space-y-4">
						<div class="flex items-center justify-between">
							<span class="font-medium">Backend Server:</span>
							<span class="badge variant-filled-success">Running</span>
						</div>
						<div class="flex items-center justify-between">
							<span class="font-medium">Port:</span>
							<span class="badge variant-outline-surface">31203</span>
						</div>
						<div class="flex items-center justify-between">
							<span class="font-medium">Ready for:</span>
							<span class="badge variant-filled-primary">Python Addin</span>
						</div>
					</div>
				</div>

				<!-- Latest Activity -->
				<div class="card variant-filled-surface p-6">
					<h3 class="h3 mb-4">Latest Activity</h3>
					<div class="space-y-3">
						{#if $hasData}
							<div class="flex items-center justify-between">
								<span class="font-medium">Last Update:</span>
								<span class="badge variant-filled-success">
									{new Date().toLocaleTimeString()}
								</span>
							</div>
							<div class="flex items-center justify-between">
								<span class="font-medium">Data Status:</span>
								<span class:badge-success={$resultData?.status === 'ok'} class:badge-error={$resultData?.status !== 'ok'}>
									{$resultData?.status || 'unknown'}
								</span>
							</div>
							<button
								class="btn variant-outline-surface w-full mt-4"
								on:click={handleReset}
							>
								Clear Display
							</button>
						{:else}
							<div class="text-center py-4">
								<div class="text-surface-400 mb-2">No data received yet</div>
								<div class="text-sm text-surface-500">
									Waiting for Python Addin to call the API...
								</div>
							</div>
						{/if}
					</div>
				</div>

				<!-- Instructions -->
				<div class="card variant-filled-surface p-6">
					<h3 class="h3 mb-4">How to Use</h3>
					<div class="space-y-3 text-sm text-surface-600">
						<p>1. Python Addin should POST to:</p>
						<code class="code text-xs block p-2 bg-surface-100 rounded">
							http://127.0.0.1:31203/process
						</code>
						<p>2. Send JSON with lat/lng coordinates:</p>
						<code class="code text-xs block p-2 bg-surface-100 rounded">
							<!-- {"{"}"lat": 21.041797, "lng": 105.803279{"}"} -->
						</code>
						<p>3. Results will auto-update here</p>
					</div>
				</div>
			</div>

			<!-- Results Display -->
			<div class="lg:col-span-2">
				{#if $hasData && $resultData}
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

						<!-- Contact Information -->
						{#if $resultData.phone || $resultData.mail || $resultData.web}
							<div class="card variant-filled-surface p-6">
								<h3 class="h3 mb-4">📞 Contact Information</h3>
								<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
									{#if $resultData.phone}
										<div class="text-center p-3 bg-primary-500/10 rounded">
											<div class="text-sm text-surface-500">Phone</div>
											<div class="font-semibold">{$resultData.phone}</div>
										</div>
									{/if}
									{#if $resultData.mail}
										<div class="text-center p-3 bg-secondary-500/10 rounded">
											<div class="text-sm text-surface-500">Email</div>
											<div class="font-semibold">{$resultData.mail}</div>
										</div>
									{/if}
									{#if $resultData.web}
										<div class="text-center p-3 bg-tertiary-500/10 rounded">
											<div class="text-sm text-surface-500">Website</div>
											<div class="font-semibold">{$resultData.web}</div>
										</div>
									{/if}
								</div>
							</div>
						{/if}
					</div>
				{:else}
					<!-- Empty State -->
					<div class="card variant-filled-surface p-12 text-center">
						<div class="space-y-10 flex flex-col items-center">
							<!-- Animated Logo -->
							<figure>
								<section class="img-bg" />
								<svg
									class="fill-token -scale-x-[100%]"
									xmlns="http://www.w3.org/2000/svg"
									viewBox="0 0 200 200"
								>
									<path
										fill-rule="evenodd"
										d="M98.77 50.95c25.1 0 46.54 8.7 61.86 23a41.34 41.34 0 0 0 5.19-1.93c4.35-2.02 10.06-6.17 17.13-12.43-1.15 10.91-2.38 18.93-3.7 24.04-.7 2.75-1.8 6.08-3.3 10a80.04 80.04 0 0 1 8.42 23.33c6.04 30.3-4.3 43.7-28.33 51.18.18.9.32 1.87.42 2.9.86 8.87-3.62 23.19-9 23.19-3.54 0-5.84-4.93-8.3-12.13-.78 8.34-4.58 17.9-8.98 17.9-4.73 0-7.25-8.84-10.93-20.13a214 214 0 0 1-.64 2.93l-.16.71-.16.71-.17.71c-1.84 7.58-4.46 15.07-8.5 15.07-5.06 0-2.29-15.9-10.8-22.63-43.14 2.36-79.43-13.6-79.43-59.62 0-8.48 2-16.76 5.69-24.45a93.72 93.72 0 0 1-1.77-3.68c-2.87-6.32-6.3-15.88-10.31-28.7 10.26 7.66 18.12 12.22 23.6 13.68.5.14 1.02.26 1.57.36 14.36-14.44 35.88-24.01 60.6-24.01Zm-9.99 62.3c-14.57 0-26.39 11.45-26.39 25.58 0 14.14 11.82 25.6 26.39 25.6s26.39-11.46 26.39-25.6c0-13.99-11.58-25.35-25.95-25.58Zm37.45 31.95c-4.4 0-6.73 9.4-6.73 13.62 0 3.3 1.1 5.12 2.9 5.45 4.39.4 3.05-5.97 5.23-5.97 1.06 0 2.2 1.35 3.34 2.73l.34.42c1.25 1.52 2.5 2.93 3.64 2.49 2.7-1.61 1.67-5.12.74-7.88-3.3-6.96-5.05-10.86-9.46-10.86Zm-36.85-28.45c12.57 0 22.76 9.78 22.76 21.85 0 12.07-10.2 21.85-22.76 21.85-.77 0-1.53-.04-2.29-.11 11.5-1.1 20.46-10.42 20.46-21.74 0-11.32-8.97-20.63-20.46-21.74.76-.07 1.52-.1 2.3-.1Zm65.54-5c-10.04 0-18.18 10.06-18.18 22.47 0 12.4 8.14 22.47 18.18 22.47s18.18-10.06 18.18-22.47c0-12.41-8.14-22.48-18.18-22.48Zm.6 3.62c8.38 0 15.16 8.4 15.16 18.74 0 10.35-6.78 18.74-15.16 18.74-.77 0-1.54-.07-2.28-.21 7.3-1.36 12.89-9.14 12.89-18.53 0-9.4-5.6-17.17-12.89-18.53.74-.14 1.5-.2 2.28-.2Zm3.34-72.27.12.07c.58.38.75 1.16.37 1.74l-2.99 4.6c-.35.55-1.05.73-1.61.44l-.12-.07a1.26 1.26 0 0 1-.37-1.74l2.98-4.6a1.26 1.26 0 0 1 1.62-.44ZM39.66 42l.08.1 2.76 3.93a1.26 1.26 0 0 1-2.06 1.45l-2.76-3.94A1.26 1.26 0 0 1 39.66 42Zm63.28-42 2.85 24.13 10.62-11.94.28 29.72-2.1-.47a77.8 77.8 0 0 0-16.72-2.04c-4.96 0-9.61.67-13.96 2l-2.34.73L83.5 4.96l9.72 18.37L102.94 0Zm-1.87 13.39-7.5 17.96-7.3-13.8-1.03 19.93.22-.06a51.56 51.56 0 0 1 12.1-1.45h.31c4.58 0 9.58.54 15 1.61l.35.07-.15-16.54-9.79 11-2.21-18.72Zm38.86 19.23c.67.2 1.05.89.86 1.56l-.38 1.32c-.17.62-.8 1-1.42.89l-.13-.03a1.26 1.26 0 0 1-.86-1.56l.38-1.32c.19-.66.88-1.05 1.55-.86ZM63.95 31.1l.05.12.7 2.17a1.26 1.26 0 0 1-2.34.9l-.04-.12-.71-2.17a1.26 1.26 0 0 1 2.34-.9Z"
									/>
								</svg>
							</figure>

							<h2 class="h2">Ready to Receive Data</h2>
							<p class="text-surface-600 text-lg max-w-md">
								The application is running and waiting for Python Addin to send coordinate data.
								Results will appear here automatically.
							</p>
							<div class="flex items-center space-x-2 text-surface-500">
								<div class="w-2 h-2 rounded-full bg-success-500 animate-pulse"></div>
								<span class="text-sm">Listening on port 31203</span>
							</div>
						</div>
					</div>
				{/if}
			</div>
		</div>
	</div>
</div>

<style lang="postcss">
	figure {
		@apply flex relative flex-col;
	}
	figure svg,
	.img-bg {
		@apply w-64 h-64 md:w-80 md:h-80;
	}
	.img-bg {
		@apply absolute z-[-1] rounded-full blur-[50px] transition-all;
		animation: pulse 5s cubic-bezier(0, 0, 0, 0.5) infinite,
			glow 5s linear infinite;
	}
	@keyframes glow {
		0% {
			@apply bg-primary-400/50;
		}
		33% {
			@apply bg-secondary-400/50;
		}
		66% {
			@apply bg-tertiary-400/50;
		}
		100% {
			@apply bg-primary-400/50;
		}
	}
	@keyframes pulse {
		50% {
			transform: scale(1.5);
		}
	}

	.badge-success {
		@apply badge variant-filled-success;
	}

	.badge-error {
		@apply badge variant-filled-error;
	}
</style>