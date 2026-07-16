import { goto } from '$app/navigation';
import { api, type Contact, type ConversationMessage } from '$lib/api';
import { markDmThreadRead } from '$lib/dm-unread';

export type ConversationPrefetch = {
	contactId: string;
	/** Snapshot from the thread list for instant header paint. */
	contact: Contact;
	contactPromise: Promise<Contact>;
	messagesPromise: Promise<ConversationMessage[]>;
};

let prefetch: ConversationPrefetch | null = null;

/** Start contact + message fetches on thread click, before navigation. */
export function primeConversationOpen(contact: Contact): ConversationPrefetch {
	const payload: ConversationPrefetch = {
		contactId: contact.id,
		contact,
		contactPromise: api.getContact(contact.id),
		messagesPromise: api.listConversationMessages(contact.id)
	};
	prefetch = payload;
	return payload;
}

export function takeConversationPrefetch(contactId: string): ConversationPrefetch | null {
	if (!prefetch || prefetch.contactId !== contactId) return null;
	const next = prefetch;
	prefetch = null;
	return next;
}

export function openConversation(contact: Contact) {
	markDmThreadRead(contact.id);
	if (contact.signing_pubkey) markDmThreadRead(contact.signing_pubkey);
	primeConversationOpen(contact);
	void goto(`/friends/${contact.id}`);
}
