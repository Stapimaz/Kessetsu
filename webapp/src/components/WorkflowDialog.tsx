import { useEffect, useRef, type ReactNode } from 'react';
import { CheckCircle2, X } from 'lucide-react';
import './WorkflowDialog.css';

export interface WorkflowStep<T extends string = string> {
  id: T;
  number: number;
  label: string;
  badge?: string;
  done?: boolean;
  disabled?: boolean;
}

interface WorkflowDialogProps<T extends string = string> {
  open: boolean;
  onClose: () => void;
  ariaLabel: string;
  closeLabel: string;
  className?: string;
  icon?: ReactNode;
  title: string;
  subtitle: ReactNode;
  headerExtras?: ReactNode;
  stepperLabel: string;
  steps: WorkflowStep<T>[];
  activeStep: T;
  onStepSelect: (step: T) => void;
  stepperActions?: ReactNode;
  contextBanner?: ReactNode;
  children: ReactNode;
  status?: ReactNode;
  footerActions?: ReactNode;
}

export function WorkflowDialog<T extends string = string>({
  open,
  onClose,
  ariaLabel,
  closeLabel,
  className = '',
  icon,
  title,
  subtitle,
  headerExtras,
  stepperLabel,
  steps,
  activeStep,
  onStepSelect,
  stepperActions,
  contextBanner,
  children,
  status,
  footerActions,
}: WorkflowDialogProps<T>) {
  const dialogRef = useRef<HTMLDialogElement | null>(null);
  const bodyRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    if (!open && dialog.open) dialog.close();
  }, [open]);

  useEffect(() => {
    if (bodyRef.current) {
      bodyRef.current.scrollTop = 0;
    }
  }, [activeStep]);

  return (
    <dialog
      ref={dialogRef}
      className={`app-dialog workflow-dialog ${className}`.trim()}
      aria-label={ariaLabel}
      onClose={onClose}
    >
      <header className="workflow-header">
        <div className="workflow-header-copy">
          <h2>
            {icon}
            <span>{title}</span>
          </h2>
          <p>{subtitle}</p>
          {headerExtras}
        </div>
        <button type="button" className="workflow-close" aria-label={closeLabel} onClick={onClose}>
          <X size={16} />
        </button>
      </header>

      <nav className="workflow-stepper" aria-label={stepperLabel}>
        <div className="workflow-steps-list">
          {steps.map((step) => {
            const active = step.id === activeStep;
            return (
              <button
                key={step.id}
                type="button"
                className={`workflow-step ${active ? 'is-active' : ''} ${step.done ? 'is-done' : ''}`.trim()}
                aria-pressed={active}
                aria-current={active ? 'step' : undefined}
                disabled={step.disabled}
                onClick={() => onStepSelect(step.id)}
              >
                <span className="workflow-step-num" aria-hidden="true">
                  {step.done && !active ? <CheckCircle2 size={13} /> : step.number}
                </span>
                <span className="workflow-step-label">{step.label}</span>
                {step.badge && <small className="workflow-step-badge">{step.badge}</small>}
              </button>
            );
          })}
        </div>
        {stepperActions && <div className="workflow-stepper-actions">{stepperActions}</div>}
      </nav>

      {contextBanner && <div className="workflow-context-slot">{contextBanner}</div>}

      <div ref={bodyRef} className="workflow-body">
        {children}
      </div>

      <footer className="workflow-footer">
        <p role="status" className="workflow-status">
          {status}
        </p>
        {footerActions && <div className="workflow-footer-actions">{footerActions}</div>}
      </footer>
    </dialog>
  );
}

