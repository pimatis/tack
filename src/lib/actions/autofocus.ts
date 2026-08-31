// focus element on mount so keydown events (Escape) reach overlays
export function autofocus(node: HTMLElement) {
	node.focus();
}
