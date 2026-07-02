import { describe, expect, it } from 'vitest';

import type { ConversationMessage } from '$lib/api';
import { fingerprintConversation } from '$lib/local-cache';

describe('conversation cache fingerprint', () => {
	const base: ConversationMessage[] = [
		{
			content_id: 'msg-own-1',
			body: 'first',
			at: '2026-06-27T12:00:00.000Z',
			expires_at: '2026-07-04T12:00:00.000Z',
			is_own: true,
			delivery_status: 'sent'
		},
		{
			content_id: 'msg-own-2',
			body: 'second',
			at: '2026-06-27T12:01:00.000Z',
			expires_at: '2026-07-04T12:00:00.000Z',
			is_own: true,
			delivery_status: 'sent'
		}
	];

	it('changes when an earlier own message delivery status updates', () => {
		const before = fingerprintConversation(base);
		const after = fingerprintConversation([
			{ ...base[0], delivery_status: 'delivered' },
			base[1]
		]);
		expect(after).not.toBe(before);
	});
});
