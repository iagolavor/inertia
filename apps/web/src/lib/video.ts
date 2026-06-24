/** Matches `MAX_VIDEO_BYTES` in inertia-core. */
export const MAX_VIDEO_BYTES = 50 * 1024 * 1024;
/** Matches `MAX_THUMB_BYTES` in inertia-core. */
export const MAX_THUMB_BYTES = 256 * 1024;

const THUMB_MAX_DIMENSION = 720;
const THUMB_JPEG_QUALITY = 0.82;

export interface PreparedVideo {
	videoBase64: string;
	thumbBase64: string;
	durationMs: number;
	previewUrl: string;
}

function isVideoFile(file: File): boolean {
	return file.type.startsWith('video/') || /\.(mp4|webm|mov|m4v)$/i.test(file.name);
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

async function loadVideoElement(file: File): Promise<{ video: HTMLVideoElement; objectUrl: string }> {
	const objectUrl = URL.createObjectURL(file);
	const video = document.createElement('video');
	video.preload = 'metadata';
	video.muted = true;
	video.playsInline = true;

	await new Promise<void>((resolve, reject) => {
		video.onloadedmetadata = () => resolve();
		video.onerror = () => reject(new Error('Unsupported or corrupt video file'));
		video.src = objectUrl;
	});

	return { video, objectUrl };
}

async function seekVideo(video: HTMLVideoElement, time: number): Promise<void> {
	await new Promise<void>((resolve, reject) => {
		const onSeeked = () => {
			video.removeEventListener('seeked', onSeeked);
			resolve();
		};
		video.addEventListener('seeked', onSeeked);
		video.onerror = () => reject(new Error('Failed to read video frame'));
		video.currentTime = time;
	});
}

async function captureThumb(video: HTMLVideoElement): Promise<{ base64: string; previewUrl: string }> {
	const seekTime = Math.min(0.5, Math.max(0, (video.duration || 1) * 0.1));
	await seekVideo(video, seekTime);

	const longest = Math.max(video.videoWidth, video.videoHeight);
	const scale = longest > THUMB_MAX_DIMENSION ? THUMB_MAX_DIMENSION / longest : 1;
	const width = Math.round(video.videoWidth * scale);
	const height = Math.round(video.videoHeight * scale);

	const canvas = document.createElement('canvas');
	canvas.width = width;
	canvas.height = height;
	const ctx = canvas.getContext('2d');
	if (!ctx) throw new Error('Failed to generate thumbnail');
	ctx.drawImage(video, 0, 0, width, height);

	let quality = THUMB_JPEG_QUALITY;
	let blob = await canvasToBlob(canvas, quality);
	while (blob.size > MAX_THUMB_BYTES && quality > 0.4) {
		quality -= 0.1;
		blob = await canvasToBlob(canvas, quality);
	}
	if (blob.size > MAX_THUMB_BYTES) {
		throw new Error('Video thumbnail too large — try a shorter clip');
	}

	const previewUrl = await blobToDataUrl(blob);
	const base64 = previewUrl.split(',')[1];
	if (!base64) throw new Error('Failed to read thumbnail');
	return { base64, previewUrl };
}

export async function prepareVideoForUpload(file: File): Promise<PreparedVideo> {
	if (!isVideoFile(file)) {
		throw new Error('Only video files are supported (MP4, WebM, MOV)');
	}
	if (file.size > MAX_VIDEO_BYTES) {
		throw new Error(`Video too large — max ${MAX_VIDEO_BYTES / (1024 * 1024)} MB`);
	}

	const { video, objectUrl } = await loadVideoElement(file);
	try {
		const durationMs = Math.round((video.duration || 0) * 1000);
		if (!Number.isFinite(durationMs) || durationMs <= 0) {
			throw new Error('Could not read video duration');
		}
		const { base64: thumbBase64, previewUrl } = await captureThumb(video);
		const videoBase64 = await fileToBase64(file);
		return { videoBase64, thumbBase64, durationMs, previewUrl };
	} finally {
		URL.revokeObjectURL(objectUrl);
		video.removeAttribute('src');
		video.load();
	}
}

export function isVideoUploadFile(file: File): boolean {
	return isVideoFile(file);
}
