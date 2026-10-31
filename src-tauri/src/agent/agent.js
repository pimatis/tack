// tack agent runtime - injected into the main webview by the rust agent
// server on demand. evaluates to the api object, and is also assigned to
// window.__tackAgent so later calls skip re-injection. plain js on purpose:
// it runs through window.eval, not the bundler
(function () {
	const INTERACTIVE =
		'a,button,input,textarea,select,summary,[role="button"],[role="link"],[role="menuitem"],[role="option"],[role="tab"],[role="checkbox"],[role="switch"],[role="combobox"],[role="listbox"],[role="treeitem"],[role="slider"],[role="textbox"],[role="radio"],[contenteditable="true"],[tabindex]';
	// destructive verbs only: "empty trash" and "delete view" match, the
	// "Trash" nav link alone does not
	const DANGER_RE =
		/delete|remove|erase|empty|permanent|reset|destroy|destructive|sign out|log out/i;
	// destructive styling on the control itself or its one-hop parent; plain
	// utility tokens only, so variants like aria-invalid:border-destructive
	// (every form control) do not false-positive
	const DANGER_CLASS =
		/(?:^|\s)(?:(?:group-)?hover:)?(?:text|bg|border)-destructive(?:\/[\d.]+)?(?=\s|$)/;

	const state = { refs: new Map(), seq: 0 };

	// --- visual feedback overlay: ghost cursor + target highlight, so the
	// user can watch the agent work in real time. pure dom, pointer-events
	// none, sits above everything else
	const ICONS = {
		cursor:
			'<path fill="currentColor" d="M16.5744 19.1999L12.6361 15.2616L11.4334 16.4643C10.2022 17.6955 9.58656 18.3111 8.92489 18.1658C8.26322 18.0204 7.96225 17.2035 7.3603 15.5696L5.3527 10.1205C4.15187 6.86106 3.55146 5.23136 4.39141 4.39141C5.23136 3.55146 6.86106 4.15187 10.1205 5.35271L15.5696 7.3603C17.2035 7.96225 18.0204 8.26322 18.1658 8.92489C18.3111 9.58656 17.6955 10.2022 16.4643 11.4334L15.2616 12.6361L19.1999 16.5744C19.6077 16.9821 19.8116 17.186 19.9058 17.4135C20.0314 17.7168 20.0314 18.0575 19.9058 18.3608C19.8116 18.5882 19.6077 18.7921 19.1999 19.1999C18.7921 19.6077 18.5882 19.8116 18.3608 19.9058C18.0575 20.0314 17.7168 20.0314 17.4135 19.9058C17.186 19.8116 16.9821 19.6077 16.5744 19.1999Z"/>',
		camera:
			'<path fill="currentColor" fill-rule="evenodd" d="M9.77778 21H14.2222C17.3433 21 18.9038 21 20.0248 20.2646C20.51 19.9462 20.9267 19.5371 21.251 19.0607C22 17.9601 22 16.4279 22 13.3636C22 10.2994 22 8.76721 21.251 7.6666C20.9267 7.19014 20.51 6.78104 20.0248 6.46268C19.3044 5.99013 18.4027 5.82123 17.022 5.76086C16.3631 5.76086 15.7959 5.27068 15.6667 4.63636C15.4728 3.68489 14.6219 3 13.6337 3H10.3663C9.37805 3 8.52715 3.68489 8.33333 4.63636C8.20412 5.27068 7.63685 5.76086 6.978 5.76086C5.59733 5.82123 4.69555 5.99013 3.97524 6.46268C3.48995 6.78104 3.07328 7.19014 2.74902 7.6666C2 8.76721 2 10.2994 2 13.3636C2 16.4279 2 17.9601 2.74902 19.0607C3.07328 19.5371 3.48995 19.9462 3.97524 20.2646C5.09624 21 6.65675 21 9.77778 21ZM12 9.27273C9.69881 9.27273 7.83333 11.1043 7.83333 13.3636C7.83333 15.623 9.69881 17.4545 12 17.4545C14.3012 17.4545 16.1667 15.623 16.1667 13.3636C16.1667 11.1043 14.3012 9.27273 12 9.27273ZM12 10.9091C10.6193 10.9091 9.5 12.008 9.5 13.3636C9.5 14.7192 10.6193 15.8182 12 15.8182C13.3807 15.8182 14.5 14.7192 14.5 13.3636C14.5 12.008 13.3807 10.9091 12 10.9091ZM16.7222 10.0909C16.7222 9.63904 17.0953 9.27273 17.5556 9.27273H18.6667C19.1269 9.27273 19.5 9.63904 19.5 10.0909C19.5 10.5428 19.1269 10.9091 18.6667 10.9091H17.5556C17.0953 10.9091 16.7222 10.5428 16.7222 10.0909Z" clip-rule="evenodd"/>'
	};
	const ui = { cursor: null, iconEl: null, hideTimer: 0 };

	function ensureCursor() {
		if (ui.cursor && ui.cursor.isConnected) return ui.cursor;
		const el = document.createElement('div');
		el.id = 'tack-agent-cursor';
		el.style.cssText =
			'position:fixed;top:0;left:0;z-index:2147483647;pointer-events:none;display:flex;align-items:flex-start;gap:4px;opacity:0;transition:transform .18s ease-out,opacity .25s;will-change:transform';
		ui.iconEl = document.createElement('span');
		ui.iconEl.style.cssText =
			'width:22px;height:22px;color:#fff;filter:drop-shadow(0 1px 2px rgba(0,0,0,.8))';
		ui.iconEl.innerHTML = `<svg viewBox="0 0 24 24" width="22" height="22">${ICONS.cursor}</svg>`;
		const tag = document.createElement('span');
		tag.textContent = 'Agent';
		tag.style.cssText =
			'margin-top:14px;margin-left:-4px;padding:1px 7px;border-radius:999px;background:#6366f1;color:#fff;font:600 10px/16px system-ui;letter-spacing:.02em;box-shadow:0 1px 3px rgba(0,0,0,.4)';
		el.append(ui.iconEl, tag);
		document.documentElement.appendChild(el);
		ui.cursor = el;
		return el;
	}

	// slide the ghost cursor to a point and keep it visible briefly
	function moveCursor(x, y) {
		const el = ensureCursor();
		el.style.opacity = '1';
		el.style.transform = `translate(${x}px,${y}px)`;
		clearTimeout(ui.hideTimer);
		ui.hideTimer = setTimeout(() => {
			if (ui.cursor) ui.cursor.style.opacity = '0';
		}, 2500);
	}

	// dashed outline + soft fill on the element the agent is about to use
	function highlight(el) {
		const r = el.getBoundingClientRect();
		if (!r.width || !r.height) return;
		const box = document.createElement('div');
		box.style.cssText = `position:fixed;z-index:2147483646;pointer-events:none;left:${r.x - 3}px;top:${r.y - 3}px;width:${r.width + 6}px;height:${r.height + 6}px;border:1.5px dashed #6366f1;border-radius:6px;background:rgba(99,102,241,.12);transition:opacity .4s`;
		document.documentElement.appendChild(box);
		setTimeout(() => (box.style.opacity = '0'), 900);
		setTimeout(() => box.remove(), 1400);
	}

	// ripple marker where the click lands
	function flash(x, y) {
		const dot = document.createElement('div');
		dot.style.cssText = `position:fixed;z-index:2147483646;pointer-events:none;left:${x - 10}px;top:${y - 10}px;width:20px;height:20px;border-radius:50%;border:2px solid #6366f1;background:rgba(99,102,241,.35);transform:scale(.4);opacity:1;transition:transform .35s,opacity .35s`;
		document.documentElement.appendChild(dot);
		requestAnimationFrame(() => {
			dot.style.transform = 'scale(1.6)';
			dot.style.opacity = '0';
		});
		setTimeout(() => dot.remove(), 500);
	}

	// screenshot: swap cursor glyph to a camera and flash the viewport edge
	function markScreenshot() {
		ensureCursor();
		moveCursor(window.innerWidth / 2, window.innerHeight / 2);
		ui.iconEl.innerHTML = `<svg viewBox="0 0 24 24" width="22" height="22">${ICONS.camera}</svg>`;
		const frame = document.createElement('div');
		frame.style.cssText =
			'position:fixed;inset:0;z-index:2147483646;pointer-events:none;border:3px solid rgba(255,255,255,.85);box-shadow:inset 0 0 60px rgba(255,255,255,.25);transition:opacity .45s';
		document.documentElement.appendChild(frame);
		setTimeout(() => (frame.style.opacity = '0'), 350);
		setTimeout(() => frame.remove(), 800);
		setTimeout(() => {
			if (ui.iconEl) {
				ui.iconEl.innerHTML = `<svg viewBox="0 0 24 24" width="22" height="22">${ICONS.cursor}</svg>`;
			}
		}, 1200);
	}

	function visible(el) {
		if (!el.isConnected) return false;
		const style = getComputedStyle(el);
		if (style.display === 'none' || style.visibility === 'hidden') return false;
		const r = el.getBoundingClientRect();
		return r.width > 0 && r.height > 0;
	}

	function label(el) {
		const aria = el.getAttribute('aria-label');
		if (aria) return aria.trim();
		const text = (el.innerText || el.textContent || '').trim().replace(/\s+/g, ' ');
		if (text) return text.slice(0, 80);
		return (
			el.getAttribute('placeholder') ||
			el.getAttribute('title') ||
			el.getAttribute('value') ||
			el.getAttribute('name') ||
			''
		).slice(0, 80);
	}

	function roleOf(el) {
		return el.getAttribute('role') || el.tagName.toLowerCase();
	}

	// dangerous targets: destructive wording or destructive styling anywhere on
	// the element or a one-hop parent (icon buttons carry the class on <button>)
	function dangerous(el) {
		const cls =
			String(el.className || '') +
			' ' +
			String((el.parentElement && el.parentElement.className) || '');
		if (DANGER_CLASS.test(cls)) return true;
		return DANGER_RE.test(label(el));
	}

	function disabled(el) {
		return el.disabled === true || el.getAttribute('aria-disabled') === 'true';
	}

	function describe(el, ref) {
		const r = el.getBoundingClientRect();
		const vw = window.innerWidth;
		const vh = window.innerHeight;
		return {
			ref,
			tag: el.tagName.toLowerCase(),
			role: roleOf(el),
			label: label(el),
			x: Math.round(r.x),
			y: Math.round(r.y),
			w: Math.round(r.width),
			h: Math.round(r.height),
			offscreen: r.bottom < 0 || r.top > vh || r.right < 0 || r.left > vw,
			disabled: disabled(el),
			danger: dangerous(el)
		};
	}

	function snapshot() {
		state.refs.clear();
		state.seq = 0;
		const elements = [];
		for (const el of document.querySelectorAll(INTERACTIVE)) {
			if (!visible(el)) continue;
			const ref = ++state.seq;
			state.refs.set(ref, el);
			el.setAttribute('data-agent-ref', String(ref));
			elements.push(describe(el, ref));
		}
		return {
			url: location.pathname + location.search,
			title: document.title,
			count: elements.length,
			elements
		};
	}

	function resolve(p) {
		if (p.ref != null) {
			const el = state.refs.get(p.ref) || document.querySelector(`[data-agent-ref="${p.ref}"]`);
			if (el && el.isConnected) return el;
			throw new Error(`no element for ref ${p.ref} - take a fresh snapshot`);
		}
		if (typeof p.x === 'number' && typeof p.y === 'number') {
			const el = document.elementFromPoint(p.x, p.y);
			if (el) return el;
			throw new Error(`no element at ${p.x},${p.y}`);
		}
		throw new Error('target needs ref or x/y');
	}

	function center(el) {
		el.scrollIntoView({ block: 'nearest', inline: 'nearest' });
		const r = el.getBoundingClientRect();
		return { x: Math.round(r.x + r.width / 2), y: Math.round(r.y + r.height / 2) };
	}

	function mouse(type, x, y, opts) {
		const base = {
			bubbles: true,
			cancelable: true,
			composed: true,
			view: window,
			clientX: x,
			clientY: y,
			button: (opts && opts.button) || 0,
			buttons: type === 'pointerdown' || type === 'mousedown' ? 1 : 0
		};
		if (type.startsWith('pointer')) {
			return new PointerEvent(type, {
				...base,
				pointerId: 1,
				pointerType: 'mouse',
				isPrimary: true,
				...opts
			});
		}
		return new MouseEvent(type, { ...base, ...opts });
	}

	// full press chain; bits-ui listens on pointer*, plain buttons on click
	function press(el, x, y, button) {
		const extra = button === 2 ? { button: 2, buttons: 2 } : {};
		el.dispatchEvent(mouse('pointerdown', x, y, extra));
		el.dispatchEvent(mouse('mousedown', x, y, extra));
		el.focus({ preventScroll: true });
		el.dispatchEvent(mouse('pointerup', x, y, extra));
		el.dispatchEvent(mouse('mouseup', x, y, extra));
		if (button === 2) {
			el.dispatchEvent(mouse('contextmenu', x, y, extra));
		} else {
			el.dispatchEvent(mouse('click', x, y, extra));
		}
	}

	function click(p, dbl) {
		const el = resolve(p);
		const { x, y } = center(el);
		const danger = dangerous(el);
		const what = `${dbl ? 'double-click' : 'click'} "${label(el) || roleOf(el)}"`;
		if (danger && !p.confirmed) return { ok: false, danger: true, summary: what };
		moveCursor(x, y);
		highlight(el);
		press(el, x, y, 0);
		if (dbl) press(el, x, y, 0);
		flash(x, y);
		return { ok: true, action: what, x, y };
	}

	function rightClick(p) {
		const el = resolve(p);
		const { x, y } = center(el);
		const danger = dangerous(el);
		if (danger && !p.confirmed) {
			return { ok: false, danger: true, summary: `right-click "${label(el)}"` };
		}
		moveCursor(x, y);
		highlight(el);
		press(el, x, y, 2);
		flash(x, y);
		return { ok: true, action: `right-click "${label(el)}"`, x, y };
	}

	function hover(p) {
		const el = resolve(p);
		const { x, y } = center(el);
		moveCursor(x, y);
		el.dispatchEvent(mouse('pointerover', x, y));
		el.dispatchEvent(mouse('mouseover', x, y));
		el.dispatchEvent(mouse('pointermove', x, y));
		el.dispatchEvent(mouse('mousemove', x, y));
		return { ok: true, action: `hover "${label(el) || roleOf(el)}"`, x, y };
	}

	function type(p) {
		const el = resolve(p);
		const text = String(p.text ?? '');
		const { x, y } = center(el);
		moveCursor(x, y);
		highlight(el);
		el.focus({ preventScroll: true });
		const tag = el.tagName.toLowerCase();
		if (tag === 'input' || tag === 'textarea') {
			el.value = text;
			el.dispatchEvent(
				new InputEvent('input', { bubbles: true, data: text, inputType: 'insertText' })
			);
			el.dispatchEvent(new Event('change', { bubbles: true }));
			return { ok: true, action: `type ${text.length} chars into "${label(el) || tag}"` };
		}
		if (el.isContentEditable) {
			// execCommand keeps svelte/prosemirror bindings happy where raw dom writes do not
			document.execCommand('insertText', false, text);
			return { ok: true, action: `type ${text.length} chars into editable` };
		}
		for (const ch of text) {
			for (const t of ['keydown', 'keypress', 'keyup']) {
				el.dispatchEvent(new KeyboardEvent(t, { bubbles: true, key: ch }));
			}
			el.dispatchEvent(
				new InputEvent('input', { bubbles: true, data: ch, inputType: 'insertText' })
			);
		}
		return { ok: true, action: `typed ${text.length} chars` };
	}

	const KEYMAP = {
		enter: { key: 'Enter', code: 'Enter' },
		return: { key: 'Enter', code: 'Enter' },
		escape: { key: 'Escape', code: 'Escape' },
		esc: { key: 'Escape', code: 'Escape' },
		tab: { key: 'Tab', code: 'Tab' },
		space: { key: ' ', code: 'Space' },
		backspace: { key: 'Backspace', code: 'Backspace' },
		delete: { key: 'Delete', code: 'Delete' },
		home: { key: 'Home', code: 'Home' },
		end: { key: 'End', code: 'End' },
		pageup: { key: 'PageUp', code: 'PageUp' },
		pagedown: { key: 'PageDown', code: 'PageDown' },
		up: { key: 'ArrowUp', code: 'ArrowUp' },
		down: { key: 'ArrowDown', code: 'ArrowDown' },
		left: { key: 'ArrowLeft', code: 'ArrowLeft' },
		right: { key: 'ArrowRight', code: 'ArrowRight' }
	};

	// "meta+k", "ctrl+shift+z", "enter"; mac-style shortcuts use meta
	function key(p) {
		const parts = String(p.keys || '')
			.split('+')
			.map((s) => s.trim())
			.filter(Boolean);
		const mods = { meta: false, ctrl: false, alt: false, shift: false };
		let name = '';
		for (const part of parts) {
			const low = part.toLowerCase();
			if (['meta', 'cmd', 'command', 'super'].includes(low)) mods.meta = true;
			else if (['ctrl', 'control'].includes(low)) mods.ctrl = true;
			else if (['alt', 'option'].includes(low)) mods.alt = true;
			else if (low === 'shift') mods.shift = true;
			else name = part;
		}
		if (!name) throw new Error(`no key in "${p.keys}"`);
		const def = KEYMAP[name.toLowerCase()] || {
			key: name.length === 1 ? name : name,
			code: name.length === 1 ? `Key${name.toUpperCase()}` : name
		};
		const opts = {
			bubbles: true,
			cancelable: true,
			composed: true,
			key: def.key,
			code: def.code,
			metaKey: mods.meta,
			ctrlKey: mods.ctrl,
			altKey: mods.alt,
			shiftKey: mods.shift
		};
		const target =
			document.activeElement && document.activeElement !== document.body
				? document.activeElement
				: document.body;
		target.dispatchEvent(new KeyboardEvent('keydown', opts));
		target.dispatchEvent(new KeyboardEvent('keyup', opts));
		return { ok: true, action: `key ${p.keys}` };
	}

	function scrollableFrom(el) {
		for (let cur = el; cur && cur !== document.body; cur = cur.parentElement) {
			const s = getComputedStyle(cur);
			if (
				/(auto|scroll)/.test(s.overflowY + s.overflowX) &&
				(cur.scrollHeight > cur.clientHeight || cur.scrollWidth > cur.clientWidth)
			) {
				return cur;
			}
		}
		return document.scrollingElement || document.documentElement;
	}

	function scroll(p) {
		const dy = Number(p.dy ?? 0);
		const dx = Number(p.dx ?? 0);
		let box = null;
		if (p.ref != null || (typeof p.x === 'number' && typeof p.y === 'number')) {
			box = scrollableFrom(resolve(p));
		}
		box = box || document.scrollingElement || document.documentElement;
		box.scrollBy({ top: dy, left: dx, behavior: 'instant' });
		return { ok: true, action: `scroll ${dx},${dy}` };
	}

	async function drag(p) {
		const steps = Math.max(2, Math.min(40, Number(p.steps) || 12));
		const from =
			p.fromRef != null ? center(resolve({ ref: p.fromRef })) : { x: p.fromX, y: p.fromY };
		const to = p.toRef != null ? center(resolve({ ref: p.toRef })) : { x: p.toX, y: p.toY };
		if ([from.x, from.y, to.x, to.y].some((v) => typeof v !== 'number' || Number.isNaN(v))) {
			throw new Error('drag needs from/to coordinates or refs');
		}
		const src = document.elementFromPoint(from.x, from.y);
		const dst = document.elementFromPoint(to.x, to.y);
		if (!src || !dst) throw new Error('drag endpoints must be on visible elements');
		moveCursor(from.x, from.y);
		highlight(src);

		const dt = new DataTransfer();
		src.dispatchEvent(
			new DragEvent('dragstart', {
				bubbles: true,
				clientX: from.x,
				clientY: from.y,
				dataTransfer: dt
			})
		);
		src.dispatchEvent(mouse('pointerdown', from.x, from.y));
		src.dispatchEvent(mouse('mousedown', from.x, from.y));
		for (let i = 1; i <= steps; i++) {
			const x = Math.round(from.x + ((to.x - from.x) * i) / steps);
			const y = Math.round(from.y + ((to.y - from.y) * i) / steps);
			const over = document.elementFromPoint(x, y) || dst;
			over.dispatchEvent(mouse('pointermove', x, y, { buttons: 1 }));
			over.dispatchEvent(mouse('mousemove', x, y, { buttons: 1 }));
			moveCursor(x, y);
			over.dispatchEvent(
				new DragEvent('dragover', { bubbles: true, clientX: x, clientY: y, dataTransfer: dt })
			);
			await new Promise((r) => setTimeout(r, 12));
		}
		dst.dispatchEvent(
			new DragEvent('drop', { bubbles: true, clientX: to.x, clientY: to.y, dataTransfer: dt })
		);
		dst.dispatchEvent(mouse('pointerup', to.x, to.y));
		dst.dispatchEvent(mouse('mouseup', to.x, to.y));
		src.dispatchEvent(
			new DragEvent('dragend', { bubbles: true, clientX: to.x, clientY: to.y, dataTransfer: dt })
		);
		return { ok: true, action: `drag ${from.x},${from.y} -> ${to.x},${to.y}` };
	}

	// serialize eval results safely: elements get described, big payloads
	// truncated, throwables stringified
	function serializable(value, depth) {
		if (depth > 4) return '[max depth]';
		if (value == null || typeof value === 'number' || typeof value === 'boolean') return value;
		if (typeof value === 'string') return value.length > 4000 ? value.slice(0, 4000) + '…' : value;
		if (value instanceof Element) return describe(value, 0);
		if (Array.isArray(value)) {
			return value.slice(0, 200).map((v) => serializable(v, depth + 1));
		}
		if (typeof value === 'object') {
			const out = {};
			for (const k of Object.keys(value).slice(0, 100)) out[k] = serializable(value[k], depth + 1);
			return out;
		}
		return String(value);
	}

	async function act(name, p) {
		switch (name) {
			case 'click':
				return click(p || {});
			case 'dblclick':
				return click({ ...(p || {}), confirmed: p && p.confirmed }, true);
			case 'rightclick':
				return rightClick(p || {});
			case 'hover':
				return hover(p || {});
			case 'type':
				return type(p || {});
			case 'key':
				return key(p || {});
			case 'scroll':
				return scroll(p || {});
			case 'drag':
				return drag(p || {});
			case 'goto': {
				const path = String((p && p.path) || '/');
				setTimeout(() => location.assign(path), 0);
				return { ok: true, action: `navigate ${path}` };
			}
			case 'wait':
				await new Promise((r) => setTimeout(r, Math.min(Number(p && p.ms) || 500, 5000)));
				return { ok: true };
			case 'snapshot':
				return { ok: true, snapshot: snapshot() };
			case 'notify':
				if (p && p.kind === 'screenshot') markScreenshot();
				return { ok: true };
			case 'eval': {
				const code = String((p && p.code) || '');
				// indirect eval: runs in global scope, `this` is undefined
				const value = (0, eval)(code);
				const result = value && typeof value.then === 'function' ? await value : value;
				return { ok: true, result: serializable(result, 0) };
			}
			default:
				throw new Error(`unknown action "${name}"`);
		}
	}

	return { snapshot, act };
})();
