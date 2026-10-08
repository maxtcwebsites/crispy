import type { Action } from 'svelte/action';

// ---- tooltip ------------------------------------------------------------------------------------

export interface TooltipOptions {
  text: string;
  /** Keycaps shown after the text, e.g. ['⌃', '⌥', '⇧', 'L']. */
  keys?: string[];
  placement?: 'top' | 'bottom' | 'right' | 'left';
}

type TooltipParam = string | TooltipOptions | null | undefined;

let tipEl: HTMLDivElement | null = null;
let showTimer: ReturnType<typeof setTimeout> | undefined;
let warm = false;
let coolTimer: ReturnType<typeof setTimeout> | undefined;

function tooltipElement(): HTMLDivElement {
  if (!tipEl) {
    tipEl = document.createElement('div');
    tipEl.className = 'crispy-tooltip';
    tipEl.setAttribute('role', 'tooltip');
    tipEl.id = 'crispy-tooltip';
    document.body.appendChild(tipEl);
  }
  return tipEl;
}

function normalise(p: TooltipParam): TooltipOptions | null {
  if (!p) return null;
  return typeof p === 'string' ? { text: p } : p;
}

function place(anchor: HTMLElement, el: HTMLElement, placement: TooltipOptions['placement'] = 'top') {
  const a = anchor.getBoundingClientRect();
  const t = el.getBoundingClientRect();
  const gap = 8;
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  let x = a.left + a.width / 2 - t.width / 2;
  let y = a.top - t.height - gap;
  if (placement === 'bottom' || (placement === 'top' && y < 6)) y = a.bottom + gap;
  if (placement === 'right') {
    x = a.right + gap;
    y = a.top + a.height / 2 - t.height / 2;
  }
  if (placement === 'left') {
    x = a.left - t.width - gap;
    y = a.top + a.height / 2 - t.height / 2;
  }
  x = Math.max(6, Math.min(vw - t.width - 6, x));
  y = Math.max(6, Math.min(vh - t.height - 6, y));
  el.style.left = `${Math.round(x)}px`;
  el.style.top = `${Math.round(y)}px`;
}

function render(el: HTMLElement, o: TooltipOptions) {
  el.textContent = '';
  const span = document.createElement('span');
  span.textContent = o.text;
  el.appendChild(span);
  if (o.keys?.length) {
    const keys = document.createElement('span');
    keys.className = 'keys';
    for (const k of o.keys) {
      const kbd = document.createElement('kbd');
      kbd.textContent = k;
      keys.appendChild(kbd);
    }
    el.appendChild(keys);
  }
}

/** Lightweight tooltip: `use:tooltip={'Lock to this computer'}` or `{ text, keys }`. */
export const tooltip: Action<HTMLElement, TooltipParam> = (node, param) => {
  let opts = normalise(param);
  let open = false;

  const show = () => {
    if (!opts) return;
    clearTimeout(showTimer);
    clearTimeout(coolTimer);
    showTimer = setTimeout(
      () => {
        if (!opts || !node.isConnected) return;
        const el = tooltipElement();
        render(el, opts);
        el.dataset.show = 'false';
        el.style.left = '-9999px';
        requestAnimationFrame(() => {
          if (!opts) return;
          place(node, el, opts.placement);
          el.dataset.show = 'true';
        });
        node.setAttribute('aria-describedby', 'crispy-tooltip');
        open = true;
        warm = true;
      },
      warm ? 60 : 450,
    );
  };
  const hide = () => {
    clearTimeout(showTimer);
    if (open && tipEl) tipEl.dataset.show = 'false';
    open = false;
    node.removeAttribute('aria-describedby');
    coolTimer = setTimeout(() => (warm = false), 500);
  };

  node.addEventListener('pointerenter', show);
  node.addEventListener('pointerleave', hide);
  node.addEventListener('focusin', show);
  node.addEventListener('focusout', hide);
  node.addEventListener('pointerdown', hide);

  return {
    update(p) {
      opts = normalise(p);
      if (open && opts && tipEl) {
        render(tipEl, opts);
        place(node, tipEl, opts.placement);
      }
      if (!opts) hide();
    },
    destroy() {
      hide();
      node.removeEventListener('pointerenter', show);
      node.removeEventListener('pointerleave', hide);
      node.removeEventListener('focusin', show);
      node.removeEventListener('focusout', hide);
      node.removeEventListener('pointerdown', hide);
    },
  };
};

// ---- portal -------------------------------------------------------------------------------------

/** Move an element to <body> so fixed positioning escapes transformed / clipped ancestors. */
export const portal: Action<HTMLElement> = (node) => {
  document.body.appendChild(node);
  return {
    destroy() {
      node.remove();
    },
  };
};

// ---- outside click ------------------------------------------------------------------------------

export const outside: Action<HTMLElement, { onoutside: () => void; ignore?: HTMLElement | null }> = (node, p) => {
  let param = p;
  const handler = (e: PointerEvent) => {
    const t = e.target as Node;
    if (node.contains(t) || param?.ignore?.contains(t)) return;
    param?.onoutside();
  };
  document.addEventListener('pointerdown', handler, true);
  return {
    update(next) {
      param = next;
    },
    destroy() {
      document.removeEventListener('pointerdown', handler, true);
    },
  };
};

// ---- focus trap ---------------------------------------------------------------------------------

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export const trapFocus: Action<HTMLElement, { initial?: string } | undefined> = (node, p) => {
  const previous = document.activeElement as HTMLElement | null;
  const focusables = () => Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE)).filter((el) => el.offsetParent !== null);
  requestAnimationFrame(() => {
    // Focus the requested element, or the dialog itself (so no control shows a stray focus ring).
    const first = p?.initial ? node.querySelector<HTMLElement>(p.initial) : null;
    if (node.contains(document.activeElement) && document.activeElement !== node) return;
    (first ?? node).focus({ preventScroll: true });
  });
  const keydown = (e: KeyboardEvent) => {
    if (e.key !== 'Tab') return;
    const list = focusables();
    if (!list.length) return;
    const first = list[0];
    const last = list[list.length - 1];
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  };
  node.addEventListener('keydown', keydown);
  return {
    destroy() {
      node.removeEventListener('keydown', keydown);
      if (previous?.isConnected) previous.focus({ preventScroll: true });
    },
  };
};
