import tsIcon from "devicon/icons/typescript/typescript-original.svg";
import rustIcon from "devicon/icons/rust/rust-original.svg";
import pyIcon from "devicon/icons/python/python-original.svg";
import goIcon from "devicon/icons/go/go-original.svg";
import bashIcon from "devicon/icons/bash/bash-original.svg";
import htmlIcon from "devicon/icons/html5/html5-original.svg";
import swiftIcon from "devicon/icons/swift/swift-original.svg";
import kotlinIcon from "devicon/icons/kotlin/kotlin-original.svg";
import rubyIcon from "devicon/icons/ruby/ruby-original.svg";
import cppIcon from "devicon/icons/cplusplus/cplusplus-original.svg";

/*
 * 语言走马灯：logo 墙卡片化（composition-repertoire #motion-effects marquee，
 * 悬停暂停 + reduced-motion 静止在 index.css）。图标为 devicon 官方 SVG。
 */
const languages = [
  { name: "TypeScript", icon: tsIcon },
  { name: "Rust", icon: rustIcon },
  { name: "Python", icon: pyIcon },
  { name: "Go", icon: goIcon },
  { name: "Shell", icon: bashIcon },
  { name: "HTML", icon: htmlIcon },
  { name: "Swift", icon: swiftIcon },
  { name: "Kotlin", icon: kotlinIcon },
  { name: "Ruby", icon: rubyIcon },
  { name: "C / C++", icon: cppIcon },
];

function Track() {
  return (
    <>
      {languages.map((lang) => (
        <span
          key={lang.name}
          className="mx-2.5 flex items-center gap-2.5 rounded-xl border border-border bg-card px-4 py-2.5 text-sm font-medium text-foreground shadow-sm"
        >
          <img src={lang.icon} alt="" className="size-5" loading="lazy" />
          {lang.name}
        </span>
      ))}
    </>
  );
}

export function LanguageMarquee() {
  return (
    <section className="border-t border-border bg-background py-10">
      <p className="type-eyebrow mb-6 text-center">
        Syntax highlighting for every language you write
      </p>
      <div className="marquee-mask overflow-hidden py-1">
        <div className="marquee-track flex w-max animate-marquee">
          <Track />
          <div aria-hidden className="contents">
            <Track />
          </div>
        </div>
      </div>
    </section>
  );
}
