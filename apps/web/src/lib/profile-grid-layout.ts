import type { ProfilePhoto } from '$lib/api';

export const PROFILE_GRID_COLS = 3;
export const EXPAND_COL_SPAN = 2;
export const EXPAND_ROW_SPAN = 2;

export type ProfileGridCell =
	| { kind: 'thumb'; photo: ProfilePhoto; photoIndex: number; row: number; col: number }
	| {
			kind: 'expand';
			photo: ProfilePhoto | null;
			photoIndex: number;
			row: number;
			col: number;
			rowSpan: number;
			colSpan: number;
	  };

function naturalSlot(photoIndex: number): { row: number; col: number } {
	return { row: Math.floor(photoIndex / PROFILE_GRID_COLS), col: photoIndex % PROFILE_GRID_COLS };
}

function expandAnchor(photoIndex: number): { row: number; col: number } {
	const { row, col } = naturalSlot(photoIndex);
	return {
		row,
		col: col >= PROFILE_GRID_COLS - 1 ? col - 1 : col
	};
}

function cellKey(row: number, col: number): string {
	return `${row},${col}`;
}

export function computeProfileGridLayoutAtIndex(
	photos: ProfilePhoto[],
	expandIndex: number
): ProfileGridCell[] {
	if (expandIndex < 0 || expandIndex > photos.length) {
		return photos.map((photo, photoIndex) => ({
			kind: 'thumb' as const,
			photo,
			photoIndex,
			row: naturalSlot(photoIndex).row,
			col: naturalSlot(photoIndex).col
		}));
	}

	const cells: ProfileGridCell[] = [];
	const occupied = new Set<string>();
	const anchor = expandAnchor(expandIndex);

	for (let dr = 0; dr < EXPAND_ROW_SPAN; dr++) {
		for (let dc = 0; dc < EXPAND_COL_SPAN; dc++) {
			occupied.add(cellKey(anchor.row + dr, anchor.col + dc));
		}
	}

	cells.push({
		kind: 'expand',
		photo: photos[expandIndex] ?? null,
		photoIndex: expandIndex,
		row: anchor.row,
		col: anchor.col,
		rowSpan: EXPAND_ROW_SPAN,
		colSpan: EXPAND_COL_SPAN
	});

	let cursor = { row: 0, col: 0 };

	const nextFree = (): { row: number; col: number } => {
		while (occupied.has(cellKey(cursor.row, cursor.col))) {
			cursor.col++;
			if (cursor.col >= PROFILE_GRID_COLS) {
				cursor.col = 0;
				cursor.row++;
			}
		}
		const spot = { row: cursor.row, col: cursor.col };
		occupied.add(cellKey(spot.row, spot.col));
		cursor.col++;
		if (cursor.col >= PROFILE_GRID_COLS) {
			cursor.col = 0;
			cursor.row++;
		}
		return spot;
	};

	for (let i = 0; i < photos.length; i++) {
		if (i === expandIndex) continue;
		const spot = nextFree();
		cells.push({ kind: 'thumb', photo: photos[i], photoIndex: i, row: spot.row, col: spot.col });
	}

	return cells;
}

export function computeProfileGridLayout(
	photos: ProfilePhoto[],
	selectedContentId: string | null
): ProfileGridCell[] {
	const selectedIndex =
		selectedContentId == null
			? -1
			: photos.findIndex((photo) => photo.content_id === selectedContentId);

	return computeProfileGridLayoutAtIndex(photos, selectedIndex);
}

export function gridCellStyle(cell: ProfileGridCell): string {
	if (cell.kind === 'expand') {
		return `grid-row: ${cell.row + 1} / span ${cell.rowSpan}; grid-column: ${cell.col + 1} / span ${cell.colSpan};`;
	}
	return `grid-row: ${cell.row + 1}; grid-column: ${cell.col + 1};`;
}

/** Row-major order — keeps mobile stack aligned with desktop grid positions. */
export function sortProfileGridCells(cells: ProfileGridCell[]): ProfileGridCell[] {
	return [...cells].sort((a, b) => a.row - b.row || a.col - b.col);
}
