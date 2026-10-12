# Empower Personal Dashboard: functional parity spec for a SimpleFIN-backed app

Compiled 2026-10-10 from public sources only. I did not download, decompile, log in to, or call the app or any Empower API. The text is paraphrased, with only short UI labels quoted. It contains no PII: reviewer names and store-review authors are left out.

---

## 0. Product status (read this first)

Where the product stood on 2026-10-10:

- **The standalone app is effectively retired.**
  - Google Play listing for `com.personalcapital.pcapandroid` (developer shown as "Empower Personal Wealth, LLC"; "About the developer" shows Personal Capital Corporation): last archived in Sep 2025, showing 3.8 stars, ~20K reviews, 1M+ downloads, "Updated on Jun 5, 2025". The live URL now returns 404.
  - iOS listing `id504672168` ("Empower Personal Dashboard™", subtitle "Personal Capital"): last archived Apr 2026, at version 17.3 (Jan 2026). The live page now returns 404, and the iTunes lookup API returns zero results for that id.
  - APKMirror lists Android 13.9.4 as the last upload (Jan 15, 2026).
- **The successor is the unified "Empower ®" app.** iOS `id1001257338`, bundle `com.empower-retirement.ios.Participant-Mobile`, formerly the 401(k) participant app; v2026.09.16, 4.78 stars from ~417K ratings. Its release notes from late 2025 to 2026 include "Duplicate username handling for migrating users", "Restored ability to upload attachments to transactions", and a fix for "duplicate linked accounts". A May–Sep 2026 third-party review says the Personal Dashboard was merged into this app in late 2025. I could not find an official dated announcement of the merger, so treat the date as **reported, not confirmed**. I did not find this app's Android package id.
- **A new dashboard (v1.1–1.5) rolled out from Apr to Aug 2026.** It is described as a cleaner layout, "New Budgeting", a personalized-guidance quiz, and a Personal Strategy widget. In Aug 2026 (v1.5) it shipped on iOS and Android with a **bottom navigation bar that replaces nested menus**. Empower has not published the bottom-tab labels.
- **Implication for this spec.** Most detailed behavior, computations and labels come from the help center. The archived copies of those articles date from **2024–2025**, so they describe the legacy dashboard (web plus the old mobile app). The 2026 redesign kept the same toolset (Budgeting, Emergency Fund, Debt Paydown, Retirement Planner, Net Worth, Transactions, Portfolio Analysis, Savings Planner) and migrated budget data. Exact 2026 mobile screens are not public, so they are marked **[inferred]** below.

Confidence tags used throughout:
- **[confirmed]**: stated in an Empower help or release article or a store listing.
- **[review]**: stated by a third-party review.
- **[inferred]**: my reconstruction.

---

## 1. Sources

All accessed 2026-10-10. "WB" means read via an Internet Archive Wayback Machine capture of the original URL, because the live help center (`support-personalwealth.empower.com`) is behind a Cloudflare bot challenge. WB captures of help articles date mostly from 2024-09 to 2025-10.

### 1.1 Store listings and release notes
| # | Source | URL | Notes |
|---|---|---|---|
| S1 | Google Play, Empower Personal Dashboard | https://play.google.com/store/apps/details?id=com.personalcapital.pcapandroid | Live page 404. WB capture 2025-09-05 used for description, rating, "What's new" ("Fix for unexpected logout/crash…"), and data-safety summary |
| S2 | App Store, Empower Personal Dashboard (id504672168) | https://apps.apple.com/us/app/empower-personal-dashboard/id504672168 | Live page 404. WB capture 2026-04-21: description, version history 15.1.1→17.3, privacy labels. Notable: 16.3 (06/28/2024) "New category rules to better manage your transactions" |
| S3 | App Store, Empower ® (id1001257338) | https://apps.apple.com/us/app/empower/id1001257338 | Live iTunes lookup API (public store metadata) plus WB capture 2026-08-24 for version history (tax filing, Stock Plan Services, attachment restore, migrating users) |
| S4 | APKMirror, Personal Capital / Empower Personal Dashboard | https://www.apkmirror.com/apk/personal-capital-corporation/personal-capital-finance/ | Version and date list only |
| S5 | Empower dashboard release notes, Apr 2026 (v1.1) | https://www.empower.com/dashboard/product-releases-april-2026 | New dashboard, New Budgeting, guidance quiz |
| S6 | Release notes, May 2026 (v1.2) | https://www.empower.com/dashboard/product-releases-may-2026 | Personal Strategy widget; manual accounts under "Other" editable |
| S7 | Release notes, Jun 2026 (v1.3) | https://www.empower.com/dashboard/product-releases-june-2026 | "Meet the new dashboard!" |
| S8 | Release notes, Jul 2026 (v1.4) | https://www.empower.com/dashboard/product-releases-july-2026 | Web/mobile display differences |
| S9 | Release notes, Aug 2026 (v1.5) | https://www.empower.com/dashboard/product-releases-august-2026 | New dashboard on iOS/Android; bottom navigation |

### 1.2 Empower marketing pages
| # | Source | URL |
|---|---|---|
| S10 | Tools index ("Empower Personal Dashboard™" tools) | https://www.empower.com/personal-investors/tools-template |
| S11 | Budgeting & Cash Flow | https://www.empower.com/tools/budgeting-cash-flow |
| S12 | Retirement Planner | https://www.empower.com/tools/retirement-planner |
| S13 | Net Worth | https://www.empower.com/tools/net-worth |
| S14 | Portfolio Analysis | https://www.empower.com/tools/portfolio-analysis |
| S15 | Emergency Fund | https://www.empower.com/tools/emergency-fund |
| S16 | Personal Capital → Empower transition page | https://www.empower.com/empower-personal-wealth-transition |

### 1.3 Empower help center articles (all WB)
Base URL: `https://support-personalwealth.empower.com/hc/en-us/articles/<id>`

| # | Article | id |
|---|---|---|
| H1 | Dashboard Overview | 201169740 |
| H2 | My Accounts Panel Overview | 201169750 |
| H3 | Transactions Overview | 201169690 |
| H4 | What is the Cash Flow Graph, and How is it Useful? | 201169700 |
| H5 | Using the Budgeting Widget | 360007128514 |
| H6 | How are yearly and weekly spending goals calculated? | 216597618 |
| H7 | How is average spending calculated? | 204228324 |
| H8 | Creating a New Rule for Transactions | 26791477954583 |
| H9 | How to Split Transactions | 360055544694 |
| H10 | Create Custom Categories for Transactions | 115012742188 |
| H11 | Delete or Remove a Custom Category | 115012742488 |
| H12 | Edit Multiple Transactions | 201169860 |
| H13 | How Do I Mark Duplicate Transactions? | 115006084588 |
| H14 | How do I edit the category of a transaction? | 201169830 |
| H15 | New Feature: Editing Transaction Dates | 29808430167191 |
| H16 | How to Manually Add Transactions to an Account | 30719550034711 |
| H17 | Bill Reminders Overview | 201169730 |
| H18 | How do I add my utility bills…? | 201169990 |
| H19 | Emergency Fund | 360024294574 |
| H20 | Debt Paydown | 360024461133 |
| H21 | Accounts are not listed in the Debt Paydown tool | 360042774954 |
| H22 | What is the Savings and Withdrawal Planner? | 360024293814 |
| H23 | Retirement Savings | 360024460873 |
| H24 | How do I adjust the amounts saved in the Savings Planner? | 360025979013 |
| H25 | What is the Retirement Planner? | 360007616873 |
| H26 | Retirement Spending Ability | 360008421893 |
| H27 | Growth Assumptions (Retirement Planner) | 360007520814 |
| H28 | Understanding Detailed Cash Flow in Retirement Planner | 360008463853 |
| H29 | What Income Events can I create…? | 360007518034 |
| H30 | What Spending Goals can I create…? | 360007621673 |
| H31 | Social Security Estimate | 4404885954711 |
| H32 | What is the Accumulation Phase? | 360007617333 |
| H33 | What is the Distribution Phase? | 360007517154 |
| H34 | What is my Starting Portfolio? | 360007617113 |
| H35 | How are Taxes applied on Withdrawals? | 360007517174 |
| H36 | Can I adjust how much I spend each year…? | 360005382133 |
| H37 | How do we calculate Yearly Savings in the Retirement Planner? | 115009407728 |
| H38 | Why aren't "other assets" included in my retirement assets? | 216377177 |
| H39 | Retirement Fee Analyzer Calculations Overview | 201169600 |
| H40 | How is the Cost of Fees over Time chart calculated? | 201170190 |
| H41 | What does "Fund Fees/Yr" mean? | 201170170 |
| H42 | How do I view fees for all of my Investment accounts? | 201169630 |
| H43 | What is the purpose of the Investment Checkup? | 201170260 |
| H44 | What are the primary limitations of the Investment Checkup? | 201170270 |
| H45 | What does the Efficient Frontier section show? | 201170230 |
| H46 | What does the Historical Performance section show? | 201170240 |
| H47 | What does the Monte Carlo section show? | 201170250 |
| H48 | What is tracked by the Alternative Asset Class? | 201170290 |
| H49 | Portfolio Overview | 201169770 |
| H50 | What is the You Index®? | 201169610 |
| H51 | Why does your performance graph not match…? | 218209787 |
| H52 | Why don't my holdings show 1-Day % and 1-Day $ changes? | 201170030 |
| H53 | How do I classify unclassified holdings? | 202776684 |
| H54 | Investment Account Details | 201169720 |
| H55 | Credit Card Account Details | 201169760 |
| H56 | Editing Cash Account Details | 360034533454 |
| H57 | How do I change my account type? | 204974614 |
| H58 | Change Your Account Name | 201169840 |
| H59 | Group Cash Accounts with Investments | 201169880 |
| H60 | 'Exclude from Investment and Planning tools' Feature | 18955705554199 |
| H61 | How to Mark an Account as Closed | 215263538 |
| H62 | Managing Closed Accounts | 360050987994 |
| H63 | Why is there a spike/drop in my Net Worth? | 115013120247 |
| H64 | Create a Manual Account | 360008988414 |
| H65 | How do I Update the Balance of my Manual Accounts? | 201169870 |
| H66 | How do I add the value of my home…? | 203285240 |
| H67 | How do I add the value of my personal property…? | 201169670 |
| H68 | Tracking the Value of Precious Metals | 360008221913 |
| H69 | Cryptocurrency | 360000048568 |
| H70 | Add Manual Investments Holdings | 201169980 |
| H71 | How do I add stock option grants? | 201169640 |
| H72 | Account Linking Tool | 201169660 |
| H73 | Some of my accounts are showing a Red or Yellow Icon… | 115006083908 |
| H74 | How often does Empower Personal Dashboard update my data? | 201169810 |
| H75 | Am I able to see more than 3 months of data…? | 201170160 |
| H76 | Why do some of my accounts have more data than others? | 360013422514 |
| H77 | Re-sync Account Transactions | 23481877705751 |
| H78 | How to Clear Pending Transactions | 29604563209495 |
| H79 | Why can't I see my pending transactions? | 201170060 |
| H80 | Transactions Showing as Pending on Mobile (As of July 7, 2025) | 33310851381527 |
| H81 | Why are my credit card payments not included in the cash flow section? | 201170070 |
| H82 | Investment Income, Dividends Received, and Advisory Fees not showing in Cash Flow/Budgeting Tools | 115009570667 |
| H83 | How can I use "categories" to budget with Empower Personal Cash? | 1500007916922 |
| H84 | Export Net Worth/Transactional Data Through The Mobile Application | 360015842114 |
| H85 | Export Transactions To A .CSV File Through Our Web Application | 18679551671703 |
| H86 | Can I Import My Data from Mint? | 19048263355287 |
| H87 | Feature Differences Between the … Web and iPad applications | 201169790 |
| H88 | Daily Email: Your daily financial monitor | 201169780 |
| H89 | How can I enable daily or weekly emails…? | 201169900 |
| H90 | Change the Settings for Security Alerts | 360006954354 |
| H91 | Additional Second Layer of Multi-Factor Authentication (MFA) | 115012967988 |
| H92 | Why did my TouchID stop working on mobile application? | 115013120067 |
| H93 | What is the Mobile Authentication Feature? (TOTP) | 360056729413 |
| H94 | How to Use Mobile App Verification | 360054680354 |
| H95 | Manage Your Authorized Devices | 360037607854 |
| H96 | How do I log out of the mobile application? | 201169910 |
| H97 | Android Biometrics Login Shows "No Internet" Error Message | 28869544285207 |
| H98 | Multiple Layers of Security | 201169540 |
| H99 | Adding an Additional User… | 360049789034 |
| H100 | Financial Roadmap | 360047947774 |
| H101 | Notification Disclaimer | 360009703374 |
| H102 | Logging in on the New Empower Experience | 33313053468055 |

### 1.4 Third-party reviews
| # | Source | Date | URL |
|---|---|---|---|
| R1 | Rob Berger, Empower Review and User's Guide | updated 2026-10-03 | https://robberger.com/empower-review/ |
| R2 | NerdWallet, Empower Personal Dashboard budget app review | updated 2026-02-26 | https://www.nerdwallet.com/finance/learn/empower-personal-dashboard-budget-app-review |
| R3 | The Penny Hoarder, Empower App Review 2026 | updated 2026-06-03 | https://www.thepennyhoarder.com/budgeting/empower-app-review/ |
| R4 | FinCompareLab, Empower Review 2026 | 2026-05-28, updated 2026-09-22 | https://www.fincomparelab.com/reviews/empower-review/ |
| R5 | Finny blog, Empower Personal Dashboard Review 2026 (written by a competing app) | 2026-08-08 | https://getfinny.app/blog/empower-personal-dashboard-review-2026 |
| R6 | Benzinga (sponsored), Retirement Planner Tool Tested | 2025-06-16 | https://www.benzinga.com/money/empower-personal-dashboard-review-retirement-planner-tool-tested (WB) |
| R7 | Benzinga (sponsored), Investment Checkup hands-on | 2025-06-16 | https://www.benzinga.com/money/empower-personal-dashboard-review-a-hands-on-look-at-the-investment-checkup-tool (WB) |
| R8 | Financial Samurai, Empower Personal Dashboard Review (affiliate) | 2026-02-08 | https://www.financialsamurai.com/empower-personal-dashboard-review/ |
| R9 | Robo-Advisor Pros, A Review of Empower's Free Tools | 2023-08-22 (older; used for legacy UI labels) | https://www.roboadvisorpros.com/a-review-of-empowers-free-tools/ |

Sources I tried but could not use:
- The live help center (Cloudflare challenge).
- The Bogleheads migration thread (HTTP 402).
- Reddit (the local reddit tool failed with a chrome-agent CLI incompatibility).

Benzinga and Financial Samurai are sponsored or affiliate content, so weigh their praise accordingly.

---

## 2. Navigation map

### 2.1 Legacy mobile app (iOS/Android "Empower Personal Dashboard", up to ~2025)
```
Launch → Login (email + password; device verification by SMS/call/email or Mobile App Verification) → 6-digit PIN / Touch ID / Face ID / fingerprint on return
│
├─ Menu icon, top left (side drawer) [confirmed: H84 "select the menu icon in the top left … select 'Net Worth' / 'Transactions'"]
│   ├─ Dashboard (widget stack) [inferred: mirrors web H1; R7 says mobile is "slightly different"]
│   ├─ Net Worth [confirmed H57/H14/H84] → account list grouped by type → Account Details → pencil = Edit Account
│   │     date-range calendar icon (top middle); export/download arrow (top right, iOS → PDF) [confirmed H84]
│   ├─ Transactions [confirmed H84] → calendar date range (top right) → export arrow (CSV, iOS) → Transaction details
│   ├─ Cash Flow [inferred: web-parity tool, cash-flow graphs referenced on mobile in H9]
│   ├─ Budgeting [confirmed H83: "Budgeting Tool" on Dashboard or mobile app]
│   ├─ Portfolio / Investing (Holdings, Allocation; balances and per-holding performance only) [confirmed H87: account performance web-only]
│   ├─ Investment Checkup [review R7: "Investment Checkup on the mobile menu"; H87 (older) says web-only → conflict, likely added later]
│   ├─ Retirement Planner (default scenario only on iPad; alternative scenarios web-only) [confirmed H87; mobile screens referenced in H32/H33/H34]
│   ├─ Savings Planner (Retirement / Emergency Fund / Debt Paydown) [inferred]
│   ├─ Wealth Management / advisor scheduling [inferred]
│   └─ Referrals [confirmed S2 v17.2 "referrals not opening from the main menu"]
│
└─ Gear icon, top right ("Account Settings") [confirmed H70, H95, H97]
    ├─ Link account → "Find account" search → "Add manual account" → type picker (incl. "Manual Investment Holdings", Cryptocurrency) [confirmed H70, H69]
    ├─ Authorized Devices (Edit/remove) [confirmed H95]
    ├─ Fingerprint Authentication toggle (Android) / Touch ID (iOS) [confirmed H97, H92]
    ├─ Mobile app verification / Mobile Authentication (TOTP code generator) [confirmed H93, H94]
    └─ Profile & Settings, Log out [confirmed H96]
```
iPad: "Financial Planning" screen with a "Retirement planner" widget [confirmed H87]; also "My Plans" [review].

### 2.2 Legacy web (gives section names the mobile app mirrored)
- **Top nav**:
  - "Overview" → Dashboard, Transactions [confirmed H85]
  - "Banking" → Cash Flow, Budgeting, Bills [review R9]
  - "Investing" → Holdings, Allocation, Sectors, Performance, You Index [confirmed H53, H87; R9]
  - "Planning" → Retirement Planner, Savings Planner, Investment Checkup, Retirement Fee Analyzer, Education planner [confirmed H42; R1, R9]
  - "Wealth Management" [confirmed H87]
- **Left "My Accounts" panel**:
  - "+" Link button
  - NET WORTH header
  - Assets/Liabilities mini-graphs
  - "Needs attention" messages
  - Account groups: Cash, Investment, Credit, Loan, Mortgage, Other Asset, Other Liabilities
  - Collapse button

  [confirmed H2]
- **Name menu, top right**: Settings (Profile, Email Subscriptions, Security Settings, Authorized Devices, Additional Users) [confirmed H88, H90, H91, H99]

### 2.3 Current product (new dashboard, 2026; unified Empower app)
- **Bottom navigation bar** replaced nested menus (v1.5, Aug 2026) [confirmed S9]. **The tab labels are not published.**
- **Dashboard widgets named in release notes**:
  - Net Worth (dollar change shown) [confirmed S8, S9]
  - Budgeting ("New Budgeting") [confirmed S5, S7, S8]
  - Retirement (age plus income goal, color-coded on-track indicators, "Lifetime Income Score") [confirmed S5–S9]
  - Personal Strategy (advisory clients) [confirmed S6]
  - Workplace-accounts vs Net Worth toggle [confirmed S7, S8]
  - Personalized guidance quiz [confirmed S5]
- **Emergency Fund and Debt Paydown** are reachable through Savings Planner during the transition and will "later appear as dashboard widgets". Users must set them up again [confirmed S7–S9].
- **Accounts section** handles account actions [confirmed S8]. Search appears as a feature [confirmed S5]. A virtual assistant exists [confirmed S7].
- **Plausible tabs [inferred]**: Home/Dashboard · Accounts · (Budget/Spending or Planning) · Invest/Retirement · More/Profile. Do not treat this as fact.

### 2.4 Recommended navigation for our SimpleFIN app [inferred design]
Bottom tabs:
1. **Home**: widget stack: net worth, spending vs budget, cash flow, upcoming recurring, emergency fund, debt, alerts.
2. **Accounts**: grouped list, net worth chart, account details.
3. **Transactions**: search, filters, edit.
4. **Plan**: budget, cash flow, savings planner (emergency fund, debt paydown), retirement planner (later).
5. **More**: categories, tags and rules, manual accounts, settings, security, export.

---

## 3. Feature catalog

Feasibility tags (one or more per feature):
- **[existing lib]**: already provided by the simplefin Rust library (unified SimpleFIN plus manual accounts; five-way classification Cash/Investments/Other Assets/Credit Cards/Loans; net worth and history; balance-change anomalies; keyword spending categorization; recurring-expense detection; month-over-month trends; manual accounts with staleness).
- **[lib extension]**: computable from SimpleFIN data plus existing lib output with new code.
- **[needs user input]**: requires the user to enter or confirm data.
- **[needs data SimpleFIN lacks]**: holdings, prices, APR, credit limits, due dates, statement or minimum due, principal/interest split, institution-provided categories, market or benchmark data, third-party valuations.

SimpleFIN gives us:
- Accounts: id, name, currency, balance, available balance, balance date, and org (name, domain, URLs).
- Transactions: id, posted, amount, description, payee, memo, pending, and optional transacted_at.
- No holdings and no categories.

### F1. Home dashboard (widget overview)
- **Purpose:** One-screen summary with links into each tool [confirmed H1].
- **Legacy widgets** [confirmed H1]:
  - **Net Worth**: 90-day chart, current total at top, 90-day change and 1-day change at the bottom, hover/scrub for daily values.
  - **Budgeting**: this month vs last month.
  - **Cash Flow**: last 30 days, inflow vs outflow as colored bars by category; tapping a segment drills into Cash Flow for that category.
  - **Portfolio Balance**: 90-day chart of investment accounts with 90-day and 1-day change.
  - **Market Movers**: 1-day % for S&P 500, US stock, foreign stock and US bond indexes; links to Holdings.
  - **Retirement Savings**: meter of saved vs annual goal, with a "still need to save" or congratulations message.
  - **Emergency Fund**: 12 monthly bars of bank cash against a target band.
  - **Debt Paydown**: YTD change, total owed, scrub tooltip.
- **Other dashboard elements** [review R7]: a news ("FOMO") feed and a budget prompt that builds a monthly budget automatically once cash or credit accounts are linked.
- **2026 widgets** (see 2.3): net worth, budgeting with "personalized insights", retirement on-track indicators, guidance quiz results.
- **Data:** account balances and history, transactions, budget, plan inputs.
- **Feasibility:**
  - Net worth, cash flow, budget and debt widgets: **[existing lib]** + **[lib extension]**.
  - Market Movers and Portfolio intraday: **[needs data SimpleFIN lacks]**.
  - Portfolio Balance from investment-account balance history: **[existing lib]**.

### F2. Account linking, sync and connection health
- **Purpose:** Connect institutions and keep data fresh [confirmed H72].
- **UI:** "+"/Link → search institution by name or URL → credentials or OAuth → MFA prompts → all accounts at that login are auto-discovered [confirmed H72]. A **red icon** means the user must act (re-enter credentials or answer MFA); a **yellow icon** means a provider-side sync error. A "Needs attention" section lists messages [confirmed H2, H73]. A per-account **re-sync/refetch** button is limited to once per week [confirmed H77, H78].
- **Behavior:**
  - Nightly refresh around midnight, plus on login at most every 8 hours [confirmed H74].
  - Initial history pulls 30–90 days, sometimes up to a year; balance history starts at link date; no historical import (including from Mint) [confirmed H75, H76, H86; R4].
- **Feasibility:**
  - SimpleFIN Bridge linking happens outside the app (user pastes a setup token): **[needs user input]**.
  - Health and staleness per account (balance-date age, SimpleFIN `errors` array) mapped to red/yellow states: **[lib extension]**; the staleness concept is **[existing lib]** for manual accounts.

### F3. Accounts list and grouping
- **Purpose:** Every account grouped by type, with totals [confirmed H2].
- **UI:**
  - Groups in this order: Cash, Investment, Credit, Loan, Mortgage, Other Asset, Other Liabilities.
  - Tapping a group header opens a filtered Net Worth view. Tapping an account opens Account Details.
  - Credit group rows show APR, Credit Limit, Credit Used and Balance [confirmed H55].
- **Feasibility:**
  - Grouping: **[existing lib]** (five classes). Mortgage is a sub-type of Loans, and "Other Liabilities" needs a sixth class: **[lib extension]**.
  - APR, limit and used: **[needs data SimpleFIN lacks]**. Limit can be inferred as `available + |balance|` for cards when available balance is present: **[lib extension]**, heuristic. Otherwise **[needs user input]**.

### F4. Account details
- **Shared:** balance chart over a chosen date range plus the account's transaction table; date selector top right; resync icon [confirmed H54, H77].
- **Credit card:** institution, Available Credit, APR, Balance, transactions [confirmed H55].
- **Investment account:** tabs **Balance** / **Income** / **Holdings**. Income is the sum of transactions categorized "Interest" or "Investment Income", excluding transfers and deposits [confirmed H54].
- **Pending:** Balance includes only posted transactions; Available Balance includes pending [confirmed H79].
- **Feasibility:**
  - Balance chart, transactions, available vs posted: **[existing lib]** / **[lib extension]**.
  - Income tab from categorized investment transactions: **[lib extension]**, provided the bridge supplies dividend/interest rows.
  - Holdings: **[needs data SimpleFIN lacks]**.

### F5. Account editing and lifecycle
- **Edit Account (pencil):**
  - Firm Name (renames every account under that login) vs Account Name.
  - Account type drop-down.
  - For cash accounts: "Group with investment accounts" (for HSAs and cash-management accounts) and "Exclude from Investment and Planning tools", which removes the account from Cash Flow and planning [confirmed H56–H60].
  - Cash account types: Bank, Certificate of Deposit (CD), Trust, Custodial, Business/Corporate, Health Savings Plan [confirmed H56].
- **Mark as closed:**
  - User picks a close date; the balance is zeroed from that date and later transactions are removed [confirmed H61].
  - A "Closed Accounts" list on the Net Worth page shows close date and final balance, a 90-day pre-close chart, and options to Reopen or Change Closed Date [confirmed H62].
  - A wrong close date causes net-worth spikes or drops, for example after a refinance [confirmed H63].
- **Feasibility:**
  - Rename, retype, group and exclude flags, closed state and close date: **[needs user input]** stored locally, with **[lib extension]** to honor them in net worth, cash flow and tools.
  - Reclassification override on top of the auto five-way classification: **[lib extension]**.

### F6. Manual accounts and assets
- **Purpose:** Track non-linked assets and liabilities in net worth [confirmed H64, H67].
- **Types** offered under "More…" / "Add manual account":
  - Home, with an optional Zillow Zestimate that updates monthly and can be switched off for a user-entered value [confirmed H66].
  - Car/vehicle, jewelry, precious metals and gems, other assets and liabilities [confirmed H64, H67, H68].
  - Cryptocurrency: symbol plus quantity, priced every 8 hours via CryptoCompare VWAP [confirmed H69].
  - **Manual Investment Holdings**: ticker or CUSIP, shares, cost basis, and a cash balance [confirmed H70].
  - **Custom Stock Options**: grants with exercise price, vesting start, schedule, cliff, interval [confirmed H71].
- **Edit:** Description, Account Number, Current Balance [confirmed H65]. Manual transactions are **not** allowed on manual or investment accounts [confirmed H16].
- **Feasibility:**
  - Manual accounts with balance and staleness: **[existing lib]**.
  - Auto-valuation (Zestimate, crypto prices, ticker pricing): **[needs data SimpleFIN lacks]**; a fallback is manual updates with staleness nudges.
  - Stock-option vesting math: **[lib extension]** + **[needs user input]**, with price **[needs data SimpleFIN lacks]** unless entered by the user.

### F7. Net worth and history
- **Purpose:** Assets minus liabilities, tracked over time [confirmed S13].
- **UI:**
  - Large total at top; line chart; scrub tooltips with daily value and 1-day change; 90-day and 1-day change chips [confirmed H1].
  - Net Worth page has a date range (calendar icon on mobile), per-type tabs (Cash, Investment, Credit, Loan, Mortgage, …), the account list below, and a Closed Accounts button [confirmed H2, H62, H84].
  - The Credit view shows the combined balance for the chosen range plus a 1-day change [confirmed H55].
  - iOS exports the Net Worth page as PDF [confirmed H84].
  - A pie chart of assets has been reported [review R8].
- **Computation:**
  - Daily sum of balances by sign/class. History begins at link date; there is no backfill [confirmed H62, H75; R4].
  - Mobile and web dollar change may differ slightly [confirmed S8, S9], a hint that "change" is computed per client and window.
- **Feasibility:**
  - Totals and history: **[existing lib]**.
  - Per-type tabs, 1-day and 90-day deltas, closed-account handling: **[lib extension]**.
  - Spike/drop explanation via balance-change anomalies: **[existing lib]**. Empower has no automatic anomaly alert, so this is a differentiator.

### F8. Transactions list
- **UI** [confirmed H3, H85]:
  - Account-type filter tabs: All, Cash, Investment, Credit, Loan, Mortgage.
  - Date range selector, **default 90 days**, with presets plus custom From/To. Reviews mention presets of past month, 90 days, past year and custom [review R9].
  - Free-text search within the range.
  - Sortable columns; a filter icon on the Category header filters by category.
  - Inline description editing.
  - CSV export of the current tab and range. On iOS: a sort choice, then share sheet [confirmed H84].
  - Pending transactions show when the institution provides them [confirmed H79].
- **Feasibility:**
  - List, search, filter, sort and date ranges: **[lib extension]**, a query layer over stored transactions.
  - Type tabs from the five-way classification: **[existing lib]**.
  - Pending from the SimpleFIN `pending` flag: available. Empower's known bug where pending items never clear [confirmed H78, H80; S1 review] argues for reconciling pending to posted by id, amount and description: **[lib extension]**.
  - CSV export: **[lib extension]**.

### F9. Transaction detail and edit
- **Editable fields:**
  - Description (rename); Category (with "apply to future/past" prompt, see F12); Tags (multiple).
  - Date: posted transactions only, not on manual accounts, no bulk [confirmed H15].
  - Image attachment ("Add Image"; restored in the Empower app Dec 2025) [confirmed H16; S3].
- **"Hide this duplicate":** zeroes the transaction out of reporting but keeps it greyed in the account; reversible [confirmed H13].
- **Manual add (non-investment, non-manual linked accounts):**
  - Fields: Date, Description, Category, Amount (positive = credit, negative = debit).
  - Auto-tag "Manual Transaction"; entered one at a time [confirmed H16].
- **Save confirmation:** "Save" top right on iOS; checkmark on Android for account edits [confirmed H14, H57, H58].
- **Feasibility:**
  - Local overlay of edits keyed by SimpleFIN transaction id, plus a duplicate flag and attachments: **[needs user input]** + **[lib extension]**. The overlay must survive re-fetch.
  - Merchant/payee comes from SimpleFIN `payee` or `description`: available.

### F10. Categories
- **Model** [confirmed H10, H11]:
  - Default categories (editable, never deletable) plus up to **30 custom categories**.
  - Each category has a type: **Income, Expense or Transfer**. The delete flow groups them as Income, Expense, "Other".
  - Deleting a custom category reverts its transactions to their original categories.
  - Mobile flow: Category field → "+" (Create category) → name and type → Done → Save. Android uses "Manage categories".
- **Behavioral categories** [confirmed H4, H54, H81, H82, H21]:
  - Transfers, credit card payments, 401k contributions, savings and securities trades are excluded from cash flow.
  - "Interest" and "Investment Income" feed investment income.
  - "Service Charges/Fees" on interest rows determines Debt Paydown eligibility.
- **Feasibility:**
  - Keyword categorization: **[existing lib]**. SimpleFIN supplies no categories, so the whole taxonomy is ours.
  - Category types and transfer detection (matching opposite-signed amounts across own accounts, card-payment keywords): **[lib extension]**.
  - Custom categories: **[needs user input]**.

### F11. Tags
- **Model:** many tags per transaction, one category [confirmed H4].
- **Built-in tags:**
  - "Reimbursable" **removes the transaction from budgeting calculations** [confirmed H4].
  - "Manual Transaction" [confirmed H16].
  - Examples given: medical, reimbursable, tax related [confirmed H83].
  - Users can tag, for example, pool-related spending [review R1].
- **Feasibility:** **[needs user input]** + **[lib extension]** (tag storage, budget exclusion rule).

### F12. Rules (apply to similar transactions)
- **Flow** [confirmed H8, H14; S2 v16.3, Jun 2024]:
  1. Edit Description and/or Category, then Save.
  2. A prompt asks whether to apply to future transactions ("No, just once" / Yes), with a checkbox to also apply to past ones.
  3. Updates are asynchronous and take minutes.
- **Limits:** tags are not included. A newer rule overwrites an older one. No standalone rules manager is documented.
- **Feasibility:** **[lib extension]**. Rules feed the keyword categorizer as user-priority overrides matched on normalized payee or description, then backfill history.

### F13. Split transactions
- **Rules** [confirmed H9]:
  - Up to 5 splits, each with its own Category, Tags and Amount. "Amount remaining" must reach $0.
  - Splits cannot change the category type (income stays income).
  - Not allowed on investment accounts.
  - **Web only**: splits did not show on mobile, causing cash-flow differences.
- **Feasibility:** **[needs user input]** + **[lib extension]** (split children replace the parent in aggregates). Offering splits on mobile is a parity improvement.

### F14. Bulk edit
- **Behavior:** Web only. Pencil above the Date column → checkboxes → set category, tags or description for all selected → Done [confirmed H12].
- **Feasibility:** **[lib extension]**.

### F15. Cash flow
- **Purpose:** Income vs expenses, month over month, for spending decisions [confirmed H4].
- **UI** [confirmed H4, R1, R2]:
  - Overview graph of monthly income vs expense, defaulting to the current month.
  - **Income** tab: breakdown chart of income sources on the left (tap to drill in); income-only month-over-month chart on the right.
  - **Expense** tab: category breakdown plus month-over-month chart.
  - An account filter (for example, business accounts only, or investment accounts only to see dividends) [review R1]. Comparison with the prior month [review R2].
- **Exclusions:**
  - Loan and mortgage account transactions; transfers; credit card payments; 401k contributions; Savings; Securities Trades [confirmed H4, H81].
  - By default also investment income, dividends and advisory fees. A Settings toggle, "Exclude investment income, dividends, and advisory fee from Cash Flow & Budgeting?", controls this [confirmed H82].
  - Accounts flagged "Exclude from … tools" [confirmed H60].
  - Purchases on a card count, while the card payment does not. A home purchase counts, while mortgage payments do not [confirmed H4].
- **Feasibility:**
  - Monthly aggregation and month-over-month trends: **[existing lib]**.
  - Exclusion engine (transfer types, account flags, settings toggle) and drill-down: **[lib extension]**.

### F16. Budgeting
- **Legacy model** [confirmed H5, H6, H7; R1, R2]:
  - **One overall monthly spending goal**, edited with a pencil. No per-category goals were documented.
  - **Outer ring** is spending in the past 30 days and turns **red** over budget. **Inner ring** is spending at this point last month.
  - Lists the **top 5 expense categories**, with the remainder as "Other".
  - Yearly goal = monthly × 12. Weekly goal = monthly ÷ days in month × 7, weighted across month boundaries.
  - Average monthly spending uses the last 3 months; yearly average uses the last 12 months. With thin history it extrapolates from current patterns.
  - The Emergency Fund target derives from this budget [confirmed H19, H22].
- **2026 "New Budgeting":** "personalized insights", refreshed visuals; data carried over [confirmed S5, S7, S8].
  - One 2026 review says you can set **monthly category budgets** [review R3], and another mentions "budget targets" [review R4]. Earlier reviews and the help center say no per-category budgets [R1, R2]. **Unresolved conflict.** Likely the 2026 budgeting added category budgets [inferred].
- **Feasibility:**
  - Totals and averages: **[existing lib]** (categorization + month-over-month).
  - Budget goal: **[needs user input]**, with a suggested default from the 3-month average: **[lib extension]**.
  - Ring and pacing math and per-category budgets: **[lib extension]**.

### F17. Bills (reminders)
- **Behavior** [confirmed H17, H18]:
  - Reminders cover linked credit cards, loans and mortgages only; utilities are not supported.
  - An expandable bill shows Statement Balance and Minimum Due. "Amount due" is usually the minimum.
  - Lives under Banking → Bills [review R9]. The weekly email includes upcoming bills [review R8].
- **Feasibility:**
  - Due date, statement balance and minimum due: **[needs data SimpleFIN lacks]**.
  - Substitute: **[lib extension]** that predicts the next payment date and amount from detected recurring payments (card payment and loan payment transfers), with **[needs user input]** for due day and amount overrides. Our recurring detection also covers utilities and subscriptions, which Empower lacks.

### F18. Recurring expenses and subscriptions
- **Empower status:** not offered; "no subscription tracking" is listed as a con [review R1].
- **Feasibility:** **[existing lib]** recurring-expense detection. A UI listing cadence, next expected date, amount and price change is a **[lib extension]**. Mark as a **differentiator**.

### F19. Emergency Fund
- **Behavior** [confirmed H1, H19, H22]:
  - Sums all linked bank (Cash) accounts, excluding those grouped with investments.
  - Shows **12 monthly bars** of cash (blue) against a **recommended range** (green band) of **3–6 months of the Monthly Budget**.
  - A list of cash accounts below shows year-to-date saved.
  - 2026: must be set up again; appears as a widget [confirmed S7–S9]. Marketing promises "messages to keep you on track" [confirmed S15].
- **Feasibility:**
  - Cash total and monthly history: **[existing lib]**.
  - Target band from budget or average spending, plus YTD per account: **[lib extension]**.
  - Target months (3–6): **[needs user input]**.

### F20. Debt Paydown
- **Behavior** [confirmed H1, H20, H21; R1]:
  - Lists loan, mortgage and credit card accounts with **total owed**, **YTD balance change**, a chart since Jan 1 with scrub, and APR per account.
  - Credit cards appear only if they have **at least two interest charges categorized "Service Charges/Fees" in the past 90 days**, so revolving debt shows and pay-in-full cards do not.
  - 2026: must be set up again [confirmed S7].
- **Feasibility:**
  - Balances, YTD change, history: **[existing lib]** + **[lib extension]**.
  - Interest-charge detection for card inclusion: **[lib extension]** (keyword "interest charge" / "finance charge").
  - APR and payoff projection: **[needs data SimpleFIN lacks]**, with **[needs user input]** for APR and payment.

### F21. Savings Planner: Retirement Savings (Withdrawal Planner once retired)
- **Behavior** [confirmed H22–H24]:
  - A "Saved This Year" **progress arc** against the annual savings goal from the Retirement Planner.
  - **Yearly Savings Needed** shows a range for about a 70% chance of success, toggleable to 50% or 90%.
  - The account table has columns Account, Type/Owner, Saved Last Year, Saved This Year, Balances. Each saved amount can be tapped to show contributing transactions and enter an adjustment. A checkbox reveals accounts with $0 saved.
  - Excludes ESPP, RSU and options. Once retired, the planner tracks withdrawals per account instead.
- **Computation:** savings = cash-in minus cash-out on investment accounts, plus net savings in cash accounts. Recalculated monthly or when accounts change [confirmed H37].
- **Feasibility:**
  - Net contributions per investment account from transactions (contribution/transfer detection): **[lib extension]**.
  - Goal: **[needs user input]**, or derived from the planner.
  - Success-probability ranges: **[lib extension]**, dependent on F22.

### F22. Retirement Planner
- **Purpose:** Monte Carlo projection of whether savings last from now to life expectancy [confirmed H25].
- **Inputs:**
  - Profile: birth date, spouse, marital and filing status, state, income, children [confirmed H35; R1, R6].
  - Retirement age (slider) [review R6].
  - Life expectancy, inflation, effective tax rate (Edit Assumptions) [confirmed H35; R1].
  - **Starting Portfolio**: synced from investment and cash accounts, or set manually. Split into sub-portfolios Taxable / Tax Deferred / Spouse Tax Deferred / Tax Free / Education. Other Assets are excluded [confirmed H34, H38].
  - **Annual Savings**: "Estimate from savings history" or manual, with an optional yearly increase above inflation [confirmed H32, H37].
  - **Retirement Spending**: annual amount, with an optional yearly decrease (default 1%) from age 65 down to a Spending Minimum [confirmed H33, H36].
  - **Income Events**: Social Security (can sync with an SSA-based estimate), Annuity, Inheritance, Pension, Rental, Sale of Property/Downsize, Work During Retirement, Other. Each can be cost-of-living adjusted and before or after tax; Social Security is always before tax. They can be one-time or for a set period [confirmed H29, H31].
  - **Spending Goals**: Charity/Gift, Dependent Support, Education, Health Care, Home Purchase/Upgrade, Renovation, Vacation, Vehicle, Wedding, Other. Education is funded first from Annual Savings, and shortfalls or surpluses flow to the taxable bucket [confirmed H30].
  - Education planner pulls tuition estimates for real colleges and builds a 529 track [review R6, R9].
- **Computation** [confirmed H26, H27, H28, H32, H33, H35]:
  - **5,000 Monte Carlo runs**. Expected return and volatility come from the user's aggregate asset-class weights applied to historical index proxies.
  - Inflation assumption 3.5%, plus a **1% annual performance drag**.
  - Contributions are allocated to buckets in a fixed order with annual limits. Withdrawals come in tax-aware order (tax-deferred early if currently in the lowest bracket).
  - 85% of Social Security is taxed. RMDs follow IRS tables.
  - Federal and state tax tables are applied in distribution.
  - Detailed Cash Flow per year: portfolio value (average and poor markets), basic saving or retirement spending, spending goals, other income, net cash flow, taxes on withdrawals.
  - **Retirement Spending Ability**: sustainable monthly spending in the median scenario vs the planned amount (example in review: "Projected $5,166/mo vs. Planned $3,750/mo") [review R6].
- **Outputs and UI:**
  - **Projected Portfolio Value** chart with median and 10th-percentile lines to age ~95 [review R6].
  - Success-rate score: 50–80% "on track", >90% comfortable [review R1].
  - **Scenarios**: create, compare side by side, and promote one to the plan [confirmed S1; R1, R6]. Alternate scenarios were web-only on iPad [confirmed H87].
  - **Recession simulator / market-conditions toggle** (Dot-com, 2008, inflation spike) [review R6; S3 "Simulate recessions"].
  - Asset-location view by tax bucket [review R6].
  - Fee and tax-loss-harvesting suggestions that lead into the advisory upsell [review R6].
  - 2026: color-coded on-track indicators and a "Lifetime Income Score" (workplace) [confirmed S5–S9].
- **Feasibility:**
  - Starting balances from investment and cash classes: **[existing lib]**.
  - Tax bucket per account: **[needs user input]**.
  - Savings history: **[lib extension]** (F21).
  - Return and volatility assumptions: SimpleFIN lacks holdings, so use user-chosen allocation presets or manual holdings: **[needs user input]**. Historical index series and recession replays: **[needs data SimpleFIN lacks]**, but can be bundled as static data.
  - Monte Carlo engine, tax approximation, cash-flow table: **[lib extension]**, a substantial build.

### F23. Portfolio: holdings, allocation, sectors, performance, You Index
- **Holdings** [confirmed H52, H53; R1]:
  - Columns: name, ticker, shares, price, 1-day % and $, value; intraday prices, with funds updating after close.
  - Unclassified holdings can be classified manually by a representative ticker mix or by percentages.
- **Allocation** [confirmed H53; R1, R9]:
  - Asset classes US/intl stocks, US/intl bonds, alternatives and cash, with drill-down. Shown as proportional boxes (treemap-like) [review R9].
  - Classification data comes from Morningstar; prices from Xignite [confirmed H49].
- **Performance** [confirmed H51; R3, S14]:
  - Account and portfolio returns computed from balance change with external cash flows neutralized (deposits removed, withdrawals added back).
  - Compared to benchmarks (S&P 500 and others) over chosen ranges. Account performance was web-only [confirmed H87].
- **You Index**: back-extrapolated performance of current priced holdings [confirmed H50].
- **Allocation targets and drift** with dollar amounts to rebalance [review R8].
- **Feasibility:**
  - Holdings, allocation, sectors, You Index, Market Movers: **[needs data SimpleFIN lacks]** (holdings plus prices). A path exists via Manual Investment Holdings: **[needs user input]**, still with no price feed.
  - **Account-level performance** using Empower's own method (balance history + contribution/withdrawal flows): **[lib extension]**, approximate and only as good as bridge transaction coverage.
  - Benchmarks: **[needs data SimpleFIN lacks]**; a bundled or remote index series would be needed.

### F24. Investment Checkup
- **Inputs:** investment profile (marital status, age, target retirement age), risk profile (five levels, "Highest Safety" … "Highest Growth"), retirement plan figures [review R7].
- **Panels** [confirmed H43–H48; R7]:
  - Alternative (target) Allocation across six classes.
  - Historical Performance since 1992, inflation-adjusted, annual rebalance.
  - Monte Carlo with 5,000 runs, median and 10% worst.
  - Efficient Frontier scatter of growth vs volatility.
  - Allocation Comparison bars plus dollars to buy or sell.
  - Fee ratings and concentration flags.
- **Limits:** unclassified assets are excluded; mostly pre-tax and pre-fee.
- **Feasibility:** **[needs data SimpleFIN lacks]** (holdings and classification). It could be offered in reduced form from user-entered allocation: **[needs user input]** + **[lib extension]**.

### F25. Retirement Fee Analyzer
- **Computation** [confirmed H39–H42; R1]:
  - Weighted expense ratio ("Fund Fees/Yr" = expected expense-ratio dollars ÷ account value).
  - Sliders for plan admin and advice fees and for unpublished fund fees.
  - Chart of retirement value before and after fees, assuming 8% compounding and a constant fee rate.
  - Non-taxable accounts are selected by default, with an account picker.
- **Feasibility:** **[needs data SimpleFIN lacks]** (fund holdings and expense ratios). A simplified "what does X% cost me" calculator on investment balances is **[needs user input]** + **[lib extension]**.

### F26. Notifications and emails
- **Confirmed channels:**
  - **Daily Email** with every transaction from the last 24 hours (fraud monitoring).
  - **Weekly** summary or performance email (net worth, markets, upcoming bills, accounts needing attention).
  - Toggled in Settings → Email Subscriptions [confirmed H88, H89; R1, R8].
  - Foreign-login alert email, toggleable [confirmed H90].
  - Push notifications for Mobile App Verification approvals [confirmed H93, H94].
  - Bill reminders [confirmed H17].
  - Portfolio recommendation notifications [confirmed H101].
- **Not documented:** per-transaction push, low-balance alerts, large-purchase alerts, budget-overrun push.
- **Feasibility:**
  - Daily or weekly digest from new transactions and net-worth deltas: **[lib extension]**.
  - Balance-change anomaly alerts: **[existing lib]** (differentiator).
  - Budget-pace alerts and recurring price-change alerts: **[lib extension]**.

### F27. Security
- **Mobile session:**
  - 6-digit PIN after password; auto-logout after 10 minutes with PIN on return [confirmed H96].
  - Touch ID / Face ID (iOS) and "Fingerprint Authentication" (Android) [confirmed H92, H97].
- **Account and device protection:**
  - Device authorization through out-of-band verification (SMS, call, email) [confirmed H98].
  - Optional second-layer MFA by SMS or phone code on every login. Enabling it **disables biometrics** in the legacy app [confirmed H91, H92].
  - **Mobile Authentication (TOTP)** code generator in-app, plus push approval [confirmed H93, H94].
  - Authorized Devices list with rename and remove [confirmed H95].
- **Back end:** AES-256 encryption; view-only aggregation [confirmed H98; S7].
- **Feasibility:** **[lib extension]** at the app layer (local PIN, biometrics, encrypted store for the SimpleFIN access URL). Device management is mostly N/A for a local-first app.

### F28. Profile, settings and household
- **Profile:** names, birth dates, spouse, income, savings, investment objectives; feeds planners [review R1].
- **Settings toggles:** investment-income exclusion (F15), email subscriptions, security settings [confirmed H82, H89, H90].
- **Additional Users:** up to five; advisory clients only [confirmed H99].
- **Feasibility:** **[needs user input]**. Household sharing is out of scope for a local-first MVP.

### F29. Export
- **Behavior:**
  - Transactions to CSV, with columns date, account, description, category, tags, amount [confirmed H85; review-site description].
  - iOS also exports Net Worth to PDF and transactions to CSV (with a sort option) [confirmed H84].
- **Feasibility:** **[lib extension]**.

### F30. Guidance and advice surfaces
- **Surfaces:**
  - 2026 personalized-guidance quiz [confirmed S5].
  - Financial Roadmap topic ranker (advisory clients) [confirmed H100].
  - Virtual Assistant [confirmed S7].
  - Advisor-call prompts throughout [review R6, R7].
- **Feasibility:**
  - Rule-based insights (for example "spending up 18% vs 3-month average in Restaurants", "emergency fund covers 2.1 months") from existing lib outputs: **[lib extension]**.
  - The advisory upsell does not apply.

---

## 4. Feasibility summary table

| # | Feature | Empower evidence | Feasibility | Tier |
|---|---|---|---|---|
| F1 | Home widget dashboard | confirmed | existing lib + lib extension (market widgets: needs data SimpleFIN lacks) | MVP |
| F2 | Linking, refresh, health icons, resync | confirmed | needs user input (bridge token) + lib extension | MVP |
| F3 | Grouped account list | confirmed | existing lib; Mortgage/Other Liabilities split: lib extension; APR/limit: needs data SimpleFIN lacks | MVP |
| F4 | Account details (balance chart, txns, available vs posted) | confirmed | existing lib + lib extension; holdings tab: needs data SimpleFIN lacks | MVP |
| F5 | Edit account (name, type, group/exclude, close/reopen) | confirmed | needs user input + lib extension | Next |
| F6 | Manual accounts + staleness | confirmed | existing lib; auto-valuation: needs data SimpleFIN lacks | MVP |
| F7 | Net worth + history + deltas | confirmed | existing lib + lib extension | MVP |
| F8 | Transactions list/search/filter/date ranges | confirmed | lib extension | MVP |
| F9 | Edit txn (desc, category, date, hide duplicate, manual add, attachment) | confirmed | needs user input + lib extension | MVP (attachments Later) |
| F10 | Categories (types, custom ≤30) | confirmed | existing lib (keyword categorization) + lib extension + needs user input | MVP |
| F11 | Tags (+Reimbursable budget exclusion) | confirmed | needs user input + lib extension | Next |
| F12 | Rules (apply to future/past) | confirmed | lib extension | MVP |
| F13 | Split transactions (≤5) | confirmed | needs user input + lib extension | Next |
| F14 | Bulk edit | confirmed | lib extension | Next |
| F15 | Cash flow (income/expense, exclusions, MoM) | confirmed | existing lib (trends) + lib extension | MVP |
| F16 | Budget (monthly goal ring, top-5, pacing; category budgets) | confirmed / conflicting | needs user input + lib extension | MVP |
| F17 | Bill reminders | confirmed | needs data SimpleFIN lacks → substitute via recurring prediction (lib extension) + needs user input | Next |
| F18 | Recurring/subscriptions | absent in Empower (R1) | existing lib | MVP (differentiator) |
| F19 | Emergency fund | confirmed | existing lib + lib extension + needs user input | Next |
| F20 | Debt paydown | confirmed | existing lib + lib extension; APR: needs user input | Next |
| F21 | Savings planner: retirement savings | confirmed | lib extension + needs user input | Later |
| F22 | Retirement planner (Monte Carlo, events, scenarios) | confirmed | needs user input + lib extension; historical/recession series: needs data SimpleFIN lacks (bundle) | Later |
| F23 | Holdings/allocation/You Index/benchmarks | confirmed | needs data SimpleFIN lacks; account-level performance: lib extension | Later |
| F24 | Investment checkup | confirmed | needs data SimpleFIN lacks (reduced version: needs user input) | Later |
| F25 | Fee analyzer | confirmed | needs data SimpleFIN lacks (simplified calc: needs user input) | Later |
| F26 | Notifications/digests/anomaly alerts | confirmed (email-centric) | existing lib (anomalies) + lib extension | Next |
| F27 | PIN/biometrics/auto-lock | confirmed | lib extension (app layer) | MVP |
| F28 | Profile/settings/household | confirmed | needs user input | Next (household: Later) |
| F29 | CSV/PDF export | confirmed | lib extension | Next |
| F30 | Guidance/insights | confirmed (2026 quiz) | lib extension | Later |

---

## 5. Published lists

### 5.1 Spending categories
Empower does **not** publish its full default category list. No help, marketing or review page I could reach lists it, and the help center says default categories exist and can be edited but not deleted [H11]. These names are **confirmed** to exist from Empower text:
- Service Charges/Fees [H21]
- Interest [H54]
- Investment Income [H54]
- General Merchandise [R2]
- Groceries [R2]
- Transfers [H4; "transfers" category class]
- Credit Card Payment(s) [H4, H81]
- 401k / Retirement Contribution(s) [H4]
- Savings [H4]
- Securities Trades [H4]

Category **types**: Income, Expense, Transfer [H10]. The delete UI labels the third group "Other" [H11].

Tags confirmed: Reimbursable (budget-excluded), Manual Transaction; example tags medical and tax related [H4, H16, H83].

Recommendation: define our own default taxonomy. Keep these names where possible for migration familiarity, and assign each category a type (Income/Expense/Transfer) that drives cash-flow exclusion. The rest of the Personal Capital default list is believed to derive from the aggregator's taxonomy (Yodlee) [inferred; not verified].

### 5.2 Account types
- **Account groups** (panel and filter order): Cash, Investment, Credit, Loan, Mortgage, Other Asset, Other Liabilities [H2, H3].
- **Cash account types:** Bank, Certificate of Deposit (CD), Trust, Custodial, Business/Corporate, Health Savings Plan [H56]. Linkable banking types: checking, savings, CDs, money market [R1].
- **Investment types** (not an exhaustive list): 401(k), 403(b), TSP, IRA, taxable/brokerage, HSA (cash HSAs can be "grouped with investments"), 529/education, ESPP/RSU/options, Charitable Gift Fund, Pension [H23, H57; R1].
- **Planner tax buckets:** Taxable, Tax Deferred, Spouse Tax Deferred, Tax Free, Education [H34].
- **Liabilities:** credit cards; mortgage; student, auto and personal loans; leases; HELOC [R1, R8].
- **Manual types:** Home (Zestimate optional), vehicle, jewelry and precious metals, personal property, Cryptocurrency, Manual Investment Holdings ("Add Portfolio"), Custom Stock Options, generic other asset or liability [H64–H71].

**Mapping to our lib's five classes:**

| Empower group | Our lib class |
|---|---|
| Cash | Cash |
| Investment | Investments |
| Credit | Credit Cards |
| Loan, Mortgage | Loans, with a sub-type flag: lib extension |
| Other Asset | Other Assets |
| Other Liabilities | no class yet: lib extension |

---

## 6. Visual design notes (described in words; no images copied)

What sources state:
- **Net worth / portfolio balance:**
  - Line chart over 90 days by default, big current total above, change chips (90-day and 1-day) below, scrub or hover tooltips with date, value and daily change [H1].
  - Closed accounts get a 90-day pre-close chart [H62].
  - One review mentions an asset pie chart [R8].
- **Budget:** two **concentric rings**. The outer is month-to-date or 30-day spend vs budget and turns **red** when over; the inner is last month at the same point. The editable budget figure sits in the ring's center or beside it, and a top-5-categories legend plus "Other" sits next to it [H5; R9].
- **Cash flow:** monthly **bar chart** of income vs expense; bars are colored by category, and hovering or tapping a segment reveals the category and amount [H1, H4]. Income/Expense tabs pair a category breakdown (left) with a month-over-month chart (right) [H4].
- **Emergency fund:** 12 monthly **bars in blue** against a **green horizontal band** for the 3–6-month target [H1, H19].
- **Retirement savings:** a **progress arc or meter** with a status message [H1, H23].
- **Retirement planner:** fan or line chart of projected value with **median and 10th-percentile** lines across ages, timeline markers for spending goals and income events, and a meter comparing projected vs planned spending [H25, H26; R6; S12 alt text].
- **Allocation:** proportional **boxes** (treemap-style) with drill-in [R9].
- **Checkup:** **efficient-frontier scatter**, historical growth curves, allocation-comparison bars [R7].
- **2026 dashboard:**
  - "Cleaner layout", **color-coded on-track indicators** for retirement, refreshed budgeting visuals, graphs and tables that open by default [S5–S9].
  - Bottom tab bar on mobile [S9].
  - Web and mobile fonts, spacing and widget layouts may differ slightly [S8].
- **Account status:** **red** icon means user action is needed; **yellow** means a provider-side error [H73].
- **Brand:** reviewers note the colors changed with the 2023 Personal Capital → Empower rebrand [R3]. The 2026 redesign is described only as "more modern" [S5].

What sources do **not** state: exact palette values, typeface, iconography, dark-mode support. Treat any palette as our own choice. A reasonable direction consistent with the above [inferred]:
- A deep navy/ink primary for headers and totals; a white or very light neutral canvas.
- A green for income, positive change and the "on track" state; red for over-budget and losses; amber for warnings and stale data; a single blue for cash and neutral series.
- A fixed categorical palette of 8–10 hues for categories, with "Other" in grey.
- Large tabular-figure numerals for money, so totals line up.
- Each widget a card with title, one hero number, one chart, and a "see more" affordance.

---

## 7. Priority tiers for a SimpleFIN-backed equivalent

**MVP: core loop, data we fully have**
- Setup: paste SimpleFIN token, sync, connection-health and staleness states (F2).
- Accounts grouped by our classes, with account details and available vs posted balance (F3, F4).
- Manual accounts with staleness reminders (F6).
- Net worth with history, date ranges, 1-day and period deltas (F7).
- Transactions: search, account-type tabs, date-range presets (default 90 days) and custom ranges, category filter (F8).
- Transaction edit: rename, recategorize, hide duplicate, manual add (F9).
- Categories with Income/Expense/Transfer types and custom categories (F10); rules with apply-to-future/past (F12).
- Cash flow, income vs expense by month, with transfer and card-payment exclusions (F15).
- Budget: overall monthly goal, ring with pacing, top-5 categories, 3- and 12-month averages. Consider per-category budgets from day one, since reviews call their absence Empower's biggest gap (F16).
- Recurring expenses list (F18, differentiator) and month-over-month spending trends.
- Home dashboard widgets for the above (F1); app lock with PIN and biometrics (F27).

**Next: depth and planning-lite**
- Tags with a Reimbursable exclusion (F11), splits (F13), bulk edit (F14).
- Account edit: retype, group/exclude, close and reopen with close date (F5).
- Emergency fund (F19); debt paydown with user-entered APR (F20).
- Upcoming bills predicted from recurring detection, with user due-date overrides (F17).
- Digest notifications plus balance-anomaly, budget-pace and recurring price-change alerts (F26).
- CSV and PDF export (F29); profile and settings including the investment-income toggle (F28).

**Later: needs heavy modeling or data SimpleFIN lacks**
- Retirement planner with Monte Carlo, income events, spending goals, scenarios and a bundled historical-returns or recession dataset (F22); savings planner retirement view (F21).
- Account-level investment performance from balance and cash-flow math (F23, partial).
- Manual holdings with optional price feed, allocation, a reduced checkup and a fee calculator (F23–F25).
- Insight cards and guidance (F30); household sharing (F28); transaction attachments (F9).

---

## 8. Gaps and uncertainties
1. **2026 mobile navigation**: bottom-tab labels and screen inventory are unpublished. Section 2.3 is inferred.
2. **App merger timing**: "late 2025" comes from one third-party review [R4]. Store evidence shows the standalone listings gone by Oct 2026 and migration-related notes in the Empower ® app, but I found no official announcement. The Android package id of the successor app was not found.
3. **Category budgets**: legacy sources say there is only one overall budget; two 2026 reviews imply category budgets or targets exist in "New Budgeting". Unresolved.
4. **Default category list**: not published (see 5.1).
5. **Investment Checkup on mobile**: an older help article says web-only; a 2025 review says it is on the mobile menu.
6. **Help-center freshness**: article captures are from 2024–2025. Some behavior (for example, splits being web-only, mobile export being iOS-only) may have changed in the 2026 app.
7. **Visual specifics** (palette, type, dark mode): not described publicly in text; section 6 marks the inferred parts.
8. **Notifications**: only email digests, security pushes and bill reminders are documented. Granular push alerts are undocumented, so they are absent or unknown.
9. Some reviews (R6, R7, R8) are sponsored or affiliate content; R5 is written by a competitor.
