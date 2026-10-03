import type { KeyboardEvent } from 'react';

const items = (menu: Element) => [...menu.querySelectorAll<HTMLElement>('[role="menuitem"]')]
  .filter(item => item.closest('[role="menu"]') === menu && !item.matches(':disabled'));

/** Keyboard navigation stays inside the currently visible level of the menu. */
export function applicationMenuKeyDown(event: KeyboardEvent<HTMLElement>) {
  const target = (event.target as HTMLElement).closest<HTMLElement>('button, a');
  if (!target) return;
  const top = [...event.currentTarget.querySelectorAll<HTMLElement>(':scope > .application-menu > button')];
  const menu = target.closest('[role="menu"]');
  const openAndFocus = (trigger: HTMLElement, last = false) => {
    if (trigger.getAttribute('aria-expanded') !== 'true') trigger.click();
    requestAnimationFrame(() => {
      const child = trigger.parentElement?.querySelector('[role="menu"]');
      const choices = child ? items(child) : [];
      (last ? choices.at(-1) : choices[0])?.focus();
    });
  };
  if (!menu && top.includes(target) && ['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(event.key)) {
    event.preventDefault(); openAndFocus(target, event.key === 'ArrowUp'); return;
  }
  if (!menu && top.includes(target) && ['ArrowLeft', 'ArrowRight'].includes(event.key)) {
    event.preventDefault();
    const next = top[(top.indexOf(target) + (event.key === 'ArrowRight' ? 1 : top.length - 1)) % top.length];
    next.focus(); return;
  }
  if (!menu) return;
  const choices = items(menu);
  const index = choices.indexOf(target);
  if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
    event.preventDefault();
    const next = event.key === 'Home' ? choices[0] : event.key === 'End' ? choices.at(-1)
      : choices[(index + (event.key === 'ArrowDown' ? 1 : choices.length - 1)) % choices.length];
    next?.focus();
  } else if (event.key === 'ArrowRight' && target.getAttribute('aria-haspopup') === 'menu') {
    event.preventDefault(); openAndFocus(target);
  } else if (event.key === 'Escape' || (event.key === 'ArrowLeft' && menu.parentElement?.classList.contains('menu-submenu'))) {
    event.preventDefault();
    const trigger = menu.parentElement?.querySelector<HTMLElement>(':scope > button');
    if (trigger?.getAttribute('aria-expanded') === 'true') trigger.click();
    trigger?.focus();
  } else if (event.key === 'Tab') {
    // Restore the top-level button before native Tab moves to its next neighbour.
    const trigger = target.closest('.application-menu')?.querySelector<HTMLElement>(':scope > button');
    if (trigger?.getAttribute('aria-expanded') === 'true') trigger.click();
    trigger?.focus();
  }
}
