import type { KeyboardEvent } from 'react';

/** Native modal dialogs make the background inert, but Tab can still leave for browser chrome. */
export function keepModalTabFocus(event: KeyboardEvent<HTMLElement>) {
  if (event.key !== 'Tab') return;
  const dialog = (event.target as HTMLElement).closest<HTMLDialogElement>('dialog[open]');
  if (!dialog) return;
  const controls = [...dialog.querySelectorAll<HTMLElement>('button, a[href], input, select, textarea, summary, [tabindex]')]
    .filter(element => element.tabIndex >= 0 && !element.matches(':disabled, [aria-disabled="true"]') && element.getClientRects().length > 0);
  const first = controls[0]; const last = controls.at(-1);
  if (!first || !last) { event.preventDefault(); dialog.focus(); return; }
  if (event.shiftKey && (document.activeElement === first || document.activeElement === dialog)) {
    event.preventDefault(); last.focus();
  } else if (!event.shiftKey && (document.activeElement === last || document.activeElement === dialog)) {
    event.preventDefault(); first.focus();
  }
}
