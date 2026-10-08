// Transitions that honour "Reduce motion" (the app setting and the OS preference).
import { cubicOut } from 'svelte/easing';
import type { TransitionConfig } from 'svelte/transition';

export const ease = (t: number) => {
  // cubic-bezier(0.2, 0.8, 0.2, 1) is close to an exponential ease-out; cubicOut is a good stand-in
  // with a slightly longer tail.
  return 1 - Math.pow(1 - t, 3.4);
};

export function reduced(): boolean {
  if (typeof document === 'undefined') return false;
  if (document.documentElement.dataset.reduceMotion === 'true') return true;
  return typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;
}

interface RiseParams {
  y?: number;
  x?: number;
  scale?: number;
  duration?: number;
  delay?: number;
  opacity?: number;
}

/** Fade + small translate/scale; the house transition for panels, menus and toasts. */
export function rise(node: Element, { y = 6, x = 0, scale = 1, duration = 220, delay = 0, opacity = 0 }: RiseParams = {}): TransitionConfig {
  if (reduced()) return { duration: 120, css: (t) => `opacity:${t}` };
  const style = getComputedStyle(node);
  const base = style.transform === 'none' ? '' : style.transform;
  return {
    duration,
    delay,
    easing: ease,
    css: (t, u) =>
      `opacity:${opacity + (1 - opacity) * t};transform:${base} translate(${x * u}px, ${y * u}px) scale(${1 - (1 - scale) * u})`,
  };
}

export function fadeQuick(_node: Element, { duration = 160, delay = 0 } = {}): TransitionConfig {
  return { duration: reduced() ? 1 : duration, delay, easing: cubicOut, css: (t) => `opacity:${t}` };
}
