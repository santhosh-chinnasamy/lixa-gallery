import type { WithElementRef } from 'bits-ui';
import type {
	HTMLAnchorAttributes,
	HTMLButtonAttributes,
} from 'svelte/elements';
import { type VariantProps, tv } from 'tailwind-variants';
import Root from './button.svelte';
// refactored based on this github issue: https://github.com/huntabyte/shadcn-svelte/issues/1468#issuecomment-2465979183
const buttonVariants = tv({
  base: 'focus-visible:ring-ring inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0',
  variants: {
    variant: {
      default: 'bg-primary text-primary-foreground hover:bg-primary/90 shadow',
      destructive:
        'bg-destructive text-destructive-foreground hover:bg-destructive/90 shadow-sm',
      outline:
        'border-input bg-background hover:bg-accent hover:text-accent-foreground border shadow-sm',
      secondary:
        'bg-secondary text-secondary-foreground hover:bg-secondary/80 shadow-sm',
      ghost: 'hover:bg-accent hover:text-accent-foreground',
      link: 'text-primary underline-offset-4 hover:underline',
    },
    size: {
      default: 'h-9 px-4 py-2',
      sm: 'h-8 rounded-md px-3 text-xs',
      lg: 'h-10 rounded-md px-8',
      icon: 'h-9 w-9',
    },
  },
  defaultVariants: {
    variant: 'default',
    size: 'default',
  },
});

type ButtonVariant = VariantProps<typeof buttonVariants>['variant'];
type ButtonSize = VariantProps<typeof buttonVariants>['size'];

type ButtonProps = WithElementRef<HTMLButtonAttributes> &
  WithElementRef<HTMLAnchorAttributes> & {
    variant?: ButtonVariant;
    size?: ButtonSize;
  };

export {
	//
	Root as Button, Root, buttonVariants,
	type ButtonProps,
	type ButtonSize,
	type ButtonVariant, type ButtonProps as Props
};

