import type { FeedItem, ProfilePhoto } from '$lib/api';

/** Map synced feed posts into grid rows (photo posts only, oldest first). */
export function feedPostsToProfilePhotos(
	items: FeedItem[],
	authorSigningKey: string
): ProfilePhoto[] {
	return items
		.filter((item) => item.author_id === authorSigningKey && item.media_ref)
		.sort(
			(a, b) => new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
		)
		.map((item, index) => ({
			id: item.content_id,
			blob_hash: item.media_ref!,
			caption: item.body || null,
			content_id: item.content_id,
			sort_order: index,
			created_at: item.created_at
		}));
}
