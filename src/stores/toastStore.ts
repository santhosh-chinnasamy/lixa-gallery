import { writable } from 'svelte/store';

export interface ToastAction {
  label: string;
  run: () => void;
}

export interface ToastMessage {
  id: number;
  message: string;
  action?: ToastAction;
}

let nextId = 1;

function createToastStore() {
  const { subscribe, update } = writable<ToastMessage[]>([]);

  return {
    subscribe,
    show(message: string, action?: ToastAction) {
      const id = nextId++;
      update((items) => [...items, { id, message, action }].slice(-3));
      if (!action) {
        setTimeout(() => {
          update((items) => items.filter((item) => item.id !== id));
        }, 8000);
      }
      return id;
    },
    dismiss(id: number) {
      update((items) => items.filter((item) => item.id !== id));
    },
    dismissAll() {
      update(() => []);
    },
  };
}

export const toasts = createToastStore();
