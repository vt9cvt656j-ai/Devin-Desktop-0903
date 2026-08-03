import * as AccordionPrimitive from "@radix-ui/react-accordion";
import { Plus } from "lucide-react";
import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

export function Accordion({
  ...props
}: ComponentProps<typeof AccordionPrimitive.Root>) {
  return <AccordionPrimitive.Root data-slot="accordion" {...props} />;
}

export function AccordionItem({
  className,
  ...props
}: ComponentProps<typeof AccordionPrimitive.Item>) {
  return (
    <AccordionPrimitive.Item
      data-slot="accordion-item"
      className={cn(
        "border-b border-border last:border-b-0 transition-colors data-[state=open]:bg-card",
        className,
      )}
      {...props}
    />
  );
}

export function AccordionTrigger({
  className,
  children,
  ...props
}: ComponentProps<typeof AccordionPrimitive.Trigger>) {
  return (
    <AccordionPrimitive.Header className="flex">
      <AccordionPrimitive.Trigger
        data-slot="accordion-trigger"
        className={cn(
          "group flex flex-1 items-start justify-between gap-4 px-5 py-5 text-left text-base font-medium transition-colors outline-none hover:text-muted-foreground focus-visible:ring-2 focus-visible:ring-ring md:px-6 md:text-lg",
          className,
        )}
        {...props}
      >
        {children}
        <Plus
          aria-hidden
          className="mt-1 size-5 shrink-0 text-muted-foreground transition-transform duration-200 group-data-[state=open]:rotate-45"
        />
      </AccordionPrimitive.Trigger>
    </AccordionPrimitive.Header>
  );
}

export function AccordionContent({
  className,
  children,
  ...props
}: ComponentProps<typeof AccordionPrimitive.Content>) {
  return (
    <AccordionPrimitive.Content
      data-slot="accordion-content"
      className="overflow-hidden"
      {...props}
    >
      <div
        className={cn(
          "px-5 pb-6 text-sm leading-relaxed text-muted-foreground md:px-6 md:text-base",
          className,
        )}
      >
        {children}
      </div>
    </AccordionPrimitive.Content>
  );
}
