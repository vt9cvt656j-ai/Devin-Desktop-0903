/**
 * Interface language.
 *
 * English is the source of truth: every other locale is a `Partial<Dict>` merged over it,
 * so a key added to EN and not yet translated shows English rather than a raw key name or
 * a type error across five files. That also means a half-finished locale is shippable.
 */
import { de } from "@/lib/locales/de";
import { es } from "@/lib/locales/es";
import { ja } from "@/lib/locales/ja";
import { ko } from "@/lib/locales/ko";
import { zhCN } from "@/lib/locales/zh-CN";
import { zhTW } from "@/lib/locales/zh-TW";

export type Lang = "en" | "ja" | "ko" | "zh-CN" | "zh-TW" | "de" | "es";
export type Currency = "cny" | "usd";

/** Offered in Settings, in the order shown there. Each label is in its own language. */
export const LANGS: { value: Lang; label: string }[] = [
  { value: "en", label: "English" },
  { value: "ja", label: "日本語" },
  { value: "ko", label: "한국어" },
  { value: "zh-CN", label: "简体中文" },
  { value: "zh-TW", label: "繁體中文" },
  { value: "de", label: "Deutsch" },
  { value: "es", label: "Español" },
];

/** BCP-47 tag for date and number formatting. */
export const LOCALE_TAG: Record<Lang, string> = {
  en: "en-US",
  ja: "ja-JP",
  ko: "ko-KR",
  "zh-CN": "zh-CN",
  "zh-TW": "zh-TW",
  de: "de-DE",
  es: "es-ES",
};

const EN = {
  openEditor: "Open the editor",
  navOverview: "Overview",
  navUsage: "Usage",
  navSettings: "Settings",
  navBilling: "Plans & billing",
  navIntegrations: "Integrations",
  navDevices: "Devices",
  accountMenu: "Account menu",
  getTheApp: "Get the desktop app",

  overview: "Overview",
  overviewLede: "Your plan, quota and recent activity.",
  includedUsage: "Your included usage",
  used: "used",
  ofQuota: "of",
  refillsIn: "Refills in",
  refillsNow: "Refills on your next request",
  noneIncluded: "None included",
  onFreePlan: "on the free plan",
  freeFallback: "Requests draw on your credit balance and daily free allowance.",
  creditBalance: "Credit balance",
  creditBalanceSub: "Available whatever your plan",
  dailyFree: "Daily free allowance",
  dailyFreeSub: "Resets every day",
  thisWeek: "This week",
  resets: "Resets",
  recentActivity: "Recent activity",
  noRequests: "No requests yet.",
  /** {days} is the plan's billing period, e.g. "every 30 days". */
  everyDays: "every {days} days",
  planGranted: "Granted, not purchased",
  when: "When",
  model: "Model",
  tokensIn: "In",
  tokensOut: "Out",
  cost: "Cost",
  estimated: "estimated",

  usage: "Usage",
  usageLede: "Every model request billed to this account, newest first.",
  spentAllTime: "Spent all time",
  requestsShown: "Requests shown",
  requestsShownSub: "The gateway keeps the most recent 200",
  requests: "Requests",
  pagePrev: "Previous",
  pageNext: "Next",
  /** {from}/{to}/{total} are replaced with the row numbers on screen. */
  showingRange: "Showing {from}–{to} of {total}",
  goToPage: "Go to page {page}",

  settings: "Settings",
  settingsLede: "Account details held by the gateway.",
  profile: "Profile",
  profileNote: "Your name and picture appear in the sidebar and on your account.",
  firstName: "First name",
  lastName: "Last name",
  profilePicture: "Profile picture",
  changePicture: "Change",
  removePicture: "Remove",
  pictureHint: "Click the picture to upload a new one — PNG, JPEG, WebP or GIF, cropped to a square.",
  saveProfile: "Save changes",
  saving: "Saving…",
  profileSaved: "Profile saved.",
  pictureTooLarge: "That image is too large. Pick one under 12 MB.",
  pictureUnreadable: "That file could not be read as an image.",
  account: "Account",
  email: "Email",
  accountId: "Account ID",
  role: "Role",
  administrator: "Administrator",
  member: "Member",
  memberSince: "Member since",
  lastSignIn: "Last sign-in",
  plan: "Plan",
  planFree: "Free",
  planTrial: "Trial",
  planBasic: "Basic",
  planPro: "Pro",
  planPower: "Power",
  planUltra: "Ultra",
  /** Joins a browser to its platform: "Chrome on macOS". */
  deviceOn: "on",
  checkoutNoUrl: "Stripe did not return a checkout link.",
  currentPlan: "Current plan",
  expires: "Renews or expires",
  includedQuota: "Included quota",
  perWindow: "per window",
  weeklyCap: "Weekly cap",
  perWeek: "per week",
  notIncluded: "Not included on this plan",
  noWeeklyCap: "No weekly cap",
  session: "Session",
  signOutNote: "Signing out clears this browser's session. The desktop app keeps its own.",
  signOut: "Sign out",

  language: "Language",
  languageNote: "Saved to your account, so everywhere you sign in uses the same language.",
  interfaceLanguage: "Interface language",

  devices: "Devices",
  devicesLede: "Where this account is signed in. Sign out anything you do not recognise.",
  deviceCol: "Device",
  ipCol: "IP address",
  browserGroup: "Browser",
  desktopGroup: "Desktop app",
  mobileGroup: "Mobile app",
  /** {n} is the number of signed-in devices in that group. */
  signedInCount: "{n} signed in",
  noneSignedIn: "Not signed in on any device of this kind.",
  createdCol: "Signed in",
  lastActiveCol: "Last active",
  revoke: "Revoke",
  revoking: "Revoking…",
  thisDevice: "This device",
  noDevices: "No signed-in devices to show.",
  revokeSelfWarning:
    "This is the session you are using. Revoking it signs this browser out immediately.",
  deviceRevoked: "Signed out. That device will be asked to log in again on its next request.",
  // Said plainly rather than hidden, because a list that silently omits sessions is
  // worse than one that admits what it cannot see.
  untrackedSession:
    "This browser signed in before device tracking existed, so it is not in the list below and cannot be signed out individually. Signing out and back in adds it.",
  integrations: "Integrations",
  integrationsLede: "How this account connects to the desktop app and the API.",
  desktopApp: "Mr.day One for desktop",
  connected: "Connected",
  signedOut: "Signed out",
  notDetected: "Not detected",
  desktopConnected: "Running here as",
  desktopOnline: "Signed in and running. Last checked in",
  desktopSecondsAgo: "seconds ago.",
  desktopVersion: "version",
  desktopReuse: "The sign-in page will offer to reuse this session instead of asking for your password.",
  desktopSignedOut: "The app is running here but nobody is signed in, so it cannot pass a session to the browser.",
  desktopUnreachable: "The page could not reach the app. If it is running, your browser is blocking the connection — the detail below says what it saw.",
  desktopDetail: "What the browser reported",
  codeHosts: "Code hosts",
  codeHostsLede:
    "Link an account and its repositories become available in the editor — type @github: or @gitlab: to pick one.",
  connect: "Connect",
  connecting: "Opening…",
  disconnect: "Disconnect",
  connectedAs: "Connected as",
  useToken: "Use a token",
  connectWithToken: "Connect with a token",
  /** Says why there is no one-click sign-in, so the button is not read as broken. */
  oauthUnavailable: "One-click sign-in needs an OAuth app registered on the server.",
  cancel: "Cancel",
  tokenPlaceholder: "Paste a personal access token",
  tokenScopes: "Needs scope:",
  tokenCreate: "Create one",
  integrationConnected: "Connected. Your repositories are now available in the editor.",
  integrationCancelled: "Connection cancelled. Nothing was linked.",
  integrationError: "That did not work. Nothing was linked — try again.",
  disconnectedNote: "Disconnected. To also revoke access at the provider, visit",
  desktopNeedsPermission: "Needs permission",
  desktopPermissionAsk:
    "Your browser has not been given permission to reach apps on this computer, so this page cannot tell whether Mr.day One is running. Chrome only asks when you click.",
  desktopConnectButton: "Check for the app",
  desktopChecking: "Checking…",
  desktopPermissionBlocked: "Blocked by the browser",
  desktopPermissionBlockedHelp:
    "Local network access is blocked for this site. Turn it back on in Chrome: click the icon to the left of the address bar, then allow local network access, and check again.",
  download: "Download the app",
  apiHeading: "API",
  baseUrl: "Base URL",
  auth: "Authentication",
  authValue: "Bearer token, same session as this page",
  modelsAvailable: "Models available",
  available: "available to your account",

  billing: "Plans & billing",
  billingLede:
    "Subscribe for a quota that refills through the day, top up credits that never expire, or redeem a code.",
  tabSubscription: "Subscription",
  tabCredits: "Credits",
  tabRedeem: "Redeem",
  includedEachMonth: "included each month",
  per55: "per 5½-hour window",
  weeklyCapSuffix: "weekly cap",
  allModels: "Every model on your account",
  fullDayQuota: "Full paid quota for 24 hours",
  onePerAccount: "One per account, never renews",
  subscribe: "Subscribe",
  buy: "Buy",
  topUp: "Top up",
  alreadyUsed: "Already used",
  current: "Current",
  bestRate: "Best rate",
  perMonth: "/mo",
  perDay: "/day",
  credits: "credits",
  neverExpires: "Never expires",
  per: "per",
  monthlyPlans: "Monthly plans",
  dayPassSet: "Day pass",
  creditPacks: "Credit packs",
  customAmount: "Custom amount",
  activationCode: "Activation code",
  redeemTitle: "Redeem a code",
  accountNow: "Your account right now",
  planCodeTitle: "Plan codes",
  // Both descriptions state what apply_plan and apply_credits actually do — grants add
  // to what is already there rather than replacing it — so nobody redeems a code fearing
  // it will wipe the time they have left.
  planCodeBody:
    "Grant a plan for a set number of days. If your plan still has time left, the days are added on top and the included quota is added to what you already have — nothing is replaced.",
  creditCodeTitle: "Credit codes",
  creditCodeBody:
    "Add credits to your balance. Credits never expire and are spent only once a plan's included quota runs out, so they keep working on any plan.",
  noPlan: "No plan",
  redeem: "Redeem",
  redeemNote:
    "Codes are issued by the operator. Redeeming adds the plan or credits to this account immediately.",
  redeemOk: "Redeemed. Your account has been updated.",
  youAreOn: "You are on",
  until: "until",
  extends: "Buying again extends it — nothing is lost.",
  freePlanBalance: "You are on the free plan, with a credit balance of",
  notEnabled: "Card payments are not switched on yet — the gateway has no Stripe key configured.",
  secure: "Payments are handled by Stripe. Card details never reach this server.",
  paidOk: "Payment received. Your account updates within a few seconds.",
  canceled: "Checkout canceled. Nothing was charged.",
  opening: "Opening Stripe…",
  loadFailed: "Could not load. Reload to try again.",
  loading: "Loading…",
} as const;

export type Dict = Record<keyof typeof EN, string>;

const BASE = EN as unknown as Dict;

/**
 * Every locale is complete at runtime because each one is spread over English first.
 * Nothing in the app has to handle a missing string.
 */
export const DICTS: Record<Lang, Dict> = {
  en: BASE,
  ja: { ...BASE, ...ja },
  ko: { ...BASE, ...ko },
  "zh-CN": { ...BASE, ...zhCN },
  "zh-TW": { ...BASE, ...zhTW },
  de: { ...BASE, ...de },
  es: { ...BASE, ...es },
};

/** The gateway answers in Chinese; translate what a buyer can actually trigger. */
const SERVER_MSG: Record<string, string> = {
  激活码无效: "That code is not valid.",
  激活码已被使用: "That code has already been used.",
  请输入激活码: "Enter a code first.",
  商品不存在或已下架: "That product is no longer available.",
  该商品每个账号仅限购买一次: "This is limited to one per account.",
  "该商品未绑定 Stripe 价格": "That product has no Stripe price attached.",
  "支付尚未配置：网关缺少 STRIPE_SECRET_KEY": "Card payments are not configured yet.",
};

/** Unrecognised messages pass through untouched so nothing is ever swallowed. */
export function serverMessage(error: unknown, lang: Lang): string {
  const raw = error instanceof Error ? error.message : String(error);
  return lang.startsWith("zh") ? raw : (SERVER_MSG[raw] ?? raw);
}
