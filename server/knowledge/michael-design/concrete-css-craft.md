# Michael Design Library — concrete-css-craft

Concrete CSS craft (具体做法层)：把数值层组合出质感的可复制写法。数值本身在提示词的 design_tokens 里，配色来自本库其他蓝图；这里只回答「同样的 token，为什么别人写出来有质感」。

## ⓪ 真值来源（最先读）：**从知识库取蓝本，别凭记忆编色** [sections/source-of-truth]

- **`knowledge_search(domain="michael-design", query="<品类> <区块>")`** → 441 份可直接落地的
  Tailwind 蓝图，带真实色板、排版构图、动效配方与组件覆盖。**配色和构图一律以命中的蓝图为准。**
- 下面这套 `:root` 是**数值层**：字体族、字号阶、4px 间距网格、圆角、阴影、动效时长——这些
  是知识库里没有的具体数字，照抄即可；但**颜色**要换成本轮从 michael-design 命中的那一套。
- 项目已经装了 Tailwind 就直接用类名与官方色阶，别另起一套 hex 变量。

## ① 用色铁律：**配色一律从 Tailwind 调色板取** [sections/colour-discipline]

- 项目装了 Tailwind → **直接用类名**：`bg-zinc-950 text-zinc-100 border-white/10 bg-emerald-500 hover:bg-emerald-600 text-emerald-400`；透明度档用 `/20 /30 /50`（如 `bg-emerald-500/20` 做淡色块，比自己调 hex 一致得多）。
- **别自己编一堆 `#xxxxxx` 自定义 CSS 变量当调色板** —— 那不是 Tailwind 项目的写法，正是你"没走 Tailwind 调色板"的根源。
- **下面所有具体 hex（近黑 `#010102`、卡底 `#0f1011`、v0 配方里的 emerald 强调示例 `#10b981` 等）都只是"技法数值示范"，照抄时一律换成 Tailwind 调色板对应档**：近黑=`neutral-950`/`zinc-950`、卡底=`zinc-900`、hairline=`border-white/10`、正文=`text-zinc-100`、强调=**你从 Tailwind 22 族里选的那一族**（见「配色怎么选」；`#10b981` 只是 emerald-500 的示例，换成你选的族）。⛔ **强调色别照搬任何品牌的固定值**（Linear 的紫 `#5e6ad2`、Vercel 的蓝 `#0072f5` 都是人家品牌色，别搬）——从 Tailwind 调色板选你自己的族，尤其别用蓝/靛紫。
- shadcn 项目：把选定的 Tailwind 族值填进 shadcn 的 `--background/--foreground/--primary` 变量（浅色 --primary=强调-600、深色=强调-500），组件用 `bg-primary/text-muted` 语义类；纯 Tailwind 项目直接用调色板类名。

## 配色怎么选（Tailwind 调色板——从这里选，别随手编 hex） [sections/tailwind-palette-selection]

项目装了 Tailwind，直接用类名 `bg-{族}-{档}`（如 `bg-zinc-900`/`text-emerald-600`）；上面 CSS 变量的 hex 也从这套调色板取。**调色板 = 22 个色族 × 11 档明度**（50 最浅 → 500 基准 → 950 最深）。
**配色公式（记死）：选 1 个中性色族做灰阶骨架 + 1 个强调色族做品牌主色，全站只用这两族的不同档位——自然干净、绝不花。**
- **中性族（灰阶骨架，选一个）**：`slate`(冷灰带蓝·科技/SaaS 首选) · `zinc`(冷中性·shadcn 默认) · `neutral`(最中性) · `gray`(纯正) · `stone`(暖灰·内容/杂志/暖品牌)。
- **强调族（品牌主色，按品类选一个）**：冷/科技 `teal`/`emerald`(墨绿·金融健康)/`cyan`/`sky`；暖/活力 `orange`/`amber`(琥珀)/`rose`(玫红)/`red`；文艺 `green`/`pink`/`fuchsia`。**默认逃生色别碰**：`blue`/`indigo`/`violet`/`purple`(蓝紫=最重 AI 味)；`lime`/霓虹绿慎用(第二 AI 味)。除非用户点名。
- **档位纪律（同一元素别跳档乱用）**：大面积背景=浅 50/100 或深 900/950；正文字=700-900；次要文字=中性 500；主色按钮=强调 500/600、hover 深一档到 600/700；边框=中性 200(浅)/800(深)；卡片底=白 或 中性 900。
- **映射到 shadcn/CSS 变量**：浅色 `--background:white、--foreground:中性-900、--muted:中性-100、--muted-foreground:中性-500、--border:中性-200、--primary:强调-600`；深色(.dark) `--background:中性-950、--foreground:中性-50、--muted:中性-800、--border:中性-800、--primary:强调-500`。深浅两档主色用**同一个强调族**、只换明度档。
- **抄不会丑的成品组合**：科技 SaaS `zinc + emerald` 或 `slate + teal`/`slate + sky`；暖内容 `stone + orange`/`stone + amber`；干净商务 `slate + teal`/`neutral + rose`；极简高级 `neutral + 一个低饱和强调 + 大留白`。**选完全站贯彻，别中途换族。**

## 绝对禁止（违反任何一条 = 不合格） [sections/hard-prohibitions]

- ❌ **禁止写死颜色**：`color: #333` → ✅ `color: var(--text)`
- ❌ **禁止写死间距**：`padding: 15px` → ✅ `padding: var(--sp-4)`（用最接近的变量）
- ❌ **禁止写死圆角**：`border-radius: 5px` → ✅ `border-radius: var(--radius-sm)`
- ❌ **禁止用 emoji 当图标**：✅ ❌ ⚠️ 这些 → 用 SVG 图标
- ❌ **禁止只做 default 态**：每个按钮/输入框/链接必须有 hover + focus-visible + disabled 态
- ❌ **禁止纯黑 #000 文字 / 纯白 #fff 背景**：用 --text / --bg
- ❌ **禁止 transition: all**：指定具体属性 `transition: background var(--duration) var(--ease)`
- ❌ **禁止 box-shadow: 0 0 10px black` 这种重阴影**：用 --shadow-sm / --shadow-md
- ❌ **禁止没有 line-height**：正文 line-height: var(--leading-normal)，标题 var(--leading-tight)
- ❌ **禁止文字满屏宽**：文本容器加 max-width: 65ch
- ❌ **禁止交付通篇纯文字+色块、一张图都没有的展示页**：落地页/官网/产品页/作品集必须有真图（hero 大图 + 卡片配图），图是布局的内容锚点
- ❌ **禁止图片不套 aspect-ratio + object-fit**：`<img>` 裸放会撑破布局/拉伸变形 → 一律用 `.media` 容器锁比例
- ❌ **禁止写 `source.unsplash.com`**（已停用必 404）→ 用 `picsum.photos/seed/…` 或 `placehold.co` 或 `generate_image`
- ❌ **禁止靛紫/蓝当默认主色**（`#4f46e5`/`#6366f1`/紫渐变 = 最重 AI 味）→ 主动为这个产品选一个克制主色，别躺进蓝紫
- ❌ **禁止纯黑底 + 霓虹/酸绿/荧光强调色**（赛博朋克烂大街 = 第二个 AI 逃生色，一样一眼假）→ 暗色用带微妙色偏的深灰(非纯 #000)、强调色克制有质感
- ❌ **禁止没指定颜色就单方面拍板一个大胆/暗色/霓虹配色闷头码** → 先出 2-3 个配色方向让用户挑（style_wardrobe），选定再落
- ❌ **禁止 lorem ipsum / `Feature one` 占位文案**：给真实产品名、真实感描述、拟真数字（`2,400+ teams`/`99.9% uptime`）

## 自查口诀（写完 CSS 对着过一遍） [sections/self-check]

颜色用变量、间距用变量、圆角用变量、阴影用变量、所有交互元素有 hover+focus、文字对比度 ≥ 4.5:1、正文 line-height 1.5+、触摸目标 ≥ 44px、布局用 flex/grid 不用 float、transition 指定属性不用 all。

## 暗色页做到 v0 级（照抄这些配方——扁平灰卡 + 彩虹装饰 + 彩色方块占位 = 一眼 AI，下面把这几个 tell 从根上干掉） [sections/dark-surface-craft]

**做暗色落地页/SaaS 页时，卡片/配色/mockup/深度一律照下面的确切数值来，别自己瞎写。**

## ① 卡片质感（surface 阶梯 + hairline + 顶边内高光 + 分层阴影，不靠厚 drop shadow） [sections/card-surface-craft]

```css
/* ============ 高级暗色卡片 / Bento 完整配方(照抄,别改数值) ============ */
/* 1) 先钉 surface 阶梯 token —— 深度靠逐级变亮浮起,不靠阴影 */
:root{
  --canvas:#010102;      /* 页底,带一丝蓝的近黑,禁 #000 */
  --surface-1:#0f1011;   /* 卡片默认底 */
  --surface-2:#141516;   /* hover / raised */
  --surface-3:#18191a;   /* dropdown / 更靠前的层 */
  --hairline:#23252a;         /* 1px 边框标准档,禁 #333 灰实线 */
  --hairline-strong:#34343a;  /* 强边档 */
  --ink:#f7f8f8;              /* 正文,禁 #fff */
  --ink-muted:#8a8f98;        /* 次要文字 */
  --accent:#10b981;           /* 示例=Tailwind emerald-500，换成你从调色板选的那一族(别用紫) */
  --r-md:8px; --r-lg:12px; --r-xl:16px;  /* 卡片只用 8-12,禁 16+ */
}

/* 2) 标准 premium 卡:hairline 边 + 顶边内高光 + 微妙背景渐变 + 分层阴影栈 */
.card{
  position:relative;
  border-radius:var(--r-lg);              /* 12px,别 16/20 一把梭 */
  padding:24px;                           /* base-4:feature=24 / testimonial=32 / CTA=48 */
  background:
    linear-gradient(180deg, rgba(255,255,255,.03), transparent 40%),  /* 近乎不可察觉的方向光 */
    var(--surface-1);
  border:1px solid var(--hairline);
  box-shadow:
    inset 0 1px 0 rgba(255,255,255,.06),   /* 顶边内高光 = 接住光,关键的一条 */
    0 1px 2px rgba(0,0,0,.30),             /* 紧贴 ambient */
    0 8px 24px -12px rgba(0,0,0,.50);      /* 宽软抬升,层层极淡才高级 */
  transition:transform .25s ease, box-shadow .25s ease, border-color .25s ease;
                                           /* 只 transition 这三个,禁 transition:all */
}
.card:hover{
  transform:translateY(-3px);             /* 空间抬升 -2~-4px,GPU 友好 */
  border-color:rgba(255,255,255,.12);     /* hairline 透明度 .06→.12 被照亮 */
  box-shadow:
    inset 0 1px 0 rgba(255,255,255,.08),
    0 2px 4px rgba(0,0,0,.30),
    0 16px 40px -12px rgba(0,0,0,.60);     /* elevation 升一级:blur/offset 加大 */
}

/* 3) Vercel 手法:shadow-as-border(inset 环当 1px 边,不占 box model,圆角不裁切) */
.card--vercel{
  border-radius:var(--r-lg); background:var(--surface-1);
  box-shadow:0 0 0 1px rgba(0,0,0,.078) inset;                        /* rest 级的"边" */
}
.card--vercel:hover{
  box-shadow:0 0 0 1px rgba(0,0,0,.078) inset,
             0 2px 2px rgba(0,0,0,.039),
             0 8px 16px -4px rgba(0,0,0,.039);                        /* float 级 */
}

/* 4) 渐变描边(linear-gradient 不能直接用在 border 上时的正解:padding+mask+exclude) */
.gradient-border{ position:relative; border-radius:var(--r-lg); background:rgba(255,255,255,.03); }
.gradient-border::before{
  content:''; position:absolute; inset:0; z-index:-1; border-radius:inherit; padding:1px;
  background:linear-gradient(to bottom right,#171717 0%,#525252 62%,#171717 100%); /* 左上亮右下暗 */
  -webkit-mask:linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
          mask:linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  -webkit-mask-composite:xor; mask-composite:exclude;                /* 挖空中间只留 1px 边 */
}

/* 5) Spotlight 跟随鼠标辉光(JS 只更新两个变量,CSS 画光球,渲染交给 compositor) */
.spotlight-card{ position:relative; overflow:hidden; border-radius:var(--r-lg);
  background:var(--surface-1); border:1px solid var(--hairline); }
.spotlight-card::before{ content:''; position:absolute; inset:0; border-radius:inherit;
  opacity:0; transition:opacity .4s ease;
  background:radial-gradient(320px circle at var(--mx) var(--my),
            rgba(16,185,129,.15), transparent 40%); }              /* 用品牌色低透明,禁 neon */
.spotlight-card:hover::before{ opacity:1; }
/* JS: c.onmousemove=e=>{const r=c.getBoundingClientRect();
   c.style.setProperty('--mx',(e.clientX-r.left)+'px');
   c.style.setProperty('--my',(e.clientY-r.top)+'px');}; */

/* 6) 大标题必收负字距(display 尺寸不收=一眼松垮),Vercel/Linear 实测量级 */
.display-xl{font-size:48px; letter-spacing:-2.4px; line-height:1.1; font-weight:600;}
.display-lg{font-size:32px; letter-spacing:-1.28px; line-height:1.1;}
.card-title{font-size:22px; letter-spacing:-.4px;}
.body{font-size:16px; letter-spacing:-.05px; line-height:1.6;}  /* eyebrow 小标签才用 +0.4px 正字距 */

/* 硬禁清单:纯黑 #000 底 / 纯白 #fff 字 / border:1px solid #333 灰实线 /
   厚 drop shadow(0 10px 30px rgba(0,0,0,.5)一条闷)/ 16px+ 圆角上小元素 /
   backdrop-blur 20~40px 每张卡都糊(玻璃只给 modal/dropdown/nav 且 blur:5px)/
   padding 随手 15/24/40(走 base-4:4/8/16/32/64)/ 大标题 letter-spacing:0 */
```

## ② 色彩克制（一个强调色的透明度/明度档做层次，物理禁彩虹） [sections/single-accent-restraint]

```css
/* ============ 色彩克制硬规则:装饰禁彩虹,只用强调色的透明度/明度档做层次 ============ */
/* 铁律 60-30-10:60% 中性阶(gray/surface) + 30% 主数据/强调色 + 10% 点睛。
   禁"5 个颜色各占 20% 平均分布"。强调色是稀缺资源,不是"丰富"工具。 */

/* 只钉一个强调色 + 中性阶,装饰物理上只能在这套里取值 */
:root{
  --accent:#10b981;              /* 示例=Tailwind emerald-500；换成你选的族。Vercel 用蓝、Linear 用紫都是它们品牌色，你别照搬 */
  --accent-hover:#828fff; --accent-focus:#5e69d1;
  --gray-300:#e5e5e5; --gray-600:#8f8f8f; --gray-900:#171717; --gray-1000:#0a0a0a;
}

/* ✅ 正解:一个 hue 换透明度/明度做层次(图标背景、图表分类、bento 分区、swatch 全用这个) */
.tone-1{ background:var(--accent); }                       /* 100% */
.tone-2{ background:rgb(16 185 129 / .55); }               /* 同色 55% 透明 */
.tone-3{ background:rgb(16 185 129 / .30); }               /* 30% */
.tone-4{ background:rgb(16 185 129 / .12); }               /* 12% 极淡背景块 */
/* 需要明度阶而非透明度时用 color-mix(比手挑多 hue 一致得多) */
:root{
  --accent-90: color-mix(in srgb, var(--accent) 90%, black);
  --accent-60: color-mix(in srgb, var(--accent) 60%, transparent);
  --accent-tint: color-mix(in srgb, var(--accent) 8%, var(--surface-1)); /* hero/卡底极淡染 */
}

/* ✅ 图表/图示:Gray-plus-Accent —— 要讲的那条给强调色,其余全部灰,不给每条一个 hue */
.series{ stroke:var(--gray-600); fill:var(--gray-300); opacity:.7; }
.series--highlight{ stroke:var(--accent); fill:var(--accent); opacity:1; }
/* 大面积降饱和、小面积才提饱和(FT/538):area fill rgb(16 185 129/.15),line stroke 满饱和 2px */

/* ✅ 语义色仅此 4 个,与装饰调色板物理隔离 */
:root{ --status-success:#27a644; --status-warning:#d99e00; --status-error:#e5484d; --status-info:var(--accent); }
/* 规则:装饰元素禁引用 --status-*;状态元素禁用装饰 hue。成功绿/错误红只表真状态,永不做装饰。 */

/* ❌ 头号 AI tell 全禁:
   - 图标/图表/mockup/swatch 每个换一个 hue(橙 #f97316 + 黄 #eab308 + 青 #06b6d4 + 绿 #22c55e + 粉轮转)
   - indigo/violet/purple 当主色 + from-indigo-500 to-purple-600 白底紫渐变(Tailwind 默认 = 满屏 AI 紫)
   - 满饱和铺大面积(bar/area 用 100% saturation)
   - 不可命名的中间怪 hue(似橙非橙、蓝绿之间)—— 颜色必须 nameable(红/蓝/橙)
   - 只换 HSL 的 H、L 不变 → 色盲不可分
   - 分类 >8 线 / >5 饼块还硬给独立 hue(超限就分组/换图表/高亮单条+其余置灰)
   焦点态:outline:2px solid rgb(16 185 129 / .5); outline-offset:2px;(禁彩色 glow) */

/* 喂 AI 的负向约束(直接进 prompt):no purple gradients、no neon、只用 token 调色板、
   一个强调色其余中性、装饰不引入新 hue、装饰与语义色互不越界。 */
```

## ③ mockup 真实感（真 chrome + 真内容，禁彩色 blob/圆点/灰空盒占位） [sections/mockup-realism]

```css
/* ============ mockup 硬规则:禁彩色 blob/圆点/灰空盒占位,做真实感 chrome + 真内容 ============ */
/* 铁律:前景"产品预览"必须是真截图或像素级真实的假 UI(真按钮真列表真表单)。
   彩色实心 blob / 圆点 / 圆角矩形 / emoji-style 简笔图 / bg-gray-100 h-96 空 div —— 全禁当主体,
   blob 只配当背景光晕层(见 depthRule),绝不当内容。 */

/* 真实感浏览器 chrome:44px 栏 + 12px 低饱和交通灯 + URL 药丸,里面塞真截图/真渲染 UI */
.browser{ border-radius:12px 12px 0 0; overflow:hidden;
  border:1px solid var(--hairline); background:var(--surface-2); }
.browser__bar{ height:44px; display:flex; align-items:center; gap:8px; padding:0 14px;
  background:var(--surface-3); border-bottom:1px solid var(--hairline); }
.dot{ width:12px; height:12px; border-radius:50%;
  box-shadow:inset 0 -1px 0 rgba(0,0,0,.25); }   /* 边缘内暗化 = 更真,禁完美纯饱和正圆 */
.dot--r{ background:#ff5f57; } .dot--y{ background:#febc2e; } .dot--g{ background:#28c840; }
/* 灯用 macOS 真值(低饱和),禁 red-500/yellow-500/green-500 满饱和 */
.browser__url{ flex:1; height:24px; margin-left:8px; border-radius:9999px;
  background:rgba(255,255,255,.04); border:1px solid var(--hairline); }

/* 产品截图别裸放:塞进 16px 圆角瓦片 + 24px 外 padding + 顶边内高光(Linear 确切做法) */
.shot-tile{ padding:24px; border-radius:16px; background:var(--surface-1); border:1px solid var(--hairline); }
.shot-tile img{ display:block; width:100%; border-radius:12px;
  box-shadow:inset 0 1px 0 rgba(255,255,255,.08), 0 20px 60px rgba(0,0,0,.5); }  /* 光从上打 + 落影 */

/* 没有真截图时的正解(按优先级):
   1) 用 generate_image 出一张真实产品 UI 图,填进 .browser / .shot-tile;
   2) 或手写像素级真实的假 UI(真侧边栏 + 真列表行 + 真按钮 + 真表单控件),用同一套 surface/hairline token;
   3) 绝不接受:纯色块、圆点阵、"Your Screenshot Here"灰盒、emoji 拼贴。 */

/* ❌ 禁清单:
   - bg-gray-100 / h-96 空 div 直接当截图(Tailwind 教程原始 demo,搬来不填 = 明显占位)
   - 彩色 blob / 渐变球当"产品"主体
   - 交通灯纯饱和 + 完美对齐正圆(真 macOS 灯偏低饱和 + 边缘暗化)
   - 标题栏死灰 #333(应贴近 UI 用 surface-3) */
```

## ④ 深度氛围（近黑非纯黑 + 噪点 + 径向光晕，不给内容外发光） [sections/depth-and-atmosphere]

```css
/* ============ 深度 / 质感硬规则:surface lift + 噪点 + 径向辉光,不靠元素外发光 ============ */
/* 铁律:暗色深度靠 surface 阶梯 + hairline + 顶边白内高光 + 独立光晕层 + 整页噪点,
   越靠近用户的面越亮(和亮色相反)。禁给文字/内容直接 text-shadow/box-shadow 外发光当氛围。 */

/* 1) 近黑画布(禁纯黑)*/
body{ background:var(--canvas); color:var(--ink); }   /* --canvas:#010102 带一丝冷调 */

/* 2) 整页噪点 grain 叠层 —— 盖渐变 banding + 加质感(核心,SVG feTurbulence data-URI)*/
.page::before{
  content:""; position:fixed; inset:0; z-index:0; pointer-events:none; opacity:.10;
  background-image:url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 600 600'%3E%3Cfilter id='a'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23a)'/%3E%3C/svg%3E");
  background-size:182px; background-repeat:repeat;
}
.page>*{ position:relative; z-index:1; }   /* 内容抬到噪点之上,opacity 一定压到 .10-.12 */

/* 3) 氛围光晕:绝对定位大圆 + 巨 blur,垫在 hero 背后(不是给内容加光)*/
.glow{ position:absolute; width:800px; height:800px; border-radius:50%; filter:blur(100px);
  background:radial-gradient(circle at 50% 50%, rgba(16,185,129,.55), rgba(16,185,129,0) 70%);
  opacity:.35; pointer-events:none; }
/* mesh 进阶:多个不同位不同色 radial 叠加再整体 blur(80px);color 只在强调色族内取,禁彩虹 mesh */

/* 4) Linear 式深度:surface 阶梯 + 1px hairline + 顶边白内高光,不用外阴影 */
.panel{ background:var(--surface-1); border:1px solid var(--hairline); border-radius:12px;
  box-shadow:inset 0 1px 0 rgba(255,255,255,.06); }   /* 顶边这道白线 = 光从上打 / 像素渲染感 */

/* 5) 多层阴影升起(真实光有多层叠影,不是一条 0 4px 6px 一把梭)*/
.card-elev{ box-shadow:
  0 1px 1px rgba(0,0,0,.5),      /* 紧贴 ambient */
  0 2px 4px rgba(0,0,0,.4),
  0 8px 24px rgba(0,0,0,.35),    /* 宽软 drop */
  inset 0 1px 0 rgba(255,255,255,.06); }   /* 顶边高光 */

/* 6) 玻璃只给真正浮层(modal/dropdown/nav),blur 小,深度靠 inset 顶高光不靠外阴影 */
.glass{ background:rgba(255,255,255,.06); backdrop-filter:blur(5px); -webkit-backdrop-filter:blur(5px);
  border:1px solid rgba(255,255,255,.10); border-radius:16px;
  box-shadow:inset 0 1px 0 rgba(255,255,255,.10), 0 24px 48px -12px rgba(0,0,0,.6); }
/* 禁:每张卡都玻璃 + blur 20~40px 把背景糊成汤 */

/* ❌ 禁:纯黑死平无颗粒无光晕无 surface 分层 / banding 不处理 /
   内容元素直接 text-shadow 发光当氛围 / 所有卡一条 0 4px 6px */
```

## 第二步：组件照这些模板写（可复制后改细节） [sections/component-templates]

**按钮：**
```css
.btn { display: inline-flex; align-items: center; gap: var(--sp-2); padding: var(--sp-2) var(--sp-4); font-size: var(--text-sm); font-weight: 500; border-radius: var(--radius-md); border: 1px solid transparent; cursor: pointer; transition: background var(--duration) var(--ease), border-color var(--duration) var(--ease), color var(--duration) var(--ease); min-height: 36px; }
.btn-primary { background: var(--primary); color: #fff; }
.btn-primary:hover { background: var(--primary-hover); }
.btn:disabled { opacity: 0.5; cursor: not-allowed; }
.btn:focus-visible { outline: 2px solid var(--primary); outline-offset: 2px; }
```
**输入框：**
```css
.input { width: 100%; padding: var(--sp-2) var(--sp-3); font-size: var(--text-sm); border: 1px solid var(--border); border-radius: var(--radius-md); background: var(--bg); color: var(--text); transition: border-color var(--duration) var(--ease); min-height: 36px; }
.input:focus { border-color: var(--primary); outline: none; box-shadow: 0 0 0 3px rgb(37 99 235 / 0.15); }
.input::placeholder { color: var(--text-faint); }
```
**卡片：**
```css
.card { background: var(--bg); border: 1px solid var(--border); border-radius: var(--radius-lg); padding: var(--sp-6); box-shadow: var(--shadow-sm); }
.card:hover { box-shadow: var(--shadow-md); transform: translateY(-1px); transition: box-shadow var(--duration) var(--ease), transform var(--duration) var(--ease); }
```

**图片进布局（默认交付带真图的成品，不是纯文字+色块的干瘪页）：**
图源三级：① 定制图 `generate_image({prompt:"…", dest:"public/hero.jpg", width:1536, height:1024})`（按真实版位传 w/h，存 public/ 再 `<img src="/hero.jpg">`）→ ② 真实感占位 `https://picsum.photos/seed/{名}/1600/900`（**必带 `/seed/`** 跨刷新稳定）→ ③ 纯占位 `https://placehold.co/1200x600`。**绝对禁 `source.unsplash.com`（已停用必 404）**。
```css
img { max-width: 100%; height: auto; display: block; }               /* 永不撑破布局 */
.media { width: 100%; aspect-ratio: 16/9; object-fit: cover; border-radius: var(--radius-lg); background: var(--surface); } /* 锁比例防塌陷，cover 填满不变形；也可 1/1、4/3 */
/* HERO 全出血 + 渐变压字（任意图上白字都可读） */
.hero { position: relative; min-height: 60vh; overflow: hidden; }
.hero > img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
.hero::after { content: ""; position: absolute; inset: 0; background: linear-gradient(to top, rgb(0 0 0 / .85) 0%, rgb(0 0 0 / .4) 40%, transparent 100%); }
.hero__content { position: relative; z-index: 1; color: #fff; }
/* BENTO 网格：焦点格跨行列（SaaS 功能区首选） */
.bento { display: grid; gap: var(--sp-4); grid-template-columns: repeat(4, 1fr); grid-auto-rows: 200px; }
.bento .feature { grid-column: span 2; grid-row: span 2; }
.bento img { width: 100%; height: 100%; object-fit: cover; border-radius: var(--radius-md); }
/* 自适应图片卡片网格（无媒体查询自动换列） */
.card-grid { display: grid; gap: var(--sp-6); grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); }
/* SPLIT 左右分栏 图+文 */
.split { display: grid; grid-template-columns: 1fr 1fr; align-items: center; gap: var(--sp-8); }
@media (max-width: 768px) { .split { grid-template-columns: 1fr; } .bento { grid-template-columns: 1fr 1fr; } }
```
`<img>` 必带 `width`/`height` 属性（防 CLS）+ `alt`；首屏之下的图加 `loading="lazy" decoding="async"`。
