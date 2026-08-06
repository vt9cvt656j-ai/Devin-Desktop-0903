import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  // `cursor-pointer` is explicit because Tailwind v4 removed it from the preflight for
  // <button>, so every button in this console was showing a plain arrow — the one cue
  // that says "this is not just text". `disabled:cursor-not-allowed` is the other half.
  "inline-flex cursor-pointer items-center justify-center gap-2 whitespace-nowrap rounded-lg text-sm font-medium transition-all duration-200 outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4",
  {
    variants: {
      variant: {
        default:
          "bg-primary text-primary-foreground hover:bg-primary/90 hover:scale-[1.02] active:scale-[0.98] shadow-sm",
        secondary:
          "bg-secondary text-secondary-foreground hover:bg-accent active:scale-[0.98]",
        outline:
          "border border-input bg-card text-foreground hover:bg-secondary active:scale-[0.98]",
        ghost: "text-foreground hover:bg-secondary active:scale-[0.98]",
        link: "text-foreground underline-offset-4 hover:underline",
        inverse:
          "bg-card text-foreground hover:bg-background hover:scale-[1.02] active:scale-[0.98] shadow-sm",
      },
      size: {
        sm: "h-9 px-3.5",
        md: "h-11 px-5",
        lg: "h-12 px-7 text-base",
        icon: "size-11",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "md",
    },
  },
);

type ButtonProps = ComponentProps<"button"> &
  VariantProps<typeof buttonVariants> & {
    asChild?: boolean;
  };

export function Button({
  className,
  variant,
  size,
  asChild = false,
  ...props
}: ButtonProps) {
  const Component = asChild ? Slot : "button";

  return (
    <Component
      data-slot="button"
      className={cn(buttonVariants({ variant, size }), className)}
      {...props}
    />
  );
}

export { buttonVariants };
