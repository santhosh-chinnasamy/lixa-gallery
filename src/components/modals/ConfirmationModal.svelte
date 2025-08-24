<script lang="ts">
  import * as Dialog from '$lib/components/ui/dialog';
  import { Button } from '$lib/components/ui/button';
  import { AlertTriangle } from '@lucide/svelte/icons';
  import type { Component } from 'svelte';

  let {
    open = $bindable(false),
    title = 'Confirm Action',
    description = 'Are you sure you want to proceed?',
    confirmText = 'Confirm',
    cancelText = 'Cancel',
    variant = 'destructive',
    icon = AlertTriangle,
    onConfirm = () => {},
    onCancel = () => {}
  }: {
    open: boolean;
    title?: string;
    description?: string;
    confirmText?: string;
    cancelText?: string;
    variant?: 'destructive' | 'default';
    icon?: Component;
    onConfirm?: () => void;
    onCancel?: () => void;
  } = $props();

  function handleConfirm() {
    open = false;
    onConfirm();
  }

  function handleCancel() {
    open = false;
    onCancel();
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Content class="sm:max-w-md">
    <Dialog.Header>
      <Dialog.Title class="flex items-center gap-2">
        {@const IconComponent = icon}
        <IconComponent class="h-5 w-5 {variant === 'destructive' ? 'text-destructive' : 'text-primary'}" />
        {title}
      </Dialog.Title>
      <Dialog.Description class="text-left">
        {description}
      </Dialog.Description>
    </Dialog.Header>
    
    <div class="flex justify-end gap-2 pt-4">
      <Button variant="outline" onclick={handleCancel}>
        {cancelText}
      </Button>
      <Button {variant} onclick={handleConfirm}>
        {confirmText}
      </Button>
    </div>
  </Dialog.Content>
</Dialog.Root>
