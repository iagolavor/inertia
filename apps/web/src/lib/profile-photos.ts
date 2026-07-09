import type { FeedItem, ProfilePhoto } from '$lib/api';

/** Build a FeedItem-shaped view model from a durable profile item for the expand panel. */
export function profileItemToFeedItem(
	item: ProfilePhoto,
	authorId: string,
	authorName: string,
	isOwn = false
): FeedItem {
	return {
		content_id: item.content_id ?? item.id,
		author_id: authorId,
		author_name: authorName,
		body: item.caption ?? '',
		media_ref: item.blob_hash,
		thumb_ref: item.blob_hash,
		media_kind: 'photo',
		media_ready: true,
		created_at: item.created_at,
		expires_at: item.created_at,
		is_own: isOwn,
		is_archived: false,
		comment_count: 0
	};
}
