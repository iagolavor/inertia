<script lang="ts">
	interface Props {
		contactId: string;
		active: 'messages' | 'posts';
	}

	let { contactId, active }: Props = $props();
</script>

<nav class="friend-tabs" aria-label="Friend sections">
	<a href="/friends/{contactId}" class:active={active === 'messages'} aria-current={active === 'messages' ? 'page' : undefined}>
		Messages
	</a>
	<a
		href="/friends/{contactId}/profile"
		class:active={active === 'posts'}
		aria-current={active === 'posts' ? 'page' : undefined}
	>
		Posts
	</a>
</nav>

<style>
	.friend-tabs {
		display: flex;
		border-top: 1px solid var(--border);
		margin-bottom: 0;
	}

	.friend-tabs a {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0.65rem 0.5rem;
		font-size: 0.72rem;
		font-weight: 700;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--muted);
		text-decoration: none;
		border-top: 2px solid transparent;
		margin-top: -1px;
	}

	.friend-tabs a:hover {
		color: var(--text);
		text-decoration: none;
	}

	.friend-tabs a.active {
		color: var(--text);
		border-top-color: var(--text);
	}
</style>
