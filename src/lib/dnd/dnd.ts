import { draggable, droppable, dndState } from '@thisux/sveltednd';
import type {
	DragDropState,
	DraggableOptions,
	DragDropOptions,
	DragDropAttributes
} from '@thisux/sveltednd';

export type { DragDropState, DragDropAttributes };

// default css classes for consistent dnd visual feedback
const defaultAttributes: DragDropAttributes = {
	draggingClass: 'dragging',
	dragOverClass: 'drag-over'
};

// floating ghost preview that follows cursor during drag
let ghostEl: HTMLElement | null = null;

function createGhost(sourceNode: HTMLElement) {
	destroyGhost();
	const rect = sourceNode.getBoundingClientRect();
	ghostEl = document.createElement('div');
	ghostEl.className = 'dnd-ghost';
	ghostEl.style.cssText = `
		position: fixed;
		width: ${rect.width}px;
		height: ${rect.height}px;
		left: -9999px;
		top: -9999px;
		z-index: 9999;
		pointer-events: none;
		opacity: 0;
		border-radius: var(--radius-sm);
		background: var(--card);
		border: 1px solid var(--border);
		box-shadow: 0 12px 32px -6px rgba(0,0,0,0.22), 0 4px 12px -4px rgba(0,0,0,0.14);
		overflow: hidden;
		transform: rotate(-1deg) scale(0.96);
		transition: opacity 100ms ease-out;
	`;
	const clone = sourceNode.cloneNode(true) as HTMLElement;
	// remove dnd-related classes from clone so it renders at full opacity
	clone.classList.remove('dragging', 'svelte-dnd-dragging', 'svelte-dnd-draggable');
	clone.style.opacity = '';
	clone.style.filter = '';
	clone.style.cursor = '';
	clone.removeAttribute('id');
	ghostEl.appendChild(clone);
	document.body.appendChild(ghostEl);
	// fade in on next frame
	requestAnimationFrame(() => {
		if (ghostEl) ghostEl.style.opacity = '0.92';
	});
}

function moveGhost(x: number, y: number) {
	if (!ghostEl) return;
	ghostEl.style.left = `${x + 10}px`;
	ghostEl.style.top = `${y + 6}px`;
}

function destroyGhost() {
	if (ghostEl) {
		ghostEl.remove();
		ghostEl = null;
	}
}

// document-level pointer move listener for ghost tracking
let ghostMoveHandler: ((e: PointerEvent) => void) | null = null;

function startGhostTracking() {
	ghostMoveHandler = (e: PointerEvent) => moveGhost(e.clientX, e.clientY);
	document.addEventListener('pointermove', ghostMoveHandler);
}

function stopGhostTracking() {
	if (ghostMoveHandler) {
		document.removeEventListener('pointermove', ghostMoveHandler);
		ghostMoveHandler = null;
	}
}

// composite action: draggable + droppable + ondragstart preventDefault
export type SortableItemOptions<T> = {
	dragData: T;
	container: string;
	direction?: 'vertical' | 'horizontal' | 'grid';
	onDrop: (state: DragDropState<T>) => Promise<void> | void;
	onDragStart?: (state: DragDropState<T>) => void;
	onDragEnd?: (state: DragDropState<T>) => void;
	disabled?: boolean;
	attributes?: DragDropAttributes;
};

export function sortableItem<T>(node: HTMLElement, options: SortableItemOptions<T>) {
	let dragAction: ReturnType<typeof draggable<T>> | null = null;
	let dropAction: ReturnType<typeof droppable<T>> | null = null;

	const onDragStart = (e: DragEvent) => {
		e.preventDefault();
		e.stopImmediatePropagation();
	};

	function apply(opts: SortableItemOptions<T>) {
		node.setAttribute('draggable', 'true');

		const mergedAttrs = { ...defaultAttributes, ...opts.attributes };
		const dragOpts: DraggableOptions<T> = {
			dragData: opts.dragData,
			container: opts.container,
			disabled: opts.disabled,
			attributes: mergedAttrs,
			callbacks: {
				onDragStart: (state) => {
					createGhost(node);
					startGhostTracking();
					opts.onDragStart?.(state);
				},
				onDragEnd: (state) => {
					destroyGhost();
					stopGhostTracking();
					opts.onDragEnd?.(state);
				},
				onDrop: opts.onDrop
			}
		};
		const dropOpts: DragDropOptions<T> = {
			container: opts.container,
			direction: opts.direction ?? 'vertical',
			disabled: opts.disabled,
			callbacks: { onDrop: opts.onDrop },
			attributes: mergedAttrs
		};

		if (!dragAction) {
			dragAction = draggable<T>(node, dragOpts);
			dropAction = droppable<T>(node, dropOpts);
			node.addEventListener('dragstart', onDragStart, true);
		} else {
			dragAction.update(dragOpts);
			dropAction!.update(dropOpts);
		}
	}

	apply(options);

	return {
		update(newOptions: SortableItemOptions<T>) {
			apply(newOptions);
		},
		destroy() {
			node.removeEventListener('dragstart', onDragStart, true);
			dragAction?.destroy();
			dropAction?.destroy();
			destroyGhost();
			stopGhostTracking();
		}
	};
}

// drop-only action for empty containers (e.g. empty board columns)
export type DropZoneOptions<T> = {
	container: string;
	direction?: 'vertical' | 'horizontal' | 'grid';
	onDrop: (state: DragDropState<T>) => Promise<void> | void;
	disabled?: boolean;
	attributes?: DragDropAttributes;
};

export function dropZone<T>(node: HTMLElement, options: DropZoneOptions<T>) {
	let dropAction: ReturnType<typeof droppable<T>> | null = null;

	function apply(opts: DropZoneOptions<T>) {
		const mergedAttrs = { ...defaultAttributes, ...opts.attributes };
		const dropOpts: DragDropOptions<T> = {
			container: opts.container,
			direction: opts.direction ?? 'vertical',
			disabled: opts.disabled,
			callbacks: { onDrop: opts.onDrop },
			attributes: mergedAttrs
		};

		if (!dropAction) {
			dropAction = droppable<T>(node, dropOpts);
		} else {
			dropAction.update(dropOpts);
		}
	}

	apply(options);

	return {
		update(newOptions: DropZoneOptions<T>) {
			apply(newOptions);
		},
		destroy() {
			dropAction?.destroy();
		}
	};
}

// toggle dnd-active body class during drag
export function useDndActive() {
	if (dndState.isDragging) document.body.classList.add('dnd-active');
	else document.body.classList.remove('dnd-active');
}

// reorder array by moving dragged item before/after target
// returns original list when the move would be a no-op (same position)
export function reorderArray<T>(
	list: T[],
	draggedItem: T,
	targetItem: T,
	dropPosition: 'before' | 'after'
): T[] {
	const draggedIndex = list.indexOf(draggedItem);
	if (draggedIndex === -1) return list;

	// detect no-op: dropping adjacent in the same direction
	const targetIndex = list.indexOf(targetItem);
	if (targetIndex !== -1) {
		if (dropPosition === 'before' && draggedIndex === targetIndex - 1) return list;
		if (dropPosition === 'after' && draggedIndex === targetIndex + 1) return list;
		if (draggedIndex === targetIndex) return list;
	}

	const reordered = list.filter((item) => item !== draggedItem);
	const newTargetIndex = reordered.indexOf(targetItem);
	if (newTargetIndex !== -1) {
		if (dropPosition === 'after') reordered.splice(newTargetIndex + 1, 0, draggedItem);
		else reordered.splice(newTargetIndex, 0, draggedItem);
	} else {
		reordered.push(draggedItem);
	}
	return reordered;
}
