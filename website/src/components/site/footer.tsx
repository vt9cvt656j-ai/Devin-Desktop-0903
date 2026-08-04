import { Separator } from "@/components/ui/separator";

const columns = [
  {
    heading: "Product",
    links: [
      { label: "Overview", href: "#features" },
      { label: "How it works", href: "#architecture" },
      { label: "Extensibility", href: "#extensions" },
      { label: "Customers", href: "#customers" },
    ],
  },
  {
    heading: "Resources",
    links: [
      { label: "Writing an extension", href: "https://github.com/fendoushaonian/Devin-Desktop" },
      { label: "GitHub", href: "https://github.com/fendoushaonian/Devin-Desktop" },
      { label: "Releases", href: "https://github.com/fendoushaonian/Devin-Desktop/releases" },
    ],
  },
  {
    heading: "Company",
    links: [
      { label: "Devin Desktop", href: "https://github.com/fendoushaonian/Devin-Desktop" },
      { label: "License", href: "https://github.com/fendoushaonian/Devin-Desktop" },
    ],
  },
];

export function Footer() {
  return (
    <footer className="border-t border-border bg-background py-14">
      <div className="mx-auto max-w-6xl px-4 sm:px-6">
        <div className="flex flex-col justify-between gap-10 sm:flex-row">
          <div className="max-w-xs">
            <p className="flex items-center gap-2.5 font-display text-lg font-semibold">
              <img src="/logo.png" alt="" className="size-8" />
              Mr.day One
            </p>
            <p className="mt-3 text-sm text-muted-foreground">
              A native desktop code editor for macOS and Windows, with an agent that verifies its
              own work.
            </p>
          </div>
          <div className="grid grid-cols-2 gap-10 sm:grid-cols-3">
            {columns.map((column) => (
              <div key={column.heading}>
                <p className="type-eyebrow mb-4">{column.heading}</p>
                <ul className="space-y-2.5">
                  {column.links.map((link) => (
                    <li key={link.label}>
                      <a
                        href={link.href}
                        className="text-sm text-muted-foreground transition-colors hover:text-foreground"
                        {...(link.href.startsWith("http")
                          ? { target: "_blank", rel: "noreferrer" }
                          : {})}
                      >
                        {link.label}
                      </a>
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        </div>
        <Separator className="my-10" />
        <p className="text-sm text-muted-foreground">
          © {new Date().getFullYear()} Mr.day One. Companion to Devin Desktop.
        </p>
      </div>
    </footer>
  );
}
