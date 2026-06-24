import {
	appendVideoDebugLine,
	isVideoDebugEnabled,
	resetVideoDebugSession,
	setVideoDebugEnabled,
	videoDebugElapsedMs
} from '$lib/video-debug.svelte';

export { isVideoDebugEnabled, setVideoDebugEnabled };

/** Matches `MAX_VIDEO_BYTES` in inertia-core. */
export const MAX_VIDEO_BYTES = 50 * 1024 * 1024;
/** Matches `MAX_THUMB_BYTES` in inertia-core. */
export const MAX_THUMB_BYTES = 256 * 1024;

const DEBUG_PREFIX = '[inertia:video]';

const THUMB_MAX_DIMENSION = 720;
const THUMB_JPEG_QUALITY = 0.82;

const READY_STATE = ['nothing', 'metadata', 'current', 'future', 'enough'] as const;
const NETWORK_STATE = ['empty', 'idle', 'loading', 'no-source'] as const;
function videoSnapshot(video: HTMLVideoElement) {
	const err = video.error;
	return {
		readyState: READY_STATE[video.readyState] ?? video.readyState,
		networkState: NETWORK_STATE[video.networkState] ?? video.networkState,
		width: video.videoWidth,
		height: video.videoHeight,
		duration: Number.isFinite(video.duration) ? video.duration : null,
		currentTime: video.currentTime,
		paused: video.paused,
		ended: video.ended,
		seeking: video.seeking,
		mediaError: err ? { code: err.code, message: err.message } : null
	};
}

function videoDebug(step: string, detail: Record<string, unknown> = {}): void {
	if (!isVideoDebugEnabled()) return;
	appendVideoDebugLine(step, detail);
	console.log(`${DEBUG_PREFIX} +${videoDebugElapsedMs()}ms ${step}`, detail);
}

function attachVideoDebugProbe(video: HTMLVideoElement, label: string): void {
	if (!isVideoDebugEnabled()) return;

	const events: (keyof HTMLMediaElementEventMap)[] = [
		'loadstart',
		'loadedmetadata',
		'loadeddata',
		'canplay',
		'canplaythrough',
		'playing',
		'pause',
		'seeking',
		'seeked',
		'waiting',
		'stalled',
		'suspend',
		'abort',
		'emptied',
		'error'
	];

	for (const event of events) {
		video.addEventListener(event, () => {
			videoDebug(`event:${event}`, { label, ...videoSnapshot(video) });
		});
	}

	videoDebug('probe attached', { label, ...videoSnapshot(video) });
}

function fileSnapshot(file: File) {
	return {
		name: file.name,
		type: file.type || '(empty)',
		sizeBytes: file.size,
		sizeMb: (file.size / (1024 * 1024)).toFixed(2)
	};
}

export type VideoPrepareStage = 'loading' | 'thumbnail' | 'reading';

export interface PreparedVideo {
	videoBase64: string;
	thumbBase64: string;
	durationMs: number;
	previewUrl: string;
}

export interface VideoPrepareProgress {
	stage: VideoPrepareStage;
	previewUrl?: string;
	durationMs?: number;
}

function isVideoFile(file: File): boolean {
	return file.type.startsWith('video/') || /\.(mp4|webm|mov|m4v)$/i.test(file.name);
}

export function assertVideoUploadAllowed(file: File): void {
	if (!isVideoFile(file)) {
		videoDebug('assert rejected: not a video', fileSnapshot(file));
		throw new Error('Only video files are supported (MP4, WebM, MOV)');
	}
	if (file.size > MAX_VIDEO_BYTES) {
		videoDebug('assert rejected: too large', {
			...fileSnapshot(file),
			maxMb: MAX_VIDEO_BYTES / (1024 * 1024)
		});
		throw new Error(`Video too large — max ${MAX_VIDEO_BYTES / (1024 * 1024)} MB`);
	}
	videoDebug('assert ok', fileSnapshot(file));
}

function canvasToBlob(canvas: HTMLCanvasElement, quality: number): Promise<Blob> {
	return new Promise((resolve, reject) => {
		canvas.toBlob(
			(result) => (result ? resolve(result) : reject(new Error('Failed to compress thumbnail'))),
			'image/jpeg',
			quality
		);
	});
}

function blobToDataUrl(blob: Blob): Promise<string> {
	return new Promise((resolve, reject) => {
		const reader = new FileReader();
		reader.onload = () => resolve(reader.result as string);
		reader.onerror = () => reject(new Error('Failed to read thumbnail'));
		reader.readAsDataURL(blob);
	});
}

function fileToBase64(file: File): Promise<string> {
	return new Promise((resolve, reject) => {
		const reader = new FileReader();
		reader.onload = () => {
			const dataUrl = reader.result as string;
			const base64 = dataUrl.split(',')[1];
			if (!base64) reject(new Error('Failed to read video'));
			else resolve(base64);
		};
		reader.onerror = () => reject(new Error('Failed to read video'));
		reader.readAsDataURL(file);
	});
}

function mountOffscreenVideo(): HTMLVideoElement {
	const video = document.createElement('video');
	video.preload = 'auto';
	video.muted = true;
	video.playsInline = true;
	video.setAttribute('playsinline', '');
	video.style.position = 'fixed';
	video.style.left = '-9999px';
	video.style.top = '0';
	video.style.width = '320px';
	video.style.height = 'auto';
	video.style.opacity = '0';
	video.style.pointerEvents = 'none';
	document.body.appendChild(video);
	return video;
}

function hasVideoDimensions(video: HTMLVideoElement): boolean {
	return video.videoWidth > 0 && video.videoHeight > 0;
}

async function waitForVideoReady(video: HTMLVideoElement): Promise<void> {
	if (hasVideoDimensions(video)) {
		videoDebug('waitForVideoReady: already ready', videoSnapshot(video));
		return;
	}

	videoDebug('waitForVideoReady: waiting', videoSnapshot(video));

	await new Promise<void>((resolve, reject) => {
		const finish = (source: string) => {
			if (!hasVideoDimensions(video)) {
				videoDebug(`waitForVideoReady: ${source} but no dimensions yet`, videoSnapshot(video));
				return;
			}
			videoDebug(`waitForVideoReady: ready via ${source}`, videoSnapshot(video));
			cleanup();
			resolve();
		};
		const fail = () => {
			videoDebug('waitForVideoReady: failed', videoSnapshot(video));
			cleanup();
			reject(new Error('Unsupported or corrupt video file'));
		};
		const cleanup = () => {
			video.removeEventListener('loadeddata', onLoadedData);
			video.removeEventListener('loadedmetadata', onLoadedMetadata);
			video.removeEventListener('error', fail);
		};
		const onLoadedData = () => finish('loadeddata');
		const onLoadedMetadata = () => finish('loadedmetadata');

		video.addEventListener('loadeddata', onLoadedData);
		video.addEventListener('loadedmetadata', onLoadedMetadata);
		video.addEventListener('error', fail, { once: true });
		finish('initial check');
	});
}

async function loadVideoElement(file: File): Promise<{ video: HTMLVideoElement; objectUrl: string }> {
	resetVideoDebugSession();
	videoDebug('loadVideoElement: start', fileSnapshot(file));

	const objectUrl = URL.createObjectURL(file);
	const video = mountOffscreenVideo();
	attachVideoDebugProbe(video, file.name);
	video.src = objectUrl;
	videoDebug('loadVideoElement: src set, calling load()', videoSnapshot(video));
	video.load();
	await waitForVideoReady(video);
	videoDebug('loadVideoElement: ready', videoSnapshot(video));
	return { video, objectUrl };
}

async function seekVideo(video: HTMLVideoElement, time: number): Promise<void> {
	const duration = video.duration;
	const target =
		Number.isFinite(duration) && duration > 0
			? Math.min(Math.max(0, time), Math.max(0, duration - 0.04))
			: Math.max(0, time);

	videoDebug('seekVideo: start', { target, ...videoSnapshot(video) });

	if (
		!video.seeking &&
		video.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA &&
		Math.abs(video.currentTime - target) < 0.05
	) {
		videoDebug('seekVideo: already at target', videoSnapshot(video));
		return;
	}

	await new Promise<void>((resolve, reject) => {
		const onSeeked = () => {
			videoDebug('seekVideo: seeked', videoSnapshot(video));
			cleanup();
			resolve();
		};
		const onError = () => {
			videoDebug('seekVideo: error', videoSnapshot(video));
			cleanup();
			reject(new Error('Failed to read video frame'));
		};
		const cleanup = () => {
			video.removeEventListener('seeked', onSeeked);
			video.removeEventListener('error', onError);
		};
		video.addEventListener('seeked', onSeeked, { once: true });
		video.addEventListener('error', onError, { once: true });
		try {
			video.currentTime = target;
			if (
				!video.seeking &&
				video.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA &&
				Math.abs(video.currentTime - target) < 0.05
			) {
				videoDebug('seekVideo: seeked synchronously', videoSnapshot(video));
				cleanup();
				resolve();
			}
		} catch (e) {
			videoDebug('seekVideo: currentTime threw', {
				target,
				error: e instanceof Error ? e.message : String(e),
				...videoSnapshot(video)
			});
			cleanup();
			resolve();
		}
	});
}

function seekCandidates(duration: number): number[] {
	if (!Number.isFinite(duration) || duration <= 0) return [0];
	const max = Math.max(0, duration - 0.04);
	return [...new Set([0.12, 0.5, 1, duration * 0.15, 0].map((t) => Math.min(t, max)))];
}

function isCanvasMostlyBlack(canvas: HTMLCanvasElement): boolean {
	const ctx = canvas.getContext('2d');
	if (!ctx || canvas.width === 0 || canvas.height === 0) return true;

	const w = Math.min(canvas.width, 48);
	const h = Math.min(canvas.height, 48);
	const { data } = ctx.getImageData(0, 0, w, h);
	let dark = 0;
	const pixels = data.length / 4;
	for (let i = 0; i < data.length; i += 4) {
		if (data[i]! + data[i + 1]! + data[i + 2]! < 36) dark++;
	}
	return dark / pixels > 0.92;
}

function drawVideoFrame(video: HTMLVideoElement): HTMLCanvasElement {
	const longest = Math.max(video.videoWidth, video.videoHeight, 1);
	const scale = longest > THUMB_MAX_DIMENSION ? THUMB_MAX_DIMENSION / longest : 1;
	const width = Math.max(1, Math.round(video.videoWidth * scale));
	const height = Math.max(1, Math.round(video.videoHeight * scale));

	const canvas = document.createElement('canvas');
	canvas.width = width;
	canvas.height = height;
	const ctx = canvas.getContext('2d');
	if (!ctx) throw new Error('Failed to generate thumbnail');
	ctx.drawImage(video, 0, 0, width, height);
	return canvas;
}

async function captureThumb(video: HTMLVideoElement): Promise<{ base64: string; previewUrl: string }> {
	videoDebug('captureThumb: start', videoSnapshot(video));

	const candidates = seekCandidates(video.duration);
	let canvas: HTMLCanvasElement | null = null;

	for (const time of candidates) {
		videoDebug('captureThumb: seeking frame', { time });
		await seekVideo(video, time);
		const attempt = drawVideoFrame(video);
		const black = isCanvasMostlyBlack(attempt);
		videoDebug('captureThumb: frame sample', {
			time,
			black,
			canvasWidth: attempt.width,
			canvasHeight: attempt.height
		});
		canvas = attempt;
		if (!black) break;
	}

	if (!canvas) throw new Error('Failed to generate thumbnail');

	let quality = THUMB_JPEG_QUALITY;
	let blob = await canvasToBlob(canvas, quality);
	while (blob.size > MAX_THUMB_BYTES && quality > 0.4) {
		quality -= 0.1;
		blob = await canvasToBlob(canvas, quality);
	}
	if (blob.size > MAX_THUMB_BYTES) {
		videoDebug('captureThumb: thumb too large', { bytes: blob.size, quality });
		throw new Error('Video thumbnail too large — try a shorter clip');
	}

	const previewUrl = await blobToDataUrl(blob);
	const base64 = previewUrl.split(',')[1];
	if (!base64) throw new Error('Failed to read thumbnail');
	videoDebug('captureThumb: done', { thumbBytes: blob.size, quality });
	return { base64, previewUrl };
}

function disposeVideo(video: HTMLVideoElement, objectUrl: string) {
	videoDebug('disposeVideo', videoSnapshot(video));
	video.pause();
	video.removeAttribute('src');
	video.load();
	video.remove();
	URL.revokeObjectURL(objectUrl);
}

export async function prepareVideoForUpload(
	file: File,
	onProgress?: (progress: VideoPrepareProgress) => void
): Promise<PreparedVideo> {
	videoDebug('prepareVideoForUpload: start', fileSnapshot(file));
	assertVideoUploadAllowed(file);

	onProgress?.({ stage: 'loading' });
	videoDebug('stage', { stage: 'loading' });

	const { video, objectUrl } = await loadVideoElement(file);
	try {
		onProgress?.({ stage: 'thumbnail' });
		videoDebug('stage', { stage: 'thumbnail' });

		const durationMs = Math.round((video.duration || 0) * 1000);
		if (!Number.isFinite(durationMs) || durationMs <= 0) {
			videoDebug('prepareVideoForUpload: invalid duration', videoSnapshot(video));
			throw new Error('Could not read video duration');
		}

		const { base64: thumbBase64, previewUrl } = await captureThumb(video);

		onProgress?.({ stage: 'reading', previewUrl, durationMs });
		videoDebug('stage', { stage: 'reading', durationMs });

		const readStart = performance.now();
		const videoBase64 = await fileToBase64(file);
		videoDebug('fileToBase64: done', {
			elapsedMs: Math.round(performance.now() - readStart),
			base64Chars: videoBase64.length
		});

		videoDebug('prepareVideoForUpload: complete', {
			durationMs,
			totalElapsedMs: videoDebugElapsedMs()
		});
		return { videoBase64, thumbBase64, durationMs, previewUrl };
	} catch (e) {
		videoDebug('prepareVideoForUpload: failed', {
			error: e instanceof Error ? e.message : String(e),
			elapsedMs: videoDebugElapsedMs(),
			...videoSnapshot(video)
		});
		throw e;
	} finally {
		disposeVideo(video, objectUrl);
	}
}

export function isVideoUploadFile(file: File): boolean {
	return isVideoFile(file);
}

export function formatVideoDuration(durationMs: number): string {
	const totalSec = Math.max(0, Math.round(durationMs / 1000));
	const min = Math.floor(totalSec / 60);
	const sec = totalSec % 60;
	return `${min}:${sec.toString().padStart(2, '0')}`;
}

export function processingLabel(stage: VideoPrepareStage | null): string {
	switch (stage) {
		case 'loading':
			return 'Loading video…';
		case 'thumbnail':
			return 'Generating thumbnail…';
		case 'reading':
			return 'Reading video file…';
		default:
			return 'Processing video…';
	}
}
