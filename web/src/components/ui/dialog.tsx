"use client";

import { useEffect, useRef, type ReactNode } from "react";
import { cn } from "@/lib/cn";

export interface DialogProps {
  open: boolean;
  onClose: () => void;
  title: ReactNode;
  description?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
  size?: "md" | "lg";
}

/** Native <dialog>: focus trap, Esc to close, and keyboard operation come from the platform. */
export function Dialog({ open, onClose, title, description, children, footer, size = "md" }: DialogProps) {
  const ref = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (open && !el.open) el.showModal();
    if (!open && el.open) el.close();
  }, [open]);

  return (
    <dialog
      ref={ref}
      onClose={onClose}
      onClick={(e) => {
        if (e.target === ref.current) onClose();
      }}
      aria-labelledby="dialog-title"
      className={cn(
        "m-auto w-[calc(100%-2rem)] rounded-[14px] border border-line bg-surface p-0 text-ink shadow-xl backdrop:bg-night/40",
        size === "lg" ? "max-w-3xl" : "max-w-lg",
      )}
    >
      <div className="flex flex-col">
        <div className="border-b border-line px-5 py-4">
          <h2 id="dialog-title" className="font-display text-base font-semibold">
            {title}
          </h2>
          {description ? <p className="mt-1 text-[13px] text-slate">{description}</p> : null}
        </div>
        <div className="px-5 py-4">{children}</div>
        {footer ? <div className="flex justify-end gap-2 border-t border-line px-5 py-3">{footer}</div> : null}
      </div>
    </dialog>
  );
}
