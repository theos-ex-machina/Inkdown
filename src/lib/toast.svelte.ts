export type ToastKind = "info" | "error";

export interface Toast {
  id: number;
  message: string;
  kind: ToastKind;
}

const TOAST_TTL = 4000;
let nextId = 1;

class ToastStore {
  toasts = $state<Toast[]>([]);

  push(message: string, kind: ToastKind = "info") {
    const id = nextId++;
    this.toasts = [...this.toasts, { id, message, kind }];
    setTimeout(() => this.dismiss(id), TOAST_TTL);
  }

  dismiss(id: number) {
    this.toasts = this.toasts.filter((t) => t.id !== id);
  }
}

export const toasts = new ToastStore();

export function pushToast(message: string, kind: ToastKind = "info"): void {
  toasts.push(message, kind);
}
